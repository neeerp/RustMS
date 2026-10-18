//! Mini rooms (v83 `PLAYER_INTERACTION`): for now only the trading room.
//! Every packet starts with a mode byte.

use crate::codec::encode_avatar_look;
use crate::codec::item::{encode_item, SlotPrefix};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{AvatarLook, Item};
use packet::{io::write::PktWrite, Packet};

/// Server → client modes.
const INVITE_MODE: u8 = 0x02;
const INVITE_RESULT_MODE: u8 = 0x03;
const VISIT_MODE: u8 = 0x04;
const ENTER_MODE: u8 = 0x05;
const CHAT_MODE: u8 = 0x06;
const LEAVE_MODE: u8 = 0x0a;
const PUT_ITEM_MODE: u8 = 0x0f;
const PUT_MESO_MODE: u8 = 0x10;
const CONFIRMED_MODE: u8 = 0x11;
const MESO_LIMIT_MODE: u8 = 0x15;

/// The trading room's type and size.
pub const TRADE_ROOM: u8 = 3;
const TRADE_USERS: u8 = 2;

/// A chat line from a member (7 would be a game notice).
const CHAT_LINE: u8 = 8;

/// Why an invitation went nowhere (INVITE_RESULT).
pub mod invite_result {
    /// "Unable to find the character."
    pub const NOT_FOUND: u8 = 1;
    /// "'%s' is doing something else right now."
    pub const BUSY: u8 = 2;
    /// "'%s' have denied invitation."
    pub const DECLINED: u8 = 3;
    /// "'%s' is currently not accepting any invitation."
    pub const NOT_ACCEPTING: u8 = 4;
}

/// Why the client couldn't enter a room (ENTER with type 0).
pub mod enter_error {
    /// "The room is already closed."
    pub const CLOSED: u8 = 1;
}

/// How a trade ended, shown to the member who stays (LEAVE).
pub mod leave_reason {
    /// "Trade cancelled by the other character."
    pub const CANCELLED: u8 = 2;
    /// "Trade successful. Please check the results."
    pub const DONE: u8 = 7;
    /// "Trade unsuccessful."
    pub const FAILED: u8 = 8;
    /// One of the items is one-of-a-kind and the receiver has one.
    pub const ONE_OF_A_KIND: u8 = 9;
}

fn room_packet(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::PlayerInteraction as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// `from` invites the receiving client into trading room `sn`.
pub fn build_trade_invite(from: &str, sn: i32) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_MODE)?;
    packet.write_byte(TRADE_ROOM)?;
    packet.write_str_with_length(from)?;
    packet.write_int(sn)?;
    Ok(packet)
}

/// The invited character couldn't be found.
pub fn build_invite_not_found() -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_RESULT_MODE)?;
    packet.write_byte(invite_result::NOT_FOUND)?;
    Ok(packet)
}

/// `target` is busy or turned the invitation down (`invite_result`).
pub fn build_invite_result(result: u8, target: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(INVITE_RESULT_MODE)?;
    packet.write_byte(result)?;
    packet.write_str_with_length(target)?;
    Ok(packet)
}

/// The receiving client enters a trading room in `my_slot`, with the
/// members already in it as `(slot, look, name)`.
pub fn build_trade_room(
    my_slot: u8,
    members: &[(u8, &AvatarLook, &str)],
) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ENTER_MODE)?;
    packet.write_byte(TRADE_ROOM)?;
    packet.write_byte(TRADE_USERS)?;
    packet.write_byte(my_slot)?;
    for &(slot, look, name) in members {
        packet.write_byte(slot)?;
        encode_avatar_look(&mut packet, look, false)?;
        packet.write_str_with_length(name)?;
    }
    packet.write_byte(0xFF)?;
    Ok(packet)
}

/// The receiving client couldn't enter the room (`enter_error`).
pub fn build_enter_error(error: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(ENTER_MODE)?;
    packet.write_byte(0)?;
    packet.write_byte(error)?;
    Ok(packet)
}

/// Someone joined the receiving client's room in `slot`.
pub fn build_room_visit(slot: u8, look: &AvatarLook, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(VISIT_MODE)?;
    packet.write_byte(slot)?;
    encode_avatar_look(&mut packet, look, false)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// The member in `slot` said `line`, already formatted as `name : text`.
pub fn build_room_chat(slot: u8, line: &str) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(CHAT_MODE)?;
    packet.write_byte(CHAT_LINE)?;
    packet.write_byte(slot)?;
    packet.write_str_with_length(line)?;
    Ok(packet)
}

