use crate::error::NetworkError;
use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::packet::build::world::{buff, skill};
use crate::skills::{self, Learned};
use game::{Buff, BuffStat, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::{Duration, Instant};
use tracing::debug;

/// `USE_SKILL`: a skill that doesn't attack. Only the beginner buffs
/// (Recovery and Nimble Feet, for every beginner job) and Monster Magnet
/// do anything yet. Every answer unlocks the client, which waits on it
/// before using another skill or item.
pub struct UseSkillHandler;

impl WorldHandler for UseSkillHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?; // tick
        let skill_id = r.read_int()?;
        let level = i32::from(r.read_byte()?);
        if skills::is_magnet(skill_id) {
            let caught = read_magnet(&mut r)?;
            return match use_magnet(ctx.player, skill_id, level, caught.as_ref()) {
                Ok(skill) => {
                    let (caught, toleft) = caught.unwrap_or_default();
                    announce_magnet(ctx.player, &skill, caught, toleft)
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Monster Magnet refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        if skill_id == skills::HEAL {
            let party = r.read_byte()?;
            return match heal(ctx.player, level, party, rand::random()) {
                Ok(hp) => {
                    let mp = ctx.player.stats.mp;
                    Ok(HandlerResult::reply(stat::build_stat_update(
                        true,
                        &[StatUpdate::Hp(hp), StatUpdate::Mp(mp)],
                    )?))
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Heal refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        match use_skill(ctx.player, skill_id, level, Instant::now()) {
            Ok(used) => announce(ctx.player, &used),
            Err(refusal) => {
                debug!(skill_id, level, ?refusal, "Skill use refused");
                Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
            }
        }
    }
}

/// `SKILL_EFFECT`: the player started preparing a skill, held down or
/// timed. Show it to the rest of the map.
pub struct SkillEffectHandler;

impl WorldHandler for SkillEffectHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let skill_id = r.read_int()?;
        let level = r.read_byte()?;
        let action_and_facing = r.read_short()? as u16;
        let speed = r.read_byte()?;
        let learned = skills::learned(ctx.player, skill_id)
            .is_some_and(|skill| skill.level == i32::from(level));
        if ctx.player.stats.hp <= 0 || !skills::prepares(skill_id) || !learned {
            return Ok(HandlerResult::empty());
        }
        let shown =
            skill::build_skill_prepare(ctx.player.id(), skill_id, level, action_and_facing, speed)?;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

/// `THROW_GRENADE`: the player let go of Grenade or Poison Bomb, throwing
/// it from where they stood (0x95efc5). Show the rest of the map the throw,
/// and end the prepare it showed: the client sends no `CANCEL_BUFF` for a
/// throw, and the attack comes only once the grenade goes off. The attack
/// is checked and spends MP then, as any other.
pub struct ThrowGrenadeHandler;

impl WorldHandler for ThrowGrenadeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let x = r.read_int()?;
        let y = r.read_int()?;
        let charge = r.read_int()?;
        let skill_id = r.read_int()?;
        if ctx.player.stats.hp <= 0
            || !skills::throws(skill_id)
            || skills::learned(ctx.player, skill_id).is_none()
        {
            return Ok(HandlerResult::empty());
        }
        let id = ctx.player.id();
        let ended = skill::build_cancel_skill_prepare(id, skill_id)?;
        let thrown = skill::build_throw_grenade(id, x, y, charge, skill_id)?;
        Ok(HandlerResult::empty()
            .with(WorldAction::Field(FieldRequest::Broadcast(ended)))
            .with(WorldAction::Field(FieldRequest::Broadcast(thrown))))
    }
}

/// `CANCEL_BUFF`: the player right-clicked a skill's buff icon, or let go
/// of a held skill without using it. End the buffs and tell the client,
/// which then drops the icon; or show the rest of the map the skill was
/// let go.
pub struct CancelBuffHandler;

impl WorldHandler for CancelBuffHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let skill_id = r.read_int()?;
        if skills::keydown(skill_id) {
            let shown = skill::build_cancel_skill_prepare(ctx.player.id(), skill_id)?;
            return Ok(
                HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown)))
            );
        }
        let ended = ctx.player.buffs.cancel(skill_id);
        if ended.is_empty() {
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::reply(buff::build_cancel_buff(&ended)?))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Dead,
    /// Not learned, or not at the level the client named.
    NotLearned,
    /// Monster Magnet reaching more mobs than it can.
    TooManyMobs,
    Unsupported,
    CoolingDown,
    NotEnoughMp,
}

/// Monster Magnet reaches at most this many mobs (0x96c215).
const MAGNET_MOBS: i32 = 15;

/// What Monster Magnet sends after its level (0x96c62a): each mob it
/// reached, 0 for one out of sight, and whether it was caught; then which
/// way the player faced. None if it claims more mobs than it can reach.
fn read_magnet(r: &mut Cursor<&Vec<u8>>) -> Result<Option<(Vec<(i32, bool)>, bool)>, NetworkError> {
    let found = r.read_int()?;
    if !(0..=MAGNET_MOBS).contains(&found) {
        return Ok(None);
    }
    let mut caught = Vec::with_capacity(found as usize);
    for _ in 0..found {
        let object_id = r.read_int()?;
        let success = r.read_byte()? != 0;
        if object_id != 0 {
            caught.push((object_id, success));
        }
    }
    let toleft = r.read_byte()? != 0;
    Ok(Some((caught, toleft)))
}

