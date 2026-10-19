//! AP Reset (5050000), used through USE_CASH_ITEM with the stat the AP goes
//! to and the one it comes from, as STAT_CHANGED mask bits (v83
//! FUN_00a0a63f, type 0x17; report 195). The stats change together and the
//! item is used up; a refusal, or a dead player, keeps it and only unlocks
//! the client.

use super::spend_points::{ap_stat, stat_value};
use crate::handler::{HandlerResult, WorldContext, WorldResult};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use crate::skills;
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

fn refuse() -> WorldResult {
    Ok(HandlerResult::reply(build_stat_update(true, &[])?))
}

#[cfg(test)]
mod tests {
    use crate::handler::{WorldAction, WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::{InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};

    const AP_RESET: i32 = 5050000;

    fn reset(player: &mut Player, to: i32, from: i32) -> Vec<Vec<u8>> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(3).unwrap();
        p.write_int(AP_RESET).unwrap();
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
}
