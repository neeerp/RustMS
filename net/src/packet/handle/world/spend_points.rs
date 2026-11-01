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

/// `AUTO_ASSIGN_AP`: tick, a count, then each stat's mask bit and the AP it
/// gets. The Stat window's auto-assign sends two pairs that add up to the
/// player's AP (0xa23b3d); it works the split out from totals with
/// equipment, so the server only checks the request is AP the player has:
/// STR, DEX, INT or LUK, each once, none below 0. A stat stops at its cap
/// and the rest stays AP. Answered once either way, as SPEND_AP is.
pub struct AutoAssignApHandler;

impl WorldHandler for AutoAssignApHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let count = reader.read_int()?;
        let mut pairs = Vec::new();
        if (1..=4).contains(&count) {
            for _ in 0..count {
                pairs.push((reader.read_int()?, reader.read_int()?));
            }
        }
        let Some(spends) = auto_spends(&pairs, ctx.player.stats.ap) else {
            debug!(?pairs, ap = ctx.player.stats.ap, "AP auto-assign refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let bonus = skills::growth_bonus(ctx.player, false);
        for (stat, amount) in spends {
            for _ in 0..amount {
                if !ctx.player.spend_ap(stat, bonus, skills::growth_roll) {
                    break;
                }
            }
        }
        let stats = &ctx.player.stats;
        Ok(HandlerResult::reply(stat::build_stat_update(
            true,
            &[
                StatUpdate::Str(stats.str),
                StatUpdate::Dex(stats.dex),
                StatUpdate::Int(stats.int),
                StatUpdate::Luk(stats.luk),
                StatUpdate::Ap(stats.ap),
            ],
        )?))
    }
}

/// The stats an auto-assign raises and by how much, if the request is one
/// the player can pay for.
fn auto_spends(pairs: &[(i32, i32)], ap: i16) -> Option<Vec<(ApStat, i32)>> {
    let mut spends: Vec<(ApStat, i32)> = Vec::new();
    let mut total = 0i64;
    for &(mask, amount) in pairs {
        let stat = ap_stat(mask).filter(|stat| !matches!(stat, ApStat::MaxHp | ApStat::MaxMp))?;
        if amount < 0 || spends.iter().any(|&(spent, _)| spent == stat) {
            return None;
        }
        total += i64::from(amount);
        spends.push((stat, amount));
    }
    (total > 0 && total <= i64::from(ap)).then_some(spends)
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
    if info.disabled || info.invisible && player.skills.get(skill_id).is_none() {
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

    fn auto_assign(pairs: &[(i32, i32)]) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::AutoAssignAp as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_int(pairs.len() as i32).unwrap();
        for &(mask, amount) in pairs {
            p.write_int(mask).unwrap();
            p.write_int(amount).unwrap();
        }
        p
    }

    #[test]
    fn auto_assign_spends_the_pairs_at_once() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.ap = 5;
        let (str, dex) = (player.stats.str, player.stats.dex);
        let replies = handle(
            &AutoAssignApHandler,
            &mut player,
            auto_assign(&[(0x80, 3), (0x40, 2)]),
        );
        assert_eq!(replies, [STAT]);
        assert_eq!(
            (player.stats.str, player.stats.dex, player.stats.ap),
            (str + 2, dex + 3, 0)
        );
        // A pair may be 0, and a stat at its cap keeps the rest as AP.
        player.stats.ap = 5;
        player.stats.luk = 998;
        handle(
            &AutoAssignApHandler,
            &mut player,
            auto_assign(&[(0x100, 0), (0x200, 5)]),
        );
        assert_eq!((player.stats.luk, player.stats.ap), (999, 4));
    }

    #[test]
    fn a_refused_auto_assign_changes_nothing_but_still_unlocks() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.ap = 5;
        let before = player.stats.clone();
        for pairs in [
            // More than the AP, below 0, HP, a stat twice, nothing at all.
            vec![(0x80, 3), (0x40, 3)],
            vec![(0x80, 6), (0x40, -1)],
            vec![(0x800, 1), (0x40, 4)],
            vec![(0x40, 2), (0x40, 3)],
            vec![(0x40, 0), (0x80, 0)],
            vec![],
            vec![(0x40, 1); 5],
        ] {
            let replies = handle(&AutoAssignApHandler, &mut player, auto_assign(&pairs));
            assert_eq!(replies, [STAT], "{pairs:?}");
            assert_eq!(player.stats, before, "{pairs:?}");
        }
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
    fn sp_never_raises_blessing_of_the_fairy() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.link(Some(("Sib".into(), 10)));
        player.stats.sp = 1;
        let replies = handle(
            &SpendSpHandler,
            &mut player,
            request(RecvOpcode::SpendSp, 12),
        );
        assert_eq!(replies, [STAT]);
        assert_eq!((player.skills.level(12), player.stats.sp), (1, 1));
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
