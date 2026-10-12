use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::{inventory, stat};
use game::{InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `ITEM_MOVE`: tick, inventory type, source slot, destination slot,
/// quantity. A negative destination equips; a negative source unequips;
/// destination 0 drops the item (not supported until there are drops).
pub struct ItemMoveHandler;

impl WorldHandler for ItemMoveHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let kind = reader.read_byte()? as i8;
        let from = reader.read_short()?;
        let to = reader.read_short()?;
        reader.read_short()?; // quantity, for drops

        let result = move_item(ctx.player, kind, from, to);
        let mut actions = HandlerResult::empty();
        match result {
            Ok((changes, look_changed)) if !changes.is_empty() => {
                actions = actions.with(WorldAction::Reply(inventory::build_inventory_operation(
                    true, &changes,
                )?));
                if look_changed {
                    actions = actions.with(WorldAction::LookChanged(inventory::build_char_look(
                        ctx.player,
                    )?));
                }
            }
            outcome => {
                debug!(kind, from, to, ?outcome, "Item move refused");
                // Unlock the client's item actions.
                actions = actions.with(WorldAction::Reply(stat::build_stat_update(true, &[])?));
            }
        }
        Ok(actions)
    }
}

/// Apply a move; returns the changes and whether the worn equipment changed.
fn move_item(
    player: &mut Player,
    kind: i8,
    from: i16,
    to: i16,
) -> Result<(Vec<game::InventoryChange>, bool), String> {
    if to == 0 {
        return Err("dropping items is not supported yet".into());
    }
    if from < 0 {
        // HeavenClient sends EQUIPPED (-1), the original client EQUIP (1).
        let changes = player
            .inventory_mut()
            .unequip(from, to)
            .map_err(|e| e.to_string())?;
        return Ok((changes, true));
    }
    let kind = InventoryType::from_i16(i16::from(kind))
        .filter(|kind| *kind != InventoryType::Equipped)
        .ok_or("unknown inventory type")?;
    if to < 0 {
        if kind != InventoryType::Equip {
            return Err("only equipment can be worn".into());
        }
        let item_id = player
            .inventory
            .get(kind, from)
            .ok_or("empty slot")?
            .item_id;
        let template = items::info(item_id)
            .and_then(|info| info.equip)
            .ok_or("unknown equipment")?;
        if !items::can_wear(&template, player) {
            return Err("requirements not met".into());
        }
        let changes = player
            .inventory_mut()
            .equip(from, to)
            .map_err(|e| e.to_string())?;
        return Ok((changes, true));
    }
    let item_id = player
        .inventory
        .get(kind, from)
        .ok_or("empty slot")?
        .item_id;
    let changes = player
        .inventory_mut()
        .move_in_bag(kind, from, to, items::slot_max(item_id))
        .map_err(|e| e.to_string())?;
    Ok((changes, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::{inventory::equip_slot, EquipStats, Item};
    use packet::io::write::PktWrite;

    fn request(kind: i8, from: i16, to: i16) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ItemMove as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(kind as u8).unwrap();
        p.write_short(from).unwrap();
        p.write_short(to).unwrap();
        p.write_short(1).unwrap();
        p
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ItemMoveHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn unequipping_and_moving_reply_with_inventory_changes() {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1302000, EquipStats::default()),
        );
        let actions = handle(&mut player, request(-1, equip_slot::WEAPON, 1));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(p), WorldAction::LookChanged(_)]
            if p.opcode() == SendOpcode::InventoryOperation as i16)
        );
        assert!(player.inventory.get(InventoryType::Equip, 1).is_some());
        assert!(player.is_dirty());

        let actions = handle(&mut player, request(1, 1, 5));
        assert!(matches!(actions.as_slice(), [WorldAction::Reply(_)]));
        assert!(player.inventory.get(InventoryType::Equip, 5).is_some());
    }

    #[test]
    fn refused_moves_only_unlock_the_client() {
        let mut player = crate::codec::stats::sample_player();
        for request in [request(2, 1, 2), request(1, 1, 0), request(9, 1, 2)] {
            let actions = handle(&mut player, request);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16));
        }
        assert!(!player.is_dirty());
    }
}
