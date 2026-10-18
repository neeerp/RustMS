//! USE_CASH_ITEM (0x4F, FUN_00a0a63f, report 179): an item used from the
//! Cash Inventory. Every request is the item's Cash slot and id, then the
//! item's own fields and the tick. The client then waits for an answer that
//! unlocks it. Only cash pet food is used so far.

use super::pet::cash_pet_food;
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
            524 if held => cash_pet_food(ctx, slot, item_id),
            _ => Ok(HandlerResult::reply(build_stat_update(true, &[])?)),
        }
    }
}
