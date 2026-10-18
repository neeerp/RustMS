use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::loot::Loot;
use crate::packet::build::world::{inventory, stat};
use game::{InventoryType, Item, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `ITEM_MOVE`: tick, inventory type, source slot, destination slot,
/// quantity. A negative destination equips; a negative source unequips;
/// destination 0 drops `quantity` of the item where the player stands.
pub struct ItemMoveHandler;

impl WorldHandler for ItemMoveHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let kind = reader.read_byte()? as i8;
        let from = reader.read_short()?;
        let to = reader.read_short()?;
        let quantity = reader.read_short()?;

        let mut actions = HandlerResult::empty();
        if to == 0 {
            match drop_item(ctx.player, kind, from, quantity) {
                Ok((changes, loot, vanish)) => {
                    return Ok(actions
                        .with(WorldAction::Reply(inventory::build_inventory_operation(
                            true, &changes,
                        )?))
                        .with(WorldAction::Field(FieldRequest::Throw { loot, vanish })));
                }
                Err(reason) => {
                    debug!(kind, from, quantity, reason, "Item drop refused");
                    return Ok(
                        actions.with(WorldAction::Reply(stat::build_stat_update(true, &[])?))
                    );
                }
            }
        }

        let result = move_item(ctx.player, kind, from, to);
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

/// Take `quantity` of the item in bag slot `from` to throw on the ground.
/// Returns the changes, what to throw, and whether it vanishes instead of
/// landing (quest and untradeable items).
fn drop_item(
    player: &mut Player,
    kind: i8,
    from: i16,
    quantity: i16,
) -> Result<(Vec<game::InventoryChange>, Loot, bool), &'static str> {
    if player.stats.hp <= 0 {
        return Err("dead");
    }
    let field = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id));
    if !field.is_none_or(|field| field.may_drop()) {
        return Err("the map forbids drops");
    }
    let kind = InventoryType::from_i16(i16::from(kind))
        .filter(|kind| *kind != InventoryType::Equipped)
        .ok_or("not a bag")?;
    let item = player
        .inventory
        .get(kind, from)
        .ok_or("empty slot")?
        .clone();
    // Throwing stars and bullets always go as a whole set.
    let quantity = if matches!(item.item_id / 10_000, 207 | 233) {
        item.quantity
    } else {
        quantity
    };
    if quantity < 1 || quantity > item.quantity {
        return Err("bad quantity");
    }
    let changes = player
        .inventory_mut()
        .take(kind, from, quantity)
        .map_err(|_| "bad quantity")?;
    let vanish = items::info(item.item_id).is_some_and(|info| info.quest || info.trade_block);
    Ok((changes, Loot::Dropped(Item { quantity, ..item }), vanish))
}

/// Apply a move; returns the changes and whether the worn equipment changed.
fn move_item(
    player: &mut Player,
    kind: i8,
    from: i16,
    to: i16,
) -> Result<(Vec<game::InventoryChange>, bool), String> {
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
        drop_request(kind, from, to, 1)
    }

    fn drop_request(kind: i8, from: i16, to: i16, quantity: i16) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ItemMove as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(kind as u8).unwrap();
        p.write_short(from).unwrap();
        p.write_short(to).unwrap();
        p.write_short(quantity).unwrap();
        p
    }

    /// What a drop throws on the field, if it was accepted.
    fn thrown(actions: &[WorldAction]) -> Option<(&Loot, bool)> {
        match actions {
            [WorldAction::Reply(p), WorldAction::Field(FieldRequest::Throw { loot, vanish })]
                if p.opcode() == SendOpcode::InventoryOperation as i16 =>
            {
                Some((loot, *vanish))
            }
            _ => None,
        }
    }

    #[test]
    fn dropping_takes_part_of_a_stack_and_keeps_equipment_as_it_was() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 10));
        let actions = handle(&mut player, drop_request(4, 1, 0, 3));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(4000019, 3)), false))
        );
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Etc, 1)
                .unwrap()
                .quantity,
            7
        );
        assert!(player.is_dirty());

        let scrolled = Item::equipment(
            1302000,
            EquipStats {
                watk: 20,
                ..EquipStats::default()
            },
        );
        player
            .inventory
            .insert(InventoryType::Equip, 1, scrolled.clone());
        let actions = handle(&mut player, drop_request(1, 1, 0, 1));
        assert_eq!(thrown(&actions), Some((&Loot::Dropped(scrolled), false)));
        assert!(player.inventory.get(InventoryType::Equip, 1).is_none());
    }

    #[test]
    fn stars_go_as_a_set_and_bad_quantities_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2070000, 500));
        let actions = handle(&mut player, drop_request(2, 1, 0, 1));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(2070000, 500)), false))
        );

        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 2));
        for quantity in [0, -1, 3] {
            let actions = handle(&mut player, drop_request(4, 1, 0, quantity));
            assert!(thrown(&actions).is_none());
        }
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Etc, 1)
                .unwrap()
                .quantity,
            2
        );

        player.stats.hp = 0;
        assert!(thrown(&handle(&mut player, drop_request(4, 1, 0, 1))).is_none());
    }

    #[test]
    fn refuses_drops_where_the_map_forbids_them() {
        if crate::get_game_data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 910_000_001;
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(4000019, 2));
        assert!(thrown(&handle(&mut player, drop_request(4, 1, 0, 1))).is_none());
        assert_eq!(player.inventory.count(4000019), 2);
    }

    #[test]
    fn quest_items_vanish() {
        // Green Apple, a quest item in v83.
        if items::info(2010007).is_none() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2010007, 1));
        let actions = handle(&mut player, drop_request(2, 1, 0, 1));
        assert_eq!(
            thrown(&actions),
            Some((&Loot::Dropped(Item::stack(2010007, 1)), true))
        );
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
