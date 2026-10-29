//! AP Reset (5050000) and SP Reset (5050001-5050004), used through
//! USE_CASH_ITEM with where the point goes and where it comes from: STAT_CHANGED
//! mask bits for AP, skill ids for SP (v83 FUN_00a0a63f, types 0x17 and 0x18;
//! report 195). Both change together and the item is used up; a refusal, or a
//! dead player, keeps it and only unlocks the client. A skill reset to
//! nothing leaves the macros that held it, and the client gets the list
//! again (as Cosmic's UseCashItemHandler; the client has no check of its own).

use super::spend_points::{ap_stat, stat_value};
use crate::handler::{HandlerResult, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::skill::build_update_skills;
use crate::packet::build::world::skill_macro::build_skill_macros;
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use crate::skills;
use game::sp_reset::SkillRule;
use game::{ap_reset, InventoryType};
use packet::io::read::PktRead;
use std::io::Cursor;

/// Use the AP Reset in Cash `slot`; `r` is past the item id.
pub fn ap_reset(ctx: &mut WorldContext, r: &mut Cursor<&Vec<u8>>, slot: i16) -> WorldResult {
    let to = ap_stat(r.read_int()?);
    let from = ap_stat(r.read_int()?);
    let (Some(to), Some(from)) = (to, from) else {
        return refuse();
    };
    if ctx.player.stats.hp <= 0 {
        return refuse();
    }
    let mut stats = ctx.player.stats.clone();
    if !ap_reset::reset(&mut stats, from, to, skills::total_int(ctx.player)) {
        return refuse();
    }
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return refuse();
    };
    let mut changed = vec![stat_value(&stats, to), stat_value(&stats, from)];
    if stats.hp != ctx.player.stats.hp {
        changed.push(StatUpdate::Hp(stats.hp));
    }
    if stats.mp != ctx.player.stats.mp {
        changed.push(StatUpdate::Mp(stats.mp));
    }
    ctx.player.stats = stats;
    ctx.player.mark_dirty();
    Ok(HandlerResult::reply(build_stat_update(true, &changed)?)
        .with_reply(build_inventory_operation(true, &changes)?))
}

/// Use the SP Reset `item_id` in Cash `slot`; `r` is past the item id.
pub fn sp_reset(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let to = r.read_int()?;
    let from = r.read_int()?;
    if ctx.player.stats.hp <= 0 {
        return refuse();
    }
    let mut book = ctx.player.skills.clone();
    let rule = |skill_id| {
        skills::info(skill_id).map(|info| SkillRule {
            max_level: info.max_level,
            invisible: info.invisible,
            requirements: info.requirements,
        })
    };
    let Some(changed) =
        game::sp_reset::reset(&ctx.player.stats, &mut book, item_id, to, from, rule)
    else {
        return refuse();
    };
    let Ok(changes) = ctx
        .player
        .inventory_mut()
        .take(InventoryType::Cash, slot, 1)
    else {
        return refuse();
    };
    ctx.player.skills = book;
    ctx.player.mark_dirty();
    let mut result = HandlerResult::reply(build_update_skills(&changed)?)
        .with_reply(build_inventory_operation(true, &changes)?);
    if ctx.player.skills.level(from) == 0 && ctx.player.skill_macros.forget(from) {
        result = result.with_reply(build_skill_macros(&ctx.player.skill_macros)?);
    }
    Ok(result)
}

fn refuse() -> WorldResult {
    Ok(HandlerResult::reply(build_stat_update(true, &[])?))
}

