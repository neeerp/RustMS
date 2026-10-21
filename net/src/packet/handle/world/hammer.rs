//! Vicious' Hammer (5570000), used through USE_CASH_ITEM with the equip's
//! inventory type and slot (v83 FUN_0082aed3, type 0x42; reports c004 and
//! c004b). Each hammer gives an equip one more upgrade slot, at most two per
//! equip. The window fills its gauge and acks the answer with USE_HAMMER;
//! the answer to that closes it.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::hammer::{
    build_hammer_applied, build_hammer_done, build_hammer_refusal, HammerRefusal,
};
use crate::packet::build::world::inventory::build_inventory_operation;
use game::{InventoryChange, InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The most hammers one equip takes.
pub const MAX_HAMMERS: i32 = 2;
/// The Horntail Necklace, the one equip the client names in its refusal.
const HORNTAIL_NECKLACE: i32 = 1_122_000;
/// The wire type of the Equip tab.
const EQUIP_TAB: i32 = 1;
/// The one hammer.
const HAMMER: i32 = 5_570_000;

/// Use the hammer in Cash `slot`; `r` is past the item id.
pub fn vicious_hammer(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let tab = r.read_int()?;
    let equip_slot = r.read_int()?;
    let Ok(equip_slot) = i16::try_from(equip_slot) else {
        return refuse(HammerRefusal::NotUpgradable);
    };
    let held = ctx.player.inventory.get(InventoryType::Cash, slot);
    let held = held.is_some_and(|item| item.item_id == HAMMER);
    if item_id != HAMMER || !held || tab != EQUIP_TAB || ctx.player.stats.hp <= 0 {
        return refuse(HammerRefusal::NotUpgradable);
    }
    match hammer(ctx.player, slot, equip_slot) {
        Ok((changes, applied)) => {
            ctx.player.mark_dirty();
            Ok(
                HandlerResult::reply(build_inventory_operation(true, &changes)?)
                    .with_reply(build_hammer_applied(applied)?),
            )
        }
        Err(why) => refuse(why),
    }
}

/// Hammer the equip in Equip `equip_slot` with the hammer in Cash `slot`:
/// the inventory changes and how many hammers the equip now has.
fn hammer(
    player: &mut Player,
    slot: i16,
    equip_slot: i16,
) -> Result<(Vec<InventoryChange>, i32), HammerRefusal> {
    use HammerRefusal::*;
    let mut item = player
        .inventory
        .get(InventoryType::Equip, equip_slot)
        .cloned()
        .ok_or(NotUpgradable)?;
    if item.item_id == HORNTAIL_NECKLACE {
        return Err(Horntail);
    }
    let base_slots = items::info(item.item_id)
        .and_then(|info| info.equip)
        .map_or(0, |equip| equip.upgrade_slots);
    // Pet equipment (18xxxxx) takes no hammer either.
    if base_slots == 0 || item.item_id / 100_000 == 18 || item.is_cash() {
        return Err(NotUpgradable);
    }
    let equip = item.equip.as_mut().ok_or(NotUpgradable)?;
    if equip.vicious >= MAX_HAMMERS {
        return Err(UsedUp);
    }
    equip.vicious += 1;
    equip.upgrade_slots = equip.upgrade_slots.saturating_add(1);
    let applied = equip.vicious;

    let mut next = player.inventory.clone();
    let mut changes = next
        .take(InventoryType::Cash, slot, 1)
        .map_err(|_| NotUpgradable)?;
    changes.extend(
        next.take(InventoryType::Equip, equip_slot, 1)
            .map_err(|_| NotUpgradable)?,
    );
    next.insert(InventoryType::Equip, equip_slot, item.clone());
    changes.push(InventoryChange::Add {
        kind: InventoryType::Equip,
        slot: equip_slot,
        item,
    });
    *player.inventory_mut() = next;
    Ok((changes, applied))
}

fn refuse(why: HammerRefusal) -> WorldResult {
    Ok(HandlerResult::reply(build_hammer_refusal(why)?))
}

/// USE_HAMMER (send 0x104, FUN_0082ae28): the window's gauge is full and
/// it acks the answer, echoing its code and flag. The answer closes it.
pub struct UseHammerHandler;

impl WorldHandler for UseHammerHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::reply(build_hammer_done()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    const HAMMER: i32 = 5_570_000;
    /// A Work Glove: 5 upgrade slots.
    const GLOVE: i32 = 1_082_002;

    fn use_hammer(player: &mut Player, tab: i32, equip_slot: i32) -> Vec<Vec<u8>> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(4).unwrap();
        p.write_int(HAMMER).unwrap();
        p.write_int(tab).unwrap();
        p.write_int(equip_slot).unwrap();
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

    fn holder(item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 4, Item::stack(HAMMER, 3));
        player
            .inventory
            .insert(InventoryType::Equip, 2, items::create(item_id, 1).unwrap());
        player
    }

    fn glove(player: &Player) -> game::EquipStats {
        let item = player.inventory.get(InventoryType::Equip, 2).unwrap();
        item.equip.clone().unwrap()
    }

    fn hammers_left(player: &Player) -> i16 {
        player
            .inventory
            .get(InventoryType::Cash, 4)
            .map_or(0, |item| item.quantity)
    }

    fn refusal(reason: i32) -> Vec<u8> {
        let mut bytes = (SendOpcode::ViciousHammer as i16).to_le_bytes().to_vec();
        bytes.push(0x3e);
        bytes.extend(reason.to_le_bytes());
        bytes
    }

    #[test]
    fn a_hammer_adds_an_upgrade_slot_twice_at_most() {
        let mut player = holder(GLOVE);
        let slots = glove(&player).upgrade_slots;

        let replies = use_hammer(&mut player, 1, 2);
        assert_eq!(replies.len(), 2);
        assert_eq!(
            replies[0][..2],
            (SendOpcode::InventoryOperation as i16).to_le_bytes()
        );
        let mut applied = (SendOpcode::ViciousHammer as i16).to_le_bytes().to_vec();
        applied.push(0x39);
        applied.extend(0i32.to_le_bytes());
        applied.extend(1i32.to_le_bytes());
        assert_eq!(replies[1], applied);
        assert_eq!(
            (glove(&player).vicious, glove(&player).upgrade_slots),
            (1, slots + 1)
        );
        assert_eq!(hammers_left(&player), 2);

        use_hammer(&mut player, 1, 2);
        assert_eq!(
            (glove(&player).vicious, glove(&player).upgrade_slots),
            (2, slots + 2)
        );
        assert_eq!(use_hammer(&mut player, 1, 2), vec![refusal(2)]);
        assert_eq!(glove(&player).upgrade_slots, slots + 2);
        assert_eq!(hammers_left(&player), 1);
    }

    #[test]
    fn the_hammer_refuses_what_the_client_would() {
        // Not the Equip tab, an empty slot, the Horntail Necklace, an equip
        // without upgrade slots, and a pet's equip.
        let mut player = holder(GLOVE);
        assert_eq!(use_hammer(&mut player, 2, 2), vec![refusal(1)]);
        assert_eq!(use_hammer(&mut player, 1, 9), vec![refusal(1)]);
        assert_eq!(use_hammer(&mut player, 1, -5), vec![refusal(1)]);
        let mut necklace = holder(HORNTAIL_NECKLACE);
        assert_eq!(use_hammer(&mut necklace, 1, 2), vec![refusal(3)]);
        // A medal has no upgrade slots; a pet's hat has five.
        let mut medal = holder(1_142_000);
        assert_eq!(use_hammer(&mut medal, 1, 2), vec![refusal(1)]);
        let mut pet_hat = holder(1_802_000);
        assert_eq!(use_hammer(&mut pet_hat, 1, 2), vec![refusal(1)]);
        assert_eq!(hammers_left(&player), 3);
        assert_eq!(glove(&player).vicious, 0);
    }

    #[test]
    fn a_hammer_not_held_still_gets_an_answer() {
        // The window waits for 354, so it must not get only a stat change.
        let mut player = holder(GLOVE);
        player.inventory.take(InventoryType::Cash, 4, 3).unwrap();
        assert_eq!(use_hammer(&mut player, 1, 2), vec![refusal(1)]);
        assert_eq!(glove(&player).vicious, 0);
    }

    #[test]
    fn the_ack_closes_the_window() {
        let mut player = holder(GLOVE);
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseHammer as i16).unwrap();
        p.write_int(0x39).unwrap();
        p.write_int(0).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = UseHammerHandler.handle(&mut p, &mut ctx).unwrap().actions;
        let mut done = (SendOpcode::ViciousHammer as i16).to_le_bytes().to_vec();
        done.push(0x3d);
        done.extend(0i32.to_le_bytes());
        assert!(matches!(&actions[..], [WorldAction::Reply(p)] if p.bytes == done));
    }
}
