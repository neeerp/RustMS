use crate::{
    codec::item::{encode_item, SlotPrefix},
    error::NetworkError,
    packet::op::SendOpcode,
    storage::result,
};
use game::{InventoryType, Storage};
use packet::{io::write::PktWrite, Packet};

/// What a TRUNK_RESULT's contents carry; the client keeps the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tabs {
    /// The mesos and every bag's items.
    All,
    /// One bag's items.
    Bag(InventoryType),
    Meso,
}

const MESO: u64 = 2;

impl Tabs {
    /// Bit 2 is the mesos; bit `2 << type` a bag (4 equip up to 0x40 cash).
    fn flags(self) -> u64 {
        match self {
            Self::All => InventoryType::BAGS
                .iter()
                .fold(MESO, |flags, &kind| flags | 2 << kind as u64),
            Self::Bag(kind) => 2 << kind as u64,
            Self::Meso => MESO,
        }
    }
}

/// TRUNK_RESULT `mode` with the storage's slots and the `tabs` it carries:
/// the mesos, then for each bag its item count and items, in order.
pub fn build_storage_contents(
    mode: u8,
    storage: &Storage,
    tabs: Tabs,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::TrunkResult as i16)?;
    packet.write_byte(mode)?;
    write_contents(&mut packet, storage, tabs)?;
    Ok(packet)
}

/// The contents format the trunk shares with Fredrick's Store Bank.
pub(crate) fn write_contents(
    packet: &mut Packet,
    storage: &Storage,
    tabs: Tabs,
) -> Result<(), NetworkError> {
    let flags = tabs.flags();
    packet.write_byte(storage.slots)?;
    packet.write_long(flags as i64)?;
    if flags & MESO != 0 {
        packet.write_int(storage.meso)?;
    }
    for kind in InventoryType::BAGS {
        if flags & 2 << kind as u64 == 0 {
            continue;
        }
        let items: Vec<_> = storage.items(kind).collect();
        packet.write_byte(items.len() as u8)?;
        for item in items {
            encode_item(packet, SlotPrefix::None, item)?;
        }
    }
    Ok(())
}

/// TRUNK_RESULT open: the client opens the storage window of `npc_id`.
pub fn build_open_storage(npc_id: i32, storage: &Storage) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::TrunkResult as i16)?;
    packet.write_byte(result::OPEN)?;
    packet.write_int(npc_id)?;
    write_contents(&mut packet, storage, Tabs::All)?;
    Ok(packet)
}

/// TRUNK_RESULT with only a mode: why a request was refused.
pub fn build_storage_result(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::TrunkResult as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::Item;

    #[test]
    fn open_storage_lists_mesos_and_each_bag() {
        let storage = Storage::new(
            6,
            1234,
            vec![
                Item::stack(4000000, 7),
                Item::stack(2000000, 3),
                Item::stack(4000001, 1),
            ],
        );
        let packet = build_open_storage(1012009, &storage).unwrap();
        let encoded = |item: &Item| {
            let mut packet = Packet::new_empty();
            encode_item(&mut packet, SlotPrefix::None, item).unwrap();
            packet.bytes
        };
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::TrunkResult as i16)
            .byte(0x16)
            .int(1012009);
        expect.byte(6).long(0x7e).int(1234);
        expect.byte(0); // equip
        expect.byte(1).bytes(&encoded(&storage.all()[1])); // use
        expect.byte(0); // setup
        expect.byte(2); // etc, in the order stored
        expect
            .bytes(&encoded(&storage.all()[0]))
            .bytes(&encoded(&storage.all()[2]));
        expect.byte(0); // cash
        expect.end();
    }

    #[test]
    fn a_bags_result_carries_only_that_bag() {
        let storage = Storage::new(
            4,
            50,
            vec![Item::stack(2000000, 3), Item::stack(4000000, 7)],
        );
        let packet =
            build_storage_contents(result::STORED, &storage, Tabs::Bag(InventoryType::Etc))
                .unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::TrunkResult as i16).byte(0x0d);
        expect.byte(4).long(0x20).byte(1).byte(2).int(4000000);

        let packet = build_storage_contents(result::MESO, &storage, Tabs::Meso).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::TrunkResult as i16).byte(0x13);
        expect.byte(4).long(2).int(50);
        expect.end();
    }
}
