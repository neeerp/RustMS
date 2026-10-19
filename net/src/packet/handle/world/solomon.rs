//! Writs of Solomon (237xxxx): their EXP is kept aside as gacha EXP, which
//! the player takes in by double-clicking the EXP gauge.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::stat::StatUpdate;
use crate::packet::build::world::{inventory, stat};
use game::{CharacterStats, InventoryChange, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// The highest level that may take in gacha EXP; v83's status bar asks only
/// below 51 (FUN_008d7611), and FUN_00a1288f refuses above it.
const GACHA_EXP_MAX_LEVEL: u8 = 50;

/// `USE_SOLOMON_ITEM` (0x9D, v83 FUN_00a12685): tick, slot in the USE bag,
/// item id. The writ is used up and its `spec/exp` becomes the player's
/// gacha EXP. The client sends none above the writ's `maxLevel` or while
/// gacha EXP is left; nor does the server take one.
pub struct UseSolomonItemHandler;

impl WorldHandler for UseSolomonItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let Some(changes) = use_writ(ctx.player, slot, item_id) else {
            debug!(slot, item_id, "Writ of Solomon refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let gacha_exp = StatUpdate::GachaExp(ctx.player.stats.gacha_exp);
        Ok(
            HandlerResult::reply(inventory::build_inventory_operation(false, &changes)?)
                .with_reply(stat::build_stat_update(true, &[gacha_exp])?),
        )
    }
}

/// Use up the writ in `slot`, if the player may: returns the change to
/// their bag.
fn use_writ(player: &mut Player, slot: i16, item_id: i32) -> Option<Vec<InventoryChange>> {
    if item_id / 10_000 != 237
        || player.stats.gacha_exp != 0
        || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id
    {
        return None;
    }
    let info = items::info(item_id)?;
    let exp = info.spec?.exp;
    if exp <= 0 || (info.max_level > 0 && i32::from(player.stats.level) > info.max_level) {
        return None;
    }
    let changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    player.stats.gacha_exp = exp;
    Some(changes)
}

/// `USE_GACHA_EXP` (0x9E, v83 FUN_00a1288f): tick. The player takes in
/// their gacha EXP (`take_gacha_exp`), which the client asks for only up
/// to level 50 with some left.
pub struct UseGachaExpHandler;

impl WorldHandler for UseGachaExpHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let gain = take_gacha_exp(&mut ctx.player.stats);
        if gain <= 0 {
            debug!("Gacha EXP refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        }
        ctx.player.mark_dirty();
        let left = StatUpdate::GachaExp(ctx.player.stats.gacha_exp);
        Ok(HandlerResult::empty()
            .with(WorldAction::GainExp(gain))
            .with_reply(stat::build_stat_update(true, &[left])?))
    }
}

/// Take what to gain out of the gacha EXP: what finishes the level, and
/// the whole next level too if the rest covers it, or all of it if it
/// doesn't finish this one (Cosmic's gainGachaExp). None above level 50.
fn take_gacha_exp(stats: &mut CharacterStats) -> i32 {
    if stats.level > GACHA_EXP_MAX_LEVEL || stats.gacha_exp <= 0 {
        return 0;
    }
    let pool = i64::from(stats.gacha_exp);
    let need = game::exp::to_next_level(stats.level).unwrap_or(i64::MAX);
    let mut gain = pool;
    if pool + i64::from(stats.exp) >= need {
        gain = (need - i64::from(stats.exp)).max(0);
        let next = game::exp::to_next_level(stats.level + 1).unwrap_or(i64::MAX);
        if pool - gain >= next {
            gain += next;
        }
    }
    stats.gacha_exp = i32::try_from(pool - gain).unwrap_or(0);
    i32::try_from(gain).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    fn handle(handler: &dyn WorldHandler, player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        handler.handle(&mut packet, &mut ctx).unwrap().actions
    }

    fn use_writ(slot: i16, item_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseSolomonItem as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p
    }

    fn take() -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseGachaExp as i16).unwrap();
        p.write_int(0).unwrap();
        p
    }

    fn unlocked_only(actions: &[WorldAction]) -> bool {
        matches!(actions, [WorldAction::Reply(p)] if p.bytes == stat::build_stat_update(true, &[]).unwrap().bytes)
    }

    #[test]
    fn a_writ_becomes_gacha_exp_up_to_its_max_level_and_none_while_some_is_left() {
        if items::data().is_err() {
            return;
        }
        let held = |player: &Player| {
            player
                .inventory
                .get(InventoryType::Use, 2)
                .map(|i| i.quantity)
        };
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 50;
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2370012, 2));
        let actions = handle(&UseSolomonItemHandler, &mut player, use_writ(2, 2370012));
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::Reply(a), WorldAction::Reply(b)]
            if a.opcode() == SendOpcode::InventoryOperation as i16
                && b.bytes == stat::build_stat_update(true, &[StatUpdate::GachaExp(100)]).unwrap().bytes
        ));
        assert_eq!(player.stats.gacha_exp, 100);
        assert_eq!(held(&player), Some(1));

        // Not while some is left, not above level 50, not from another slot.
        let actions = handle(&UseSolomonItemHandler, &mut player, use_writ(2, 2370012));
        assert!(unlocked_only(&actions));
        player.stats.gacha_exp = 0;
        player.stats.level = 51;
        let actions = handle(&UseSolomonItemHandler, &mut player, use_writ(2, 2370012));
        assert!(unlocked_only(&actions));
        player.stats.level = 50;
        let actions = handle(&UseSolomonItemHandler, &mut player, use_writ(3, 2370012));
        assert!(unlocked_only(&actions));
        assert_eq!(held(&player), Some(1));
    }

    #[test]
    fn gacha_exp_finishes_the_level_and_the_next_if_it_covers_it() {
        let mut stats = crate::codec::stats::sample_stats();
        let need = |level| game::exp::to_next_level(level).unwrap() as i32;

        // Short of the level: all of it.
        (stats.level, stats.exp, stats.gacha_exp) = (10, 100, 50);
        assert_eq!(take_gacha_exp(&mut stats), 50);
        assert_eq!(stats.gacha_exp, 0);

        // Enough for the level but not the next: what finishes it.
        (stats.exp, stats.gacha_exp) = (100, need(10));
        assert_eq!(take_gacha_exp(&mut stats), need(10) - 100);
        assert_eq!(stats.gacha_exp, 100);

        // Enough for both: both, the rest kept.
        (stats.exp, stats.gacha_exp) = (100, need(10) + need(11) + 5);
        assert_eq!(take_gacha_exp(&mut stats), need(10) - 100 + need(11));
        assert_eq!(stats.gacha_exp, 105);

        // None above level 50, nor with none.
        (stats.level, stats.gacha_exp) = (51, 1000);
        assert_eq!(take_gacha_exp(&mut stats), 0);
        assert_eq!(stats.gacha_exp, 1000);
        (stats.level, stats.gacha_exp) = (50, 0);
        assert_eq!(take_gacha_exp(&mut stats), 0);
    }

    #[test]
    fn taking_gacha_exp_gains_it_then_shows_what_is_left() {
        let mut player = crate::codec::stats::sample_player();
        (player.stats.level, player.stats.exp, player.stats.gacha_exp) = (10, 0, 40);
        let actions = handle(&UseGachaExpHandler, &mut player, take());
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::GainExp(40), WorldAction::Reply(p)]
            if p.bytes == stat::build_stat_update(true, &[StatUpdate::GachaExp(0)]).unwrap().bytes
        ));
        // The runtime adds the EXP.
        assert_eq!((player.stats.exp, player.stats.gacha_exp), (0, 0));
        let actions = handle(&UseGachaExpHandler, &mut player, take());
        assert!(unlocked_only(&actions));
    }
}