/// Check Monster Magnet can be used, then spend its MP.
fn use_magnet(
    player: &mut Player,
    skill_id: i32,
    level: i32,
    caught: Option<&(Vec<(i32, bool)>, bool)>,
) -> Result<Learned, Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    let skill = skills::learned(player, skill_id)
        .filter(|skill| skill.level == level && level > 0)
        .ok_or(Refusal::NotLearned)?;
    if caught.is_none() {
        return Err(Refusal::TooManyMobs);
    }
    if i32::from(player.stats.mp) < skill.effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }
    player.add_mp(-skill.effect.mp_cost);
    Ok(skill)
}

/// The new MP for the player; for everyone else on the map, the skill's
/// effect and each mob it caught or failed to.
fn announce_magnet(
    player: &Player,
    skill: &Learned,
    caught: Vec<(i32, bool)>,
    toleft: bool,
) -> WorldResult {
    let effect = buff::build_magnet_effect(
        player.id(),
        skill.skill_id,
        player.stats.level,
        skill.level as u8,
        toleft,
    )?;
    Ok(HandlerResult::reply(stat::build_stat_update(
        true,
        &[StatUpdate::Mp(player.stats.mp)],
    )?)
    .with(WorldAction::Field(FieldRequest::Magnet { effect, caught })))
}

