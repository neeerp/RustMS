use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::skill;
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::skills;
use game::{ApStat, CharacterStats, Player, SkillRecord};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `SPEND_AP`: tick, then the STAT_CHANGED mask bit of the stat to raise.
/// The client locks its UI until the reply, so a refusal still answers.
pub struct SpendApHandler;

impl WorldHandler for SpendApHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let mask = reader.read_int()?;
        let bonus = skills::growth_bonus(ctx.player, false);
        let raised =
            ap_stat(mask).filter(|&stat| ctx.player.spend_ap(stat, bonus, skills::growth_roll));
        let Some(stat) = raised else {
            debug!(mask, ap = ctx.player.stats.ap, "AP spend refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let stats = &ctx.player.stats;
        Ok(HandlerResult::reply(stat::build_stat_update(
            true,
            &[stat_value(stats, stat), StatUpdate::Ap(stats.ap)],
        )?))
    }
}

/// `stat`'s value, as STAT_CHANGED sends it.
pub(super) fn stat_value(stats: &CharacterStats, stat: ApStat) -> StatUpdate {
    match stat {
        ApStat::Str => StatUpdate::Str(stats.str),
        ApStat::Dex => StatUpdate::Dex(stats.dex),
        ApStat::Int => StatUpdate::Int(stats.int),
        ApStat::Luk => StatUpdate::Luk(stats.luk),
        ApStat::MaxHp => StatUpdate::MaxHp(stats.max_hp),
        ApStat::MaxMp => StatUpdate::MaxMp(stats.max_mp),
    }
}

/// The stat behind a STAT_CHANGED mask bit that AP goes into.
pub(super) fn ap_stat(mask: i32) -> Option<ApStat> {
    Some(match mask {
        0x40 => ApStat::Str,
        0x80 => ApStat::Dex,
        0x100 => ApStat::Int,
        0x200 => ApStat::Luk,
        0x800 => ApStat::MaxHp,
        0x2000 => ApStat::MaxMp,
        _ => return None,
    })
}

/// `SPEND_SP`: tick, then the skill to raise a level. Answered with the
/// remaining SP and the skill's new level, or just an unlock if refused.
pub struct SpendSpHandler;

impl WorldHandler for SpendSpHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let skill_id = reader.read_int()?;
        let Some(record) = spend_sp(ctx.player, skill_id) else {
            debug!(skill_id, sp = ctx.player.stats.sp, "SP spend refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        Ok(HandlerResult::reply(stat::build_stat_update(
            true,
            &[StatUpdate::Sp(ctx.player.stats.sp)],
        )?)
        .with_reply(skill::build_update_skill(&record)?))
    }
}

fn spend_sp(player: &mut Player, skill_id: i32) -> Option<SkillRecord> {
    if !game::skill::in_job_tree(skill_id, player.stats.job) {
        return None;
    }
    let info = skills::info(skill_id)?;
    if info.invisible && player.skills.get(skill_id).is_none() {
        return None;
    }
    let requirements_met = info
        .requirements
        .iter()
        .all(|&(id, level)| player.skills.level(id) >= level);
    if !requirements_met {
        return None;
    }
    player.raise_skill(skill_id, info.max_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use packet::io::write::PktWrite;

    fn request(op: RecvOpcode, value: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(op as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_int(value).unwrap();
        p
    }

    fn handle(handler: &dyn WorldHandler, player: &mut Player, packet: Packet) -> Vec<i16> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        handler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => p.opcode(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    const STAT: i16 = SendOpcode::StatChange as i16;
    const SKILL: i16 = SendOpcode::UpdateSkills as i16;

    #[test]
    fn ap_raises_the_requested_stat() {
        let mut player = crate::codec::stats::sample_player();
        let (str, ap) = (player.stats.str, player.stats.ap);
        let replies = handle(
            &SpendApHandler,
            &mut player,
            request(RecvOpcode::SpendAp, 0x40),
        );
        assert_eq!(replies, [STAT]);
        assert_eq!((player.stats.str, player.stats.ap), (str + 1, ap - 1));
    }

    #[test]
    fn refused_ap_still_unlocks_the_client() {
        let mut player = crate::codec::stats::sample_player();
        // Current HP is not something AP buys.
        assert_eq!(
            handle(
                &SpendApHandler,
                &mut player,
                request(RecvOpcode::SpendAp, 0x400)
            ),
            [STAT]
        );
        player.stats.ap = 0;
        let dex = player.stats.dex;
        assert_eq!(
            handle(
                &SpendApHandler,
                &mut player,
                request(RecvOpcode::SpendAp, 0x80)
            ),
            [STAT]
        );
        assert_eq!(player.stats.dex, dex);
    }

    #[test]
    fn sp_raises_a_skill_of_the_players_job() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 200;
        player.stats.sp = 1;
        let replies = handle(
            &SpendSpHandler,
            &mut player,
            request(RecvOpcode::SpendSp, 2001002),
        );
        assert_eq!(replies, [STAT, SKILL]);
        assert_eq!((player.skills.level(2001002), player.stats.sp), (1, 0));
    }

    #[test]
    fn sp_is_refused_outside_the_job_or_without_requirements() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 100;
        player.stats.sp = 5;
        // A magician skill, Slash Blast before Power Strike, and an unknown id.
        for skill_id in [2001002, 1001005, 1009999] {
            let replies = handle(
                &SpendSpHandler,
                &mut player,
                request(RecvOpcode::SpendSp, skill_id),
            );
            assert_eq!(replies, [STAT], "{skill_id}");
        }
        assert_eq!(player.stats.sp, 5);
        assert!(player.skills.is_empty());
    }

    #[test]
    fn hidden_skills_need_to_be_granted_first() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 10;
        // Legendary Spirit is a beginner skill the window never shows.
        assert_eq!(
            handle(
                &SpendSpHandler,
                &mut player,
                request(RecvOpcode::SpendSp, 1003)
            ),
            [STAT]
        );
        assert_eq!(
            handle(
                &SpendSpHandler,
                &mut player,
                request(RecvOpcode::SpendSp, 1000)
            ),
            [STAT, SKILL]
        );
    }
}
