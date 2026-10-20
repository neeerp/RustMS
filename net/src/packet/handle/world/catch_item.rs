//! Catch items (227xxxx, report 227): thrown at a mob, they catch it and
//! turn into the item they make, such as the Pheromone Perfume's Hog.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::stat::build_stat_update;
use game::{Inventory, InventoryChange, InventoryType, Player};
use game_data::items::CatchInfo;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// USE_CATCH_ITEM (0x51, FUN_00a09bdf): the tick, the USE slot, the item
/// and the mob the client found within the item's area. The field decides
/// whether the mob is caught; the client stays locked until it hears.
pub struct UseCatchItemHandler;

impl WorldHandler for UseCatchItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?; // client tick
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let object_id = r.read_int()?;
        let Some(catch) = may_throw(ctx.player, slot, item_id) else {
            debug!(slot, item_id, "Catch item refused");
            return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
        };
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::Catch {
                object_id,
                slot,
                item_id,
                catch,
            })),
        )
    }
}

/// What the catch item `item_id` in USE `slot` catches, if the player may
/// throw it: they're alive, it is there, and what it makes would fit.
fn may_throw(player: &Player, slot: i16, item_id: i32) -> Option<CatchInfo> {
    if player.stats.hp <= 0 || item_id / 10_000 != 227 {
        return None;
    }
    let catch = items::info(item_id)?.catch?;
    // The Element Rock's chance and the waits of it and the Fish Net
    // aren't supported yet.
    if catch.use_delay > 0 || catch.prop < 100.0 {
        return None;
    }
    trade_catch(&mut player.inventory.clone(), slot, item_id, catch.create)?;
    Some(catch)
}

/// Trade the catch item `item_id` in USE `slot` for the `create` it makes;
/// `None`, with the bag partly changed, if it isn't there or that won't
/// fit.
pub fn trade_catch(
    inventory: &mut Inventory,
    slot: i16,
    item_id: i32,
    create: i32,
) -> Option<Vec<InventoryChange>> {
    if inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let mut changes = inventory.take(InventoryType::Use, slot, 1).ok()?;
    changes.extend(items::give(inventory, create, 1).ok()?);
    Some(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{EquipStats, Item};

    const PERFUME: i32 = 2270000;
    const HOG: i32 = 1902000;

    #[test]
    fn the_perfume_is_thrown_while_the_hog_would_fit() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(PERFUME, 1));
        let catch = may_throw(&player, 2, PERFUME).unwrap();
        assert_eq!((catch.mob, catch.create), (9300101, HOG));
        assert!(may_throw(&player, 1, PERFUME).is_none());
        player.inventory.set_slot_limit(InventoryType::Equip, 1);
        player.inventory.insert(
            InventoryType::Equip,
            1,
            Item::equipment(1302000, EquipStats::default()),
        );
        assert!(may_throw(&player, 2, PERFUME).is_none());
    }

    #[test]
    fn the_element_rock_and_fish_net_wait_for_later() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2270002, 1));
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2270008, 1));
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2270001, 1));
        assert!(may_throw(&player, 1, 2270002).is_none());
        assert!(may_throw(&player, 2, 2270008).is_none());
        assert_eq!(may_throw(&player, 3, 2270001).unwrap().mob_hp, 40);
    }
}
