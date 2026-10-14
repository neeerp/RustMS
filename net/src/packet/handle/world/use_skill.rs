use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::buff;
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::skills::{self, Learned};
use game::{Buff, BuffStat, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::{Duration, Instant};
use tracing::debug;

/// `USE_SKILL`: a skill that doesn't attack. Only the beginner buffs
/// (Recovery and Nimble Feet, for every beginner job) do anything yet.
pub struct UseSkillHandler;

impl WorldHandler for UseSkillHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?; // tick
        let skill_id = r.read_int()?;
        let level = i32::from(r.read_byte()?);
        match use_skill(ctx.player, skill_id, level, Instant::now()) {
            Ok(used) => announce(ctx.player, &used),
            Err(refusal) => {
                debug!(skill_id, level, ?refusal, "Skill use refused");
                Ok(HandlerResult::empty())
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Dead,
    /// Not learned, or not at the level the client named.
    NotLearned,
    Unsupported,
    CoolingDown,
    NotEnoughMp,
}

/// The buff a skill gives, and its value at this level.
fn buff_of(skill: &Learned) -> Option<(BuffStat, i32)> {
    match skill.skill_id % 10_000_000 {
        1001 => Some((BuffStat::Recovery, skill.effect.x)),
        1002 => Some((BuffStat::Speed, skill.effect.speed)),
        _ => None,
    }
}

/// A buff skill that was used.
#[derive(Debug)]
struct Used {
    skill: Learned,
    stat: BuffStat,
    value: i16,
}

/// Check the skill can be used now, then spend its MP and start its buff
/// and cooldown.
fn use_skill(
    player: &mut Player,
    skill_id: i32,
    level: i32,
    now: Instant,
) -> Result<Used, Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    let skill = skills::learned(player, skill_id)
        .filter(|skill| skill.level == level)
        .ok_or(Refusal::NotLearned)?;
    let (stat, value) = buff_of(&skill).ok_or(Refusal::Unsupported)?;
    if player.buffs.cooling_down(skill_id, now) {
        return Err(Refusal::CoolingDown);
    }
    let effect = &skill.effect;
    if i32::from(player.stats.mp) < effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }

    player.add_mp(-effect.mp_cost);
    let value = value as i16;
    let duration = Duration::from_secs(effect.duration.max(0) as u64);
    player
        .buffs
        .give(Buff::new(skill_id, stat, value, now, duration));
    if effect.cooldown > 0 {
        let length = Duration::from_secs(effect.cooldown as u64);
        player.buffs.start_cooldown(skill_id, now, length);
    }
    Ok(Used { skill, stat, value })
}

/// The new MP, the buff, and the cooldown for the player; the skill's
/// effect for everyone else on the map.
fn announce(player: &Player, used: &Used) -> WorldResult {
    let Used { skill, stat, value } = used;
    let effect = &skill.effect;
    let mut result = HandlerResult::reply(stat::build_stat_update(
        false,
        &[StatUpdate::Mp(player.stats.mp)],
    )?)
    .with_reply(buff::build_give_buff(
        skill.skill_id,
        effect.duration * 1000,
        *stat,
        *value,
    )?);
    if effect.cooldown > 0 {
        result = result.with_reply(buff::build_cooldown(
            skill.skill_id,
            effect.cooldown as i16,
        )?);
    }
    let shown = buff::build_skill_effect(
        player.id(),
        skill.skill_id,
        player.stats.level,
        skill.level as u8,
    )?;
    Ok(result.with(WorldAction::Field(FieldRequest::Broadcast(shown))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::SendOpcode;
    use game::SkillRecord;

    fn beginner_with(skill_id: i32, level: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 50;
        player.stats.mp = 20;
        player.stats.max_mp = 20;
        player.skills.set(SkillRecord {
            skill_id,
            level,
            master_level: 0,
            expiration: None,
        });
        player
    }

    fn opcodes(result: WorldResult) -> Vec<i16> {
        result
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => p.opcode(),
                WorldAction::Field(FieldRequest::Broadcast(p)) => p.opcode(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn nimble_feet_spends_mp_and_gives_speed_until_it_cools_down() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(1002, 2);
        assert_eq!(
            use_skill(&mut player, 1002, 1, now).unwrap_err(),
            Refusal::NotLearned
        );
        let used = use_skill(&mut player, 1002, 2, now).unwrap();
        let sent = opcodes(announce(&player, &used));
        assert_eq!(
            sent,
            [
                SendOpcode::StatChange as i16,
                SendOpcode::GiveBuff as i16,
                SendOpcode::Cooldown as i16,
                SendOpcode::ShowForeignEffect as i16,
            ]
        );
        assert_eq!(player.stats.mp, 13);
        let buff = &player.buffs.active()[0];
        assert_eq!((buff.stat, buff.value), (BuffStat::Speed, 15));
        assert_eq!(buff.expires, now + Duration::from_secs(8));
        assert_eq!(
            use_skill(&mut player, 1002, 2, now + Duration::from_secs(59)).unwrap_err(),
            Refusal::CoolingDown
        );
        player.stats.mp = 6;
        assert_eq!(
            use_skill(&mut player, 1002, 2, now + Duration::from_secs(60)).unwrap_err(),
            Refusal::NotEnoughMp
        );
    }

    #[test]
    fn recovery_heals_by_its_x_and_attacks_are_not_buffs() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(1001, 3);
        assert!(use_skill(&mut player, 1001, 3, Instant::now()).is_ok());
        let buff = &player.buffs.active()[0];
        assert_eq!((buff.stat, buff.value), (BuffStat::Recovery, 12));

        let mut player = beginner_with(1000, 1);
        assert_eq!(
            use_skill(&mut player, 1000, 1, Instant::now()).unwrap_err(),
            Refusal::Unsupported
        );
    }
}