/// Heal comes before its attack, with a bit for each party member it
/// reaches (0x96a69d), or 0x80 without a party. It heals the cleric by each
/// one's share; the party members besides them aren't healed yet. It spends
/// the MP, so the attack doesn't.
fn heal(player: &mut Player, level: i32, party: u8, roll: f64) -> Result<i16, Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    let skill = skills::learned(player, skills::HEAL)
        .filter(|skill| skill.level == level)
        .ok_or(Refusal::NotLearned)?;
    if i32::from(player.stats.mp) < skill.effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }
    player.add_mp(-skill.effect.mp_cost);
    let reached = party.count_ones() as i32;
    let amount = skills::heal_amount(player, skill.effect.hp, reached, roll);
    Ok(player.add_hp(amount as i32))
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
        true,
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
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::SkillRecord;
    use packet::io::write::PktWrite;

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

    fn handle(player: &mut Player, skill_id: i32, level: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(skill_id).unwrap();
        packet.write_byte(level).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    /// Whether the action is a stat update that unlocks the client.
    fn unlocks(action: &WorldAction) -> bool {
        matches!(action, WorldAction::Reply(p)
            if p.opcode() == SendOpcode::StatChange as i16 && p.bytes[2] == 1)
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
    fn cancelling_a_buff_ends_it_and_tells_the_client() {
        use crate::packet::op::RecvOpcode;
        use packet::io::write::PktWrite;

        let mut player = beginner_with(1002, 2);
        let now = Instant::now();
        player.buffs.give(Buff::new(
            1002,
            BuffStat::Speed,
            15,
            now,
            Duration::from_secs(8),
        ));
        let mut cancel = |skill_id: i32| {
            let mut packet = Packet::new_empty();
            packet.write_short(RecvOpcode::CancelBuff as i16).unwrap();
            packet.write_int(skill_id).unwrap();
            let mut ctx = WorldContext {
                client_id: 1,
                channel_id: 0,
                player: &mut player,
            };
            opcodes(CancelBuffHandler.handle(&mut packet, &mut ctx))
        };
        assert_eq!(cancel(1001), Vec::<i16>::new());
        assert_eq!(cancel(1002), [SendOpcode::CancelBuff as i16]);
        assert_eq!(cancel(1002), Vec::<i16>::new());
        assert!(player.buffs.active().is_empty());
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

    #[test]
    fn heal_heals_the_cleric_by_their_share() {
        if skills::data().is_err() {
            return;
        }
        let cleric = || {
            let mut player = beginner_with(skills::HEAL, 30);
            player.stats.int = 400;
            player.stats.luk = 100;
            player.stats.max_hp = 10_000;
            player.stats.mp = 100;
            player.stats.max_mp = 100;
            player
        };
        // Alone, the lowest roll: (0.3 + 1) * 300% * (80 * 1.5 + 100) * 400
        // * 0.005.
        let mut player = cleric();
        assert_eq!(heal(&mut player, 30, 0x80, 0.0), Ok(50 + 1716));
        // Reaching two, each gets (0.6 + 1) / 2 of it.
        let mut player = cleric();
        assert_eq!(heal(&mut player, 30, 0x30, 0.0), Ok(50 + 1056));
        // Heal spends the MP, not its attack.
        let cost = skills::learned(&player, skills::HEAL)
            .unwrap()
            .effect
            .mp_cost;
        assert!(cost > 0);
        assert_eq!(i32::from(player.stats.mp), 100 - cost);

        assert_eq!(heal(&mut player, 29, 0x80, 0.0), Err(Refusal::NotLearned));
        player.stats.mp = 0;
        assert_eq!(heal(&mut player, 30, 0x80, 0.0), Err(Refusal::NotEnoughMp));
    }

    #[test]
    fn using_or_refusing_a_skill_unlocks_the_client() {
        let mut player = beginner_with(1000, 1);
        let actions = handle(&mut player, 1000, 1);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));

        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(1002, 2);
        assert!(unlocks(&handle(&mut player, 1002, 2)[0]));
    }

    fn magnet(
        player: &mut Player,
        level: u8,
        found: i32,
        mobs: &[(i32, bool)],
    ) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(1121001).unwrap();
        packet.write_byte(level).unwrap();
        packet.write_int(found).unwrap();
        for &(object_id, success) in mobs {
            packet.write_int(object_id).unwrap();
            packet.write_byte(u8::from(success)).unwrap();
        }
        packet.write_byte(1).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn hero_with_magnet() -> Player {
        let mut player = beginner_with(1121001, 30);
        player.stats.job = 112;
        player.stats.mp = 100;
        player.stats.max_mp = 100;
        player
    }

    #[test]
    fn monster_magnet_spends_mp_and_shows_the_mobs_it_reached() {
        if skills::data().is_err() {
            return;
        }
        let mut player = hero_with_magnet();
        let actions = magnet(&mut player, 30, 3, &[(101, true), (0, false), (102, false)]);
        let [unlock, WorldAction::Field(FieldRequest::Magnet { effect, caught })] =
            actions.as_slice()
        else {
            panic!("unexpected {actions:?}");
        };
        assert!(unlocks(unlock));
        assert_eq!(player.stats.mp, 79);
        assert_eq!(caught, &[(101, true), (102, false)]);
        assert_eq!(effect.opcode(), SendOpcode::ShowForeignEffect as i16);
        assert_eq!(effect.bytes.last(), Some(&1), "it ends with the facing");
    }

    #[test]
    fn monster_magnet_is_refused_at_another_level_or_past_15_mobs() {
        if skills::data().is_err() {
            return;
        }
        let mut player = hero_with_magnet();
        let actions = magnet(&mut player, 29, 0, &[]);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));
        let mobs: Vec<(i32, bool)> = (1..=16).map(|oid| (oid, false)).collect();
        let actions = magnet(&mut player, 30, 16, &mobs);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));
        assert_eq!(player.stats.mp, 100);
    }

    fn prepare(player: &mut Player, skill_id: i32, level: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::SkillEffect as i16).unwrap();
        packet.write_int(skill_id).unwrap();
        packet.write_byte(level).unwrap();
        packet.write_short(0x8016_u16 as i16).unwrap();
        packet.write_byte(4).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        SkillEffectHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn cancel(player: &mut Player, skill_id: i32) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::CancelBuff as i16).unwrap();
        packet.write_int(skill_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        CancelBuffHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn throw(player: &mut Player, skill_id: i32) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::ThrowGrenade as i16).unwrap();
        for value in [120, -45, 640, skill_id] {
            packet.write_int(value).unwrap();
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ThrowGrenadeHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn broadcast_opcodes(actions: &[WorldAction]) -> Vec<i16> {
        actions
            .iter()
            .map(|action| match action {
                WorldAction::Field(FieldRequest::Broadcast(p)) => p.opcode(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_prepared_skill_is_shown_to_the_map() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(3121004, 20);
        player.stats.job = 312;
        assert_eq!(
            broadcast_opcodes(&prepare(&mut player, 3121004, 20)),
            [SendOpcode::SkillEffect as i16]
        );
        assert!(prepare(&mut player, 3121004, 19).is_empty());
    }

    #[test]
    fn only_skills_that_prepare_are_shown() {
        let mut player = beginner_with(1002, 2);
        assert!(prepare(&mut player, 1002, 2).is_empty());
    }

    #[test]
    fn letting_go_of_a_held_skill_is_shown_to_the_map() {
        let mut player = beginner_with(1002, 2);
        assert_eq!(
            broadcast_opcodes(&cancel(&mut player, 5221004)),
            [SendOpcode::CancelSkillEffect as i16]
        );
        assert!(cancel(&mut player, 1002).is_empty());
    }

    #[test]
    fn a_throw_ends_the_prepare_and_is_shown_to_the_map() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(5201002, 10);
        player.stats.job = 520;
        assert_eq!(
            broadcast_opcodes(&throw(&mut player, 5201002)),
            [
                SendOpcode::CancelSkillEffect as i16,
                SendOpcode::ThrowGrenade as i16
            ]
        );
        player.stats.hp = 0;
        assert!(throw(&mut player, 5201002).is_empty());
    }

    #[test]
    fn only_a_learned_throw_is_shown() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(5201002, 10);
        assert!(throw(&mut player, 14111006).is_empty());
        let mut player = beginner_with(5221004, 10);
        assert!(throw(&mut player, 5221004).is_empty());
    }
}