#[cfg(test)]
mod tests {
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::build::world::skill_macro::build_skill_macros;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player, SkillMacro, SkillMacros, SkillRecord};
    use packet::{io::write::PktWrite, Packet};

    const AP_RESET: i32 = 5050000;

    fn reset(player: &mut Player, to: i32, from: i32) -> Vec<Vec<u8>> {
        use_reset(player, AP_RESET, to, from)
    }

    fn use_reset(player: &mut Player, item_id: i32, to: i32, from: i32) -> Vec<Vec<u8>> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(3).unwrap();
        p.write_int(item_id).unwrap();
        p.write_int(to).unwrap();
        p.write_int(from).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseCashItemHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => p.bytes,
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    fn holder() -> Player {
        let mut player = crate::codec::stats::sample_player();
        let stats = &mut player.stats;
        (stats.level, stats.job) = (30, 110);
        (stats.str, stats.dex, stats.int, stats.luk, stats.ap) = (120, 30, 4, 4, 2);
        (stats.max_hp, stats.hp, stats.max_mp, stats.mp) = (1500, 1500, 200, 200);
        player
            .inventory
            .insert(InventoryType::Cash, 3, Item::stack(AP_RESET, 2));
        player
    }

    fn left(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Cash, 3)
            .map_or(0, |item| item.quantity)
    }

    #[test]
    fn an_ap_moves_and_the_item_is_used() {
        let mut player = holder();
        let replies = reset(&mut player, 0x40, 0x80);
        assert_eq!(replies.len(), 2);
        // STAT_CHANGED, unlocking, with STR then DEX.
        assert_eq!(replies[0][..3], [0x1F, 0, 1]);
        assert_eq!(replies[0][3..7], 0xC0i32.to_le_bytes());
        assert_eq!(replies[1][..2], [0x1D, 0], "an inventory operation");
        assert_eq!((player.stats.str, player.stats.dex), (121, 29));
        assert_eq!(left(&player), 1);
    }

    #[test]
    fn max_hp_given_back_takes_hp_down_with_it() {
        let mut player = holder();
        let replies = reset(&mut player, 0x40, 0x800);
        assert_eq!(replies[0][3..7], (0x40i32 | 0x800 | 0x400).to_le_bytes());
        assert_eq!((player.stats.max_hp, player.stats.hp), (1446, 1446));
    }

    #[test]
    fn a_refusal_keeps_the_item_and_unlocks() {
        let mut player = holder();
        for (to, from) in [(0x40, 0x40), (0x40, 0x100), (0x40, 0x10), (0x400, 0x80)] {
            let replies = reset(&mut player, to, from);
            assert_eq!(
                replies,
                [vec![0x1F, 0, 1, 0, 0, 0, 0]],
                "{to:#x} from {from:#x}"
            );
        }
        player.stats.hp = 0;
        assert_eq!(reset(&mut player, 0x40, 0x80).len(), 1, "not when dead");
        assert_eq!(left(&player), 2);
        assert_eq!((player.stats.str, player.stats.int), (120, 4));
    }

    const SP_RESET: i32 = 5050001;
    const POWER_STRIKE: i32 = 1001004;
    const SLASH_BLAST: i32 = 1001005;

    fn skill_holder() -> Option<Player> {
        crate::skills::data().ok()?;
        let mut player = holder();
        player.stats.sp = 1;
        for (skill_id, level) in [(POWER_STRIKE, 20), (SLASH_BLAST, 3)] {
            player.skills.set(SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        player
            .inventory
            .insert(InventoryType::Cash, 3, Item::stack(SP_RESET, 1));
        Some(player)
    }

    #[test]
    fn a_skill_level_moves_and_the_item_is_used() {
        let Some(mut player) = skill_holder() else {
            return;
        };
        let replies = use_reset(&mut player, SP_RESET, SLASH_BLAST, POWER_STRIKE);
        assert_eq!(replies.len(), 2);
        // UPDATE_SKILLS, unlocking, with both skills.
        assert_eq!(replies[0][..5], [0x24, 0, 1, 2, 0]);
        assert_eq!(replies[0][5..9], SLASH_BLAST.to_le_bytes());
        assert_eq!(replies[0][25..29], POWER_STRIKE.to_le_bytes());
        assert_eq!(replies[1][..2], [0x1D, 0], "an inventory operation");
        assert_eq!(
            (
                player.skills.level(SLASH_BLAST),
                player.skills.level(POWER_STRIKE)
            ),
            (4, 19)
        );
        assert_eq!((left(&player), player.stats.sp), (0, 1));
    }

    #[test]
    fn a_skill_reset_to_nothing_leaves_the_macros() {
        let Some(mut player) = skill_holder() else {
            return;
        };
        let holding = |skills| SkillMacro {
            name: "hits".into(),
            silent: false,
            skills,
        };
        player.skill_macros = SkillMacros::new(vec![holding([SLASH_BLAST, 0, SLASH_BLAST])]);
        player.skills.set(SkillRecord {
            skill_id: POWER_STRIKE,
            level: 1,
            master_level: 0,
            expiration: None,
        });
        // Slash Blast down to 2: the macro keeps it.
        let replies = use_reset(&mut player, SP_RESET, POWER_STRIKE, SLASH_BLAST);
        assert_eq!(replies.len(), 2);
        assert_eq!(player.skills.level(SLASH_BLAST), 2);
        // Down to 1, then to nothing.
        let mut replies = vec![];
        for _ in 0..2 {
            player
                .inventory
                .insert(InventoryType::Cash, 3, Item::stack(SP_RESET, 1));
            replies = use_reset(&mut player, SP_RESET, POWER_STRIKE, SLASH_BLAST);
        }
        assert_eq!(player.skills.level(SLASH_BLAST), 0);
        assert_eq!(replies.len(), 3);
        assert_eq!(player.skill_macros.macros(), [holding([0, 0, 0])]);
        assert_eq!(
            replies[2],
            build_skill_macros(&player.skill_macros).unwrap().bytes
        );
    }

    #[test]
    fn a_refused_skill_reset_keeps_the_item() {
        let Some(mut player) = skill_holder() else {
            return;
        };
        // Power Strike is full; Slash Blast to itself; the first job has no SP
        // to spare for the second; a magician's skill.
        for (item, to, from) in [
            (SP_RESET, POWER_STRIKE, SLASH_BLAST),
            (SP_RESET, SLASH_BLAST, SLASH_BLAST),
            (SP_RESET + 1, 1101006, POWER_STRIKE),
            (SP_RESET, 2001002, POWER_STRIKE),
        ] {
            player
                .inventory
                .insert(InventoryType::Cash, 3, Item::stack(item, 1));
            let replies = use_reset(&mut player, item, to, from);
            assert_eq!(replies, [vec![0x1F, 0, 1, 0, 0, 0, 0]], "{to} from {from}");
            assert_eq!(left(&player), 1);
        }
        assert_eq!(
            (
                player.skills.level(SLASH_BLAST),
                player.skills.level(POWER_STRIKE)
            ),
            (3, 20)
        );
    }
}
