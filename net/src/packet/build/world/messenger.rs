//! Maple Messenger (v83 `MESSENGER`): a chat room of up to three characters
//! anywhere in the world. Every packet starts with a mode byte.

use crate::codec::encode_avatar_look;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::AvatarLook;
use packet::{io::write::PktWrite, Packet};

/// Client → server modes.
pub const MESSENGER_OPEN_MODE: u8 = 0x00;
pub const MESSENGER_LEAVE_MODE: u8 = 0x02;
pub const MESSENGER_INVITE_MODE: u8 = 0x03;
pub const MESSENGER_DECLINE_MODE: u8 = 0x05;
pub const MESSENGER_CHAT_MODE: u8 = 0x06;

/// Server → client modes.
const ADD_MODE: u8 = 0x00;
const JOIN_MODE: u8 = 0x01;
const REMOVE_MODE: u8 = 0x02;
const INVITE_MODE: u8 = 0x03;
const INVITE_RESULT_MODE: u8 = 0x04;
const DECLINED_MODE: u8 = 0x05;
const CHAT_MODE: u8 = 0x06;
const UPDATE_MODE: u8 = 0x07;

/// The most characters one messenger holds.
pub const MESSENGER_SLOTS: usize = 3;

fn messenger_packet(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Messenger as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

fn member_packet(
    mode: u8,
    slot: u8,
    look: &AvatarLook,
    name: &str,
    channel: u8,
) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(mode)?;
    packet.write_byte(slot)?;
    encode_avatar_look(&mut packet, look, true)?;
    packet.write_str_with_length(name)?;
    packet.write_byte(channel)?;
    packet.write_byte(0)?;
    Ok(packet)
}

/// Another character sits in `slot`. `channel` is zero-based.
pub fn build_messenger_add(
    slot: u8,
    look: &AvatarLook,
    name: &str,
    channel: u8,
) -> Result<Packet, NetworkError> {
    member_packet(ADD_MODE, slot, look, name, channel)
}

/// A member changed channel or equipment.
pub fn build_messenger_update(
    slot: u8,
    look: &AvatarLook,
    name: &str,
    channel: u8,
) -> Result<Packet, NetworkError> {
    member_packet(UPDATE_MODE, slot, look, name, channel)
}

/// The receiving client joined a messenger and sits in `slot`.
pub fn build_messenger_join(slot: u8) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(JOIN_MODE)?;
    packet.write_byte(slot)?;
    Ok(packet)
}

/// The member in `slot` left.
pub fn build_messenger_remove(slot: u8) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(REMOVE_MODE)?;
    packet.write_byte(slot)?;
    Ok(packet)
}

/// `from` invites the receiving client into messenger `messenger_id`.
pub fn build_messenger_invite(from: &str, messenger_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(INVITE_MODE)?;
    packet.write_str_with_length(from)?;
    packet.write_byte(0)?;
    packet.write_int(messenger_id)?;
    packet.write_byte(0)?;
    Ok(packet)
}

/// Whether the invitation to `target` went out.
pub fn build_messenger_invite_result(target: &str, sent: bool) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(INVITE_RESULT_MODE)?;
    packet.write_str_with_length(target)?;
    packet.write_byte(sent as u8)?;
    Ok(packet)
}

/// `target` declined the receiving client's invitation.
pub fn build_messenger_declined(target: &str) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(DECLINED_MODE)?;
    packet.write_str_with_length(target)?;
    packet.write_byte(0)?;
    Ok(packet)
}

/// A chat line, already formatted as `name : text`.
pub fn build_messenger_chat(line: &str) -> Result<Packet, NetworkError> {
    let mut packet = messenger_packet(CHAT_MODE)?;
    packet.write_str_with_length(line)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use std::collections::BTreeMap;

    const OP: i16 = SendOpcode::Messenger as i16;

    fn look() -> AvatarLook {
        AvatarLook {
            gender: 0,
            skin: 0,
            face: 20000,
            hair: 30000,
            visible: BTreeMap::from([(5, 1040002), (6, 1060002), (11, 1302000)]),
            masked: BTreeMap::new(),
            cash_weapon: None,
            pets: [0; 3],
        }
    }

    #[test]
    fn add_and_update_carry_the_member_look_name_and_channel() {
        for (packet, mode) in [
            (build_messenger_add(1, &look(), "Bob", 2).unwrap(), 0x00),
            (build_messenger_update(1, &look(), "Bob", 2).unwrap(), 0x07),
        ] {
            let mut expect = Expect::new(&packet.bytes);
            expect.short(OP).byte(mode).byte(1).look(&look(), true);
            expect.string("Bob").byte(2).byte(0).end();
        }
    }

    #[test]
    fn slot_packets_carry_only_the_slot() {
        let join = build_messenger_join(2).unwrap();
        Expect::new(&join.bytes).short(OP).byte(0x01).byte(2).end();
        let remove = build_messenger_remove(1).unwrap();
        Expect::new(&remove.bytes)
            .short(OP)
            .byte(0x02)
            .byte(1)
            .end();
    }

    #[test]
    fn invite_names_the_inviter_and_the_messenger() {
        let packet = build_messenger_invite("Alice", 7).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(OP)
            .byte(0x03)
            .string("Alice")
            .byte(0)
            .int(7)
            .byte(0);
        expect.end();
    }

    #[test]
    fn notes_name_the_target() {
        let sent = build_messenger_invite_result("Bob", true).unwrap();
        Expect::new(&sent.bytes)
            .short(OP)
            .byte(0x04)
            .string("Bob")
            .byte(1)
            .end();
        let declined = build_messenger_declined("Bob").unwrap();
        Expect::new(&declined.bytes)
            .short(OP)
            .byte(0x05)
            .string("Bob")
            .byte(0)
            .end();
    }

    #[test]
    fn chat_carries_the_formatted_line() {
        let packet = build_messenger_chat("Alice : hi").unwrap();
        Expect::new(&packet.bytes)
            .short(OP)
            .byte(0x06)
            .string("Alice : hi")
            .end();
    }
}
