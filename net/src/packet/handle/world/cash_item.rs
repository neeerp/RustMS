//! USE_CASH_ITEM (0x4F, FUN_00a0a63f, report 179): an item used from the
//! Cash Inventory. Every request is the item's Cash slot and id, then the
//! item's own fields and the tick. The client then waits for an answer that
//! unlocks it. Cash pet food, the pet name tag, megaphones, the chalkboard,
//! teleport rocks and weather items are used so far.

use super::chalkboard::chalkboard;
use super::megaphone::megaphone;
use super::pet::{cash_pet_food, pet_name_tag};
use super::teleport_rock::teleport_rock;
use super::weather::weather;
use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::stat::build_stat_update;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct UseCashItemHandler;

impl WorldHandler for UseCashItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let slot = r.read_short()?;
        let item_id = r.read_int()?;
        let held = ctx
            .player
            .inventory
            .get(InventoryType::Cash, slot)
            .is_some_and(|item| item.item_id == item_id);
        match item_id / 10_000 {
            504 if held => teleport_rock(ctx, &mut r, slot, item_id),
            507 if held => megaphone(ctx, &mut r, slot, item_id),
            512 if held => weather(ctx, &mut r, slot, item_id),
            517 if held && item_id % 10_000 == 0 => pet_name_tag(ctx, &mut r, slot),
            524 if held => cash_pet_food(ctx, slot, item_id),
            537 if held => chalkboard(ctx, &mut r),
            _ => Ok(HandlerResult::reply(build_stat_update(true, &[])?)),
        }
    }
}
