use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::stat::StatUpdate;
use crate::packet::build::world::{inventory, stat};
use game::{InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `USE_ITEM`: tick, slot in the USE bag, item id. Applies the item's `spec`
/// (HP/MP, flat and percent) and consumes one.
pub struct UseItemHandler;

impl WorldHandler for UseItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        match use_item(ctx.player, slot, item_id) {
            Some(changes) => {
                let stats = &ctx.player.stats;
                Ok(
                    HandlerResult::reply(inventory::build_inventory_operation(true, &changes)?)
                        .with_reply(stat::build_stat_update(
                            true,
                            &[StatUpdate::Hp(stats.hp), StatUpdate::Mp(stats.mp)],
                        )?),
                )
            }
            None => {
                debug!(slot, item_id, "Item use refused");
                Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
            }
        }
    }
}

fn use_item(player: &mut Player, slot: i16, item_id: i32) -> Option<Vec<game::InventoryChange>> {
    if player.stats.hp <= 0 {
        return None;
    }
    if player.inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let spec = items::info(item_id)?.spec?;
    let changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    let max_hp = i32::from(player.stats.max_hp);
    let max_mp = i32::from(player.stats.max_mp);
    player.add_hp(spec.hp + max_hp * spec.hp_r / 100);
    player.add_mp(spec.mp + max_mp * spec.mp_r / 100);
    Some(changes)
}

/// `HEAL_OVER_TIME`: the client's periodic natural recovery. The runtime
/// checks the timing; this only parses and bounds the amounts.
pub struct HealOverTimeHandler;

/// Most a beginner recovers per tick without skills (HP, MP).
pub const MAX_NATURAL_HEAL: (i16, i16) = (20, 10);

impl WorldHandler for HealOverTimeHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?;
        reader.read_int()?;
        let hp = reader.read_short()?;
        let mp = reader.read_short()?;
        if !(0..=MAX_NATURAL_HEAL.0).contains(&hp) || !(0..=MAX_NATURAL_HEAL.1).contains(&mp) {
            debug!(hp, mp, "Natural recovery out of range");
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::NaturalHeal { hp, mp }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    fn request(slot: i16, item_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseItem as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseItemHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn eating_an_apple_heals_and_consumes_it() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 50;
        player.stats.hp = 25;
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2010007, 1));
        let actions = handle(&mut player, request(3, 2010007));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(a), WorldAction::Reply(b)]
            if a.opcode() == SendOpcode::InventoryOperation as i16 && b.opcode() == SendOpcode::StatChange as i16)
        );
        assert_eq!(player.stats.hp, 50);
        assert!(player.inventory.get(InventoryType::Use, 3).is_none());
    }

    #[test]
    fn mismatched_or_missing_items_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2000000, 5));
        for packet in [request(1, 2000001), request(2, 2000000)] {
            let actions = handle(&mut player, packet);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16));
        }
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            5
        );
    }
}
