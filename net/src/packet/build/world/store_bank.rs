//! Fredrick's Store Bank (report 165 §4): STORE_BANK (311) and
//! STORE_BANK_MESSAGE (310).

use super::storage::{write_contents, Tabs};
use crate::store_bank::{mode, NOTHING_KEPT};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::Storage;
use packet::{io::write::PktWrite, Packet};

fn store_bank(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::StoreBank as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// The Store Bank window of `npc_id`, with the mesos and items it keeps,
/// in the trunk's format.
pub fn build_store_bank(npc_id: i32, kept: &Storage) -> Result<Packet, NetworkError> {
    let mut packet = store_bank(mode::OPEN)?;
    packet.write_int(npc_id)?;
    write_contents(&mut packet, kept, Tabs::All)?;
    Ok(packet)
}

/// The fee for taking everything, after `days`; the client asks the player
/// to confirm.
pub fn build_store_bank_fee(days: i32, fee: i32) -> Result<Packet, NetworkError> {
    let mut packet = store_bank(mode::FEE)?;
    packet.write_int(days)?;
    packet.write_int(fee)?;
    Ok(packet)
}

/// `npc_id` says the player's merchant is open on `map_id` in `channel`
/// (0-based), or, with `None`, that he keeps nothing for them.
pub fn build_store_bank_elsewhere(
    npc_id: i32,
    open_at: Option<(i32, u8)>,
) -> Result<Packet, NetworkError> {
    let (map_id, channel) = open_at.unwrap_or(NOTHING_KEPT);
    let mut packet = store_bank(mode::ELSEWHERE)?;
    packet.write_int(npc_id)?;
    packet.write_int(map_id)?;
    packet.write_byte(channel)?;
    Ok(packet)
}

/// "This is currently unavailable. Please try again later"
pub fn build_store_bank_unavailable() -> Result<Packet, NetworkError> {
    store_bank(mode::UNAVAILABLE)
}

/// What came of the confirm (`store_bank::message`).
pub fn build_store_bank_message(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::StoreBankMessage as i16)?;
    packet.write_byte(code)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::item::{encode_item, SlotPrefix};
    use crate::codec::testing::Expect;
    use game::Item;

    const OP: i16 = SendOpcode::StoreBank as i16;

    #[test]
    fn the_window_lists_the_mesos_and_each_bag() {
        let kept = Storage::new(16, 99_600, vec![Item::stack(4000000, 5)]);
        let packet = build_store_bank(9030000, &kept).unwrap();
        let mut shell = Packet::new_empty();
        encode_item(&mut shell, SlotPrefix::None, &kept.all()[0]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x23).int(9030000);
        expect.byte(16).long(0x7e).int(99_600);
        expect.byte(0).byte(0).byte(0); // equip, use, setup
        expect.byte(1).bytes(&shell.bytes); // etc
        expect.byte(0); // cash
        expect.end();
    }

    #[test]
    fn the_fee_the_place_and_the_messages() {
        let packet = build_store_bank_fee(3, 7_500).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x24).int(3).int(7_500).end();

        let packet = build_store_bank_elsewhere(9030000, Some((910000003, 1))).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x25).int(9030000).int(910000003);
        expect.byte(1).end();

        let packet = build_store_bank_elsewhere(9030000, None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x25).int(9030000).int(999_999_999);
        expect.byte(0xff).end();

        let packet = build_store_bank_unavailable().unwrap();
        Expect::new(&packet.bytes).short(OP).byte(0x26).end();

        let packet = build_store_bank_message(0x1e).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::StoreBankMessage as i16)
            .byte(0x1e)
            .end();
    }
}
