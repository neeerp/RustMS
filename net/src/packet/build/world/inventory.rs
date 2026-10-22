use crate::codec::item::{encode_item, SlotPrefix};
use crate::codec::look::encode_avatar_look;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{InventoryChange, InventoryType, Player};
use packet::{io::write::PktWrite, Packet};

/// The wire inventory type: worn items travel as EQUIP with a negative slot.
fn wire_type(kind: InventoryType) -> u8 {
    match kind {
        InventoryType::Equipped => InventoryType::Equip as u8,
        other => other as u8,
    }
}

/// INVENTORY_OPERATION: apply `changes` in order. `update_tick` unlocks the
/// client's item actions when the change answers one of its requests.
pub fn build_inventory_operation(
    update_tick: bool,
    changes: &[InventoryChange],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::InventoryOperation as i16)?;
    p.write_byte(update_tick as u8)?;
    p.write_byte(
        u8::try_from(changes.len())
            .map_err(|_| NetworkError::PacketHandlerError("too many inventory changes"))?,
    )?;
    // Equipment moves tell the client to refresh the wearer's look.
    let mut movement = None;
    for change in changes {
        match change {
            InventoryChange::Add { kind, slot, item } => {
                p.write_byte(0)?;
                p.write_byte(wire_type(*kind))?;
                p.write_short(*slot)?;
                encode_item(&mut p, SlotPrefix::None, item)?;
            }
            InventoryChange::Quantity {
                kind,
                slot,
                quantity,
            } => {
                p.write_byte(1)?;
                p.write_byte(wire_type(*kind))?;
                p.write_short(*slot)?;
                p.write_short(*quantity)?;
            }
            InventoryChange::Move { kind, from, to } => {
                p.write_byte(2)?;
                p.write_byte(wire_type(*kind))?;
                p.write_short(*from)?;
                p.write_short(*to)?;
                if *from < 0 || *to < 0 {
                    movement = Some(if *from < 0 { 1 } else { 2 });
                }
            }
            InventoryChange::Remove { kind, slot } => {
                p.write_byte(3)?;
                p.write_byte(wire_type(*kind))?;
                p.write_short(*slot)?;
                if *slot < 0 {
                    movement = Some(2);
                }
            }
        }
    }
    if let Some(movement) = movement {
        p.write_byte(movement)?;
    }
    Ok(p)
}

/// GATHER_ITEM_RESULT: the bag is gathered; the item window offers Sort.
pub fn build_gather_result(kind: InventoryType) -> Result<Packet, NetworkError> {
    build_rearrange_result(SendOpcode::GatherItemResult, kind)
}

/// SORT_ITEM_RESULT: the bag is sorted; the item window offers Gather again.
pub fn build_sort_result(kind: InventoryType) -> Result<Packet, NetworkError> {
    build_rearrange_result(SendOpcode::SortItemResult, kind)
}

fn build_rearrange_result(opcode: SendOpcode, kind: InventoryType) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(opcode as i16)?;
    p.write_byte(0)?;
    p.write_byte(wire_type(kind))?;
    Ok(p)
}

/// SHOW_STATUS_INFO mode 0: "You have gained an item" on the right.
pub fn build_item_gain(item_id: i32, quantity: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(0)?;
    p.write_byte(0)?;
    p.write_int(item_id)?;
    p.write_int(quantity)?;
    p.write_int(0)?;
    p.write_int(0)?;
    Ok(p)
}

/// SHOW_STATUS_INFO mode 8: "The item [name] has been expired, and
/// therefore, deleted from your inventory." for each of up to 255
/// `item_ids` (v83 FUN_00a209d4).
pub fn build_items_expired(item_ids: &[i32]) -> Result<Packet, NetworkError> {
    build_item_lines(8, item_ids)
}

/// SHOW_STATUS_INFO mode 11: "[name]'s seal has expired." for each of up
/// to 255 `item_ids` (v83 FUN_00a2187e, report c009).
pub fn build_seals_expired(item_ids: &[i32]) -> Result<Packet, NetworkError> {
    build_item_lines(11, item_ids)
}

fn build_item_lines(mode: u8, item_ids: &[i32]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(mode)?;
    let item_ids = &item_ids[..item_ids.len().min(255)];
    p.write_byte(item_ids.len() as u8)?;
    for &item_id in item_ids {
        p.write_int(item_id)?;
    }
    Ok(p)
}

