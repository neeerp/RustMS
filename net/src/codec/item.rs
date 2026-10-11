use super::time::{expiration, ZERO_TIME};
use crate::error::NetworkError;
use game::{Inventory, InventoryType, Item};
use packet::{io::write::PktWrite, Packet};

const ITEM_TYPE_EQUIP: u8 = 1;
const ITEM_TYPE_STACK: u8 = 2;

/// Where an item is being encoded, which decides how its slot is written.
#[derive(Clone, Copy, Debug)]
pub enum SlotPrefix {
    /// No position prefix (e.g. inventory operation packets that carry it separately).
    None,
    /// A bag item: one-byte slot.
    Bag(i16),
    /// An equipped or equip-bag item: two-byte slot, made positive.
    Equip(i16),
}

/// Write one item (`addItemInfo` in HeavenMS).
pub fn encode_item(packet: &mut Packet, prefix: SlotPrefix, item: &Item) -> Result<(), NetworkError> {
    match prefix {
        SlotPrefix::None => {}
        SlotPrefix::Bag(slot) => {
            packet.write_byte(slot as u8)?;
        }
        SlotPrefix::Equip(slot) => {
            let position = slot.abs();
            packet.write_short(if position > 100 { position - 100 } else { position })?;
        }
    }

    let is_cash = item.is_cash();
    packet.write_byte(if item.equip.is_some() {
        ITEM_TYPE_EQUIP
    } else {
        ITEM_TYPE_STACK
    })?;
    packet.write_int(item.item_id)?;
    packet.write_byte(is_cash as u8)?;
    if is_cash {
        packet.write_long(item.cash_serial.unwrap_or(0))?;
    }
    packet.write_long(expiration(item.expiration))?;

    let Some(equip) = &item.equip else {
        packet.write_short(item.quantity)?;
        packet.write_str_with_length(&item.owner)?;
        packet.write_short(item.flags)?;
        if is_rechargeable(item.item_id) {
            packet.write_int(2)?;
            packet.write_bytes(&[0x54, 0, 0, 0x34])?;
        }
        return Ok(());
    };

    packet.write_byte(equip.upgrade_slots)?;
    packet.write_byte(equip.level)?;
    for stat in [
        equip.str, equip.dex, equip.int, equip.luk, equip.hp, equip.mp, equip.watk, equip.matk,
        equip.wdef, equip.mdef, equip.acc, equip.avoid, equip.hands, equip.speed, equip.jump,
    ] {
        packet.write_short(stat)?;
    }
    packet.write_str_with_length(&item.owner)?;
    packet.write_short(item.flags)?;
    if is_cash {
        packet.write_bytes(&[0x40; 10])?;
    } else {
        packet.write_byte(0)?;
        packet.write_byte(equip.item_level)?;
        packet.write_int(equip.item_exp)?;
        packet.write_int(equip.vicious)?;
        packet.write_long(0)?;
    }
    packet.write_long(ZERO_TIME)?;
    packet.write_int(-1)?;
    Ok(())
}

/// Throwing stars and bullets carry an extra recharge block.
fn is_rechargeable(item_id: i32) -> bool {
    matches!(item_id / 10_000, 207 | 233)
}

/// Write slot limits plus every inventory (`addInventoryInfo` in HeavenMS),
/// including the terminator after the cash bag.
pub fn encode_inventory(packet: &mut Packet, inventory: &Inventory) -> Result<(), NetworkError> {
    for kind in InventoryType::BAGS {
        packet.write_byte(inventory.slot_limit(kind))?;
    }
    packet.write_long(ZERO_TIME)?;

    for (slot, item) in inventory.equipped_regular() {
        encode_item(packet, SlotPrefix::Equip(slot), item)?;
    }
    packet.write_short(0)?;
    for (slot, item) in inventory.equipped_cash() {
        encode_item(packet, SlotPrefix::Equip(slot), item)?;
    }
    packet.write_short(0)?;
    for (slot, item) in inventory.items(InventoryType::Equip) {
        encode_item(packet, SlotPrefix::Equip(slot), item)?;
    }
    packet.write_int(0)?;
    for kind in [
        InventoryType::Use,
        InventoryType::Setup,
        InventoryType::Etc,
        InventoryType::Cash,
    ] {
        for (slot, item) in inventory.items(kind) {
            encode_item(packet, SlotPrefix::Bag(slot), item)?;
        }
        packet.write_byte(0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::codec::time::DEFAULT_TIME;
    use game::inventory::equip_slot;
    use game::EquipStats;

    #[test]
    fn stackable_item_layout() {
        let mut packet = Packet::new_empty();
        encode_item(&mut packet, SlotPrefix::Bag(3), &Item::stack(2000000, 12)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.byte(3).byte(2).int(2000000).byte(0).long(DEFAULT_TIME);
        expect.short(12).string("").short(0);
        expect.end();
    }

    #[test]
    fn equipment_layout() {
        let stats = EquipStats {
            upgrade_slots: 7,
            watk: 17,
            ..EquipStats::default()
        };
        let mut packet = Packet::new_empty();
        encode_item(
            &mut packet,
            SlotPrefix::Equip(equip_slot::WEAPON),
            &Item::equipment(1302000, stats),
        )
        .unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(11).byte(1).int(1302000).byte(0).long(DEFAULT_TIME);
        expect.byte(7).byte(0);
        expect.short(0).short(0).short(0).short(0).short(0).short(0);
        expect.short(17);
        expect.short(0).short(0).short(0).short(0).short(0).short(0).short(0).short(0);
        expect.string("").short(0);
        expect.byte(0).byte(0).int(0).int(0).long(0);
        expect.long(ZERO_TIME).int(-1);
        expect.end();
    }

    #[test]
    fn empty_inventory_layout() {
        let mut packet = Packet::new_empty();
        encode_inventory(&mut packet, &Inventory::default()).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.byte(24).byte(24).byte(24).byte(24).byte(96).long(ZERO_TIME);
        expect.short(0).short(0).int(0);
        expect.byte(0).byte(0).byte(0).byte(0);
        expect.end();
    }
}
