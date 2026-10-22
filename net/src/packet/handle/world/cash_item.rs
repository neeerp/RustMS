//! USE_CASH_ITEM (0x4F, FUN_00a0a63f, report 179): an item used from the
//! Cash Inventory. Every request is the item's Cash slot and id, then the
//! item's own fields and the tick. The client then waits for an answer that
//! unlocks it. Cash pet food, the pet name tag, megaphones, the chalkboard,
//! teleport rocks, notes, the Quick Delivery Coupon, weather items, the
//! jukebox, meso sacks, avatar megaphones, AP Reset, SP Reset, the Vicious'
//! Hammer, Vega's Spell, the cash mists, the Item Tag and the Magical
//! Sandglasses are used so far.

use super::cash_mist::cash_mist;
use super::chalkboard::chalkboard;
use super::hammer::vicious_hammer;
use super::item_guard::{item_guard, ITEM_GUARD};
use super::item_tag::{item_tag, sandglass};
use super::jukebox::jukebox;
use super::megaphone::{avatar_megaphone, megaphone};
use super::meso_sack::meso_sack;
use super::note::note;
use super::pet::{cash_pet_food, pet_name_tag};
use super::stat_reset::{ap_reset, sp_reset};
use super::teleport_rock::teleport_rock;
use super::vega::vega_spell;
use super::weather::weather;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
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
            505 if held && item_id == 5_050_000 => ap_reset(ctx, &mut r, slot),
            505 if held && game::sp_reset::item_tab(item_id).is_some() => {
                sp_reset(ctx, &mut r, slot, item_id)
            }
            506 if held && item_id == 5_060_000 => item_tag(ctx, &mut r, slot),
            506 if held && item_id == ITEM_GUARD => item_guard(ctx, &mut r, slot, item_id),
            // Not 5060002, the incubator.
            506 if held && item_id / 1000 == 5061 => item_guard(ctx, &mut r, slot, item_id),
            507 if held => megaphone(ctx, &mut r, slot, item_id),
            509 if held => note(ctx, &mut r, slot, item_id),
            510 if held => jukebox(ctx, slot, item_id),
            512 if held => weather(ctx, &mut r, slot, item_id),
            517 if held && item_id % 10_000 == 0 => pet_name_tag(ctx, &mut r, slot),
            520 if held => meso_sack(ctx, slot, item_id),
            524 if held => cash_pet_food(ctx, slot, item_id),
            528 if held => cash_mist(ctx, slot, item_id),
            // The coupon is used up by the send it opens the window for.
            533 if held => Ok(HandlerResult::empty()
                .with(WorldAction::Duey(crate::duey::DueyRequest::OpenQuickSend))),
            537 if held => chalkboard(ctx, &mut r),
            539 if held => avatar_megaphone(ctx, &mut r, slot, item_id),
            550 if held => sandglass(ctx, &mut r, slot, item_id),
            // The window waits for 354, so even a hammer not held gets one.
            557 => vicious_hammer(ctx, &mut r, slot, item_id),
            561 if held => vega_spell(ctx, &mut r, slot, item_id),
            _ => Ok(HandlerResult::reply(build_stat_update(true, &[])?)),
        }
    }
}