/// The room closes for the receiving client, sitting in `slot`, for
/// `reason` (`leave_reason`).
pub fn build_room_leave(slot: u8, reason: u8) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(LEAVE_MODE)?;
    packet.write_byte(slot)?;
    packet.write_byte(reason)?;
    Ok(packet)
}

/// An item went into trade slot `slot`, on the receiver's own side
/// (`partner` false) or the partner's.
pub fn build_trade_item(partner: bool, slot: u8, item: &Item) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(PUT_ITEM_MODE)?;
    packet.write_byte(partner as u8)?;
    packet.write_byte(slot)?;
    encode_item(&mut packet, SlotPrefix::None, item)?;
    Ok(packet)
}

/// A side's offered mesos are now `meso` in all.
pub fn build_trade_meso(partner: bool, meso: i32) -> Result<Packet, NetworkError> {
    let mut packet = room_packet(PUT_MESO_MODE)?;
    packet.write_byte(partner as u8)?;
    packet.write_int(meso)?;
    Ok(packet)
}

/// The partner pressed Trade.
pub fn build_trade_confirmed() -> Result<Packet, NetworkError> {
    room_packet(CONFIRMED_MODE)
}

/// A character of level 15 or below offered more than the meso limit; the
/// client takes back its Trade press.
pub fn build_trade_meso_limit() -> Result<Packet, NetworkError> {
    room_packet(MESO_LIMIT_MODE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use std::collections::BTreeMap;

    const OP: i16 = SendOpcode::PlayerInteraction as i16;

    fn look(hair: i32) -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair,
            visible: BTreeMap::from([(5, 1040002), (6, 1060002)]),
            masked: BTreeMap::new(),
            cash_weapon: None,
        }
    }

    #[test]
    fn invite_names_the_room_type_inviter_and_room() {
        let packet = build_trade_invite("Alice", 42).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x02)
            .byte(3)
            .string("Alice")
            .int(42)
            .end();
    }

    #[test]
    fn invite_results_carry_the_target_except_not_found() {
        let missing = build_invite_not_found().unwrap();
        Expect::new(&missing.bytes)
            .short(OP)
            .byte(0x03)
            .byte(1)
            .end();
        let busy = build_invite_result(invite_result::BUSY, "Bob").unwrap();
        Expect::new(&busy.bytes)
            .short(OP)
            .byte(0x03)
            .byte(2)
            .string("Bob")
            .end();
    }

    #[test]
    fn enter_lists_each_member_and_ends_with_ff() {
        let (alice, bob) = (look(30000), look(30030));
        let packet = build_trade_room(1, &[(0, &alice, "Alice"), (1, &bob, "Bob")]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x05).byte(3).byte(2).byte(1);
        expect.byte(0).look(&alice, false).string("Alice");
        expect.byte(1).look(&bob, false).string("Bob");
        expect.byte(0xFF).end();
    }

    #[test]
    fn enter_error_has_room_type_zero() {
        let packet = build_enter_error(enter_error::CLOSED).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x05)
            .byte(0)
            .byte(1)
            .end();
    }

    #[test]
    fn visit_carries_the_slot_look_and_name() {
        let bob = look(30030);
        let packet = build_room_visit(1, &bob, "Bob").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(OP).byte(0x04).byte(1).look(&bob, false);
        expect.string("Bob").end();
    }

    #[test]
    fn chat_and_leave() {
        let chat = build_room_chat(1, "Bob : hi").unwrap();
        Expect::new(&chat.bytes)
            .short(OP)
            .byte(0x06)
            .byte(8)
            .byte(1)
            .string("Bob : hi")
            .end();
        let leave = build_room_leave(0, leave_reason::DONE).unwrap();
        Expect::new(&leave.bytes)
            .short(OP)
            .byte(0x0a)
            .byte(0)
            .byte(7)
            .end();
    }

    #[test]
    fn offers_say_whose_side_they_are_on() {
        let item = Item::stack(2000000, 12);
        let mut encoded = Packet::new_empty();
        encode_item(&mut encoded, SlotPrefix::None, &item).unwrap();
        let packet = build_trade_item(true, 3, &item).unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x0f)
            .byte(1)
            .byte(3)
            .bytes(&encoded.bytes)
            .end();
        let meso = build_trade_meso(false, 5000).unwrap();
        Expect::new(&meso.bytes)
            .short(OP)
            .byte(0x10)
            .byte(0)
            .int(5000)
            .end();
    }

    #[test]
    fn confirmed_and_meso_limit_are_bare_modes() {
        let confirmed = build_trade_confirmed().unwrap();
        Expect::new(&confirmed.bytes).short(OP).byte(0x11).end();
        let limit = build_trade_meso_limit().unwrap();
        Expect::new(&limit.bytes).short(OP).byte(0x15).end();
    }
}