/// INVENTORY_GROW: the bag of `kind` now has `slots` slots.
pub fn build_inventory_grow(kind: InventoryType, slots: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::InventoryGrow as i16)?;
    p.write_byte(wire_type(kind))?;
    p.write_byte(slots)?;
    Ok(p)
}

/// SHOW_STATUS_INFO 0/-1: "You can't get anymore items."
pub fn build_inventory_full() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(0)?;
    p.write_byte(0xFF)?;
    Ok(p)
}

/// SHOW_STATUS_INFO 0/-2: "This item is unavailable for the pick-up.", as
/// for a one-of-a-kind item the player already has (FUN_00a20ad9).
pub fn build_item_unavailable() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(0)?;
    p.write_byte(0xFE)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT mode 3: an item gain line in the chat log (quest
/// rewards and turn-ins).
pub fn build_item_gain_in_chat(item_id: i32, quantity: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(3)?;
    p.write_byte(1)?;
    p.write_int(item_id)?;
    p.write_int(quantity)?;
    Ok(p)
}

/// UPDATE_CHAR_LOOK: other players redraw this character's equipment.
pub fn build_char_look(player: &Player) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateCharLook as i16)?;
    p.write_int(player.id())?;
    p.write_byte(1)?;
    encode_avatar_look(&mut p, &player.look(), false)?;
    super::marriage::write_ring_looks(&mut p, player.marriage_ring().as_ref())?;
    p.write_int(0)?; // read by no v83 client
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::Item;

    #[test]
    fn quantity_and_removal_changes() {
        let packet = build_inventory_operation(
            true,
            &[
                InventoryChange::Quantity {
                    kind: InventoryType::Use,
                    slot: 2,
                    quantity: 7,
                },
                InventoryChange::Remove {
                    kind: InventoryType::Etc,
                    slot: 1,
                },
            ],
        )
        .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::InventoryOperation as i16)
            .byte(1)
            .byte(2);
        expect.byte(1).byte(2).short(2).short(7);
        expect.byte(3).byte(4).short(1);
        expect.end();
    }

    #[test]
    fn an_unavailable_pickup_is_status_0_minus_2() {
        let packet = build_item_unavailable().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(0)
            .byte(0xFE);
        expect.end();
    }

    #[test]
    fn a_bag_grows_to_its_new_slot_count() {
        let packet = build_inventory_grow(InventoryType::Etc, 28).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x1E).byte(4).byte(28);
        expect.end();
    }

    #[test]
    fn equipping_is_a_move_with_a_movement_trailer() {
        let packet = build_inventory_operation(
            true,
            &[InventoryChange::Move {
                kind: InventoryType::Equip,
                from: 3,
                to: -1,
            }],
        )
        .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::InventoryOperation as i16)
            .byte(1)
            .byte(1);
        expect.byte(2).byte(1).short(3).short(-1).byte(2);
        expect.end();
        let unequip = build_inventory_operation(
            true,
            &[InventoryChange::Move {
                kind: InventoryType::Equip,
                from: -1,
                to: 3,
            }],
        )
        .unwrap();
        assert_eq!(*unequip.bytes.last().unwrap(), 1);
    }

    #[test]
    fn adding_writes_the_item_without_a_slot_prefix() {
        let packet = build_inventory_operation(
            false,
            &[InventoryChange::Add {
                kind: InventoryType::Use,
                slot: 1,
                item: Item::stack(2010007, 1),
            }],
        )
        .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::InventoryOperation as i16)
            .byte(0)
            .byte(1);
        expect.byte(0).byte(2).short(1);
        expect.byte(2).int(2010007).byte(0);
        expect.long(crate::codec::time::expiration(None));
        expect.short(1).string("").short(0);
        expect.end();
    }

    #[test]
    fn item_gain_messages() {
        let status = build_item_gain(4031802, 1).unwrap();
        let mut expect = Expect::new(&status.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(0)
            .byte(0);
        expect.int(4031802).int(1).int(0).int(0);
        expect.end();
        let chat = build_item_gain_in_chat(4031792, -10).unwrap();
        let mut expect = Expect::new(&chat.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(3)
            .byte(1);
        expect.int(4031792).int(-10);
        expect.end();
        let expired = build_items_expired(&[4031281, 1102135]).unwrap();
        let mut expect = Expect::new(&expired.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(8)
            .byte(2);
        expect.int(4031281).int(1102135);
        expect.end();
    }
}
