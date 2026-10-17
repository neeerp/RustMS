use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Client → server: send a whisper.
pub const WHISPER_REQUEST_MODE: u8 = 0x06;
/// Server → client: the result of the client's whisper.
pub const WHISPER_RESULT_MODE: u8 = 0x0A;
/// Server → client: an incoming whisper.
pub const WHISPER_RECEIVE_MODE: u8 = 0x12;
/// Client → server: find where a character is (`/find`).
pub const FIND_REQUEST_MODE: u8 = 0x05;
/// Server → client: where the character the client looked for is.
pub const FIND_RESULT_MODE: u8 = 0x09;

/// Where a found character is, as the asker may learn it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindLocation {
    /// On the asker's channel, on this map.
    Map(i32),
    /// On another channel, zero-based.
    Channel(u8),
}

/// How a v83 client shows a BROADCAST_MSG (FUN_00a22785), for the kinds
/// that carry only their message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Broadcast {
    /// "[Notice] " and the message in blue chat, or a dialog outside a
    /// field.
    Notice = 0,
    /// A dialog.
    Popup = 1,
    /// Pink chat, as for a mob's Banish.
    Pink = 5,
}

/// A message from the server, shown as `kind` says.
pub fn build_broadcast(kind: Broadcast, message: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(kind as u8)?;
    packet.write_str_with_length(message)?;
    Ok(packet)
}

/// An incoming whisper. `channel` is the sender's zero-based channel.
pub fn build_whisper_receive(
    sender_name: &str,
    channel: u8,
    from_admin: bool,
    message: &str,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(WHISPER_RECEIVE_MODE)?;
    packet.write_str_with_length(sender_name)?;
    packet.write_byte(channel)?;
    packet.write_byte(from_admin as u8)?;
    packet.write_str_with_length(message)?;
    Ok(packet)
}

pub fn build_whisper_result(target_name: &str, success: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(WHISPER_RESULT_MODE)?;
    packet.write_str_with_length(target_name)?;
    packet.write_byte(success as u8)?;
    Ok(packet)
}

/// The answer to a find. A character who isn't online gets the failed
/// whisper result instead.
pub fn build_find_result(
    target_name: &str,
    location: FindLocation,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(FIND_RESULT_MODE)?;
    packet.write_str_with_length(target_name)?;
    match location {
        FindLocation::Map(map_id) => {
            packet.write_byte(1)?;
            packet.write_int(map_id)?;
            // The position on the map; the client shows only the map.
            packet.write_int(0)?;
            packet.write_int(0)?;
        }
        FindLocation::Channel(channel) => {
            packet.write_byte(3)?;
            packet.write_int(i32::from(channel))?;
        }
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn a_broadcast_is_its_kind_and_message() {
        let packet = build_broadcast(Broadcast::Pink, "Begone").unwrap();
        Expect::new(&packet.bytes)
            .short(0x44)
            .byte(5)
            .string("Begone")
            .end();
    }

    #[test]
    fn whisper_receive_matches_reference() {
        let packet = build_whisper_receive("Alice", 2, false, "hello there").unwrap();

        // The reference writes the sender's channel as a short; the v83
        // client reads it as channel byte + admin flag byte.
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::Whisper as i16).byte(0x12);
        expect.string("Alice").short(2).string("hello there");
        expect.end();
    }

    #[test]
    fn whisper_from_admin_sets_the_second_byte() {
        let packet = build_whisper_receive("GM", 0, true, "hi").unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::Whisper as i16).byte(0x12);
        expect.string("GM").byte(0).byte(1).string("hi");
        expect.end();
    }

    #[test]
    fn whisper_result_matches_reference() {
        let found = build_whisper_result("Bob", true).unwrap();
        Expect::new(&found.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x0A)
            .string("Bob")
            .byte(1)
            .end();

        let missing = build_whisper_result("Nobody", false).unwrap();
        Expect::new(&missing.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x0A)
            .string("Nobody")
            .byte(0)
            .end();
    }

    #[test]
    fn find_results_match_reference() {
        let map = build_find_result("Bob", FindLocation::Map(100_000_000)).unwrap();
        Expect::new(&map.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x09)
            .string("Bob")
            .byte(1)
            .int(100_000_000)
            .int(0)
            .int(0)
            .end();

        let channel = build_find_result("Bob", FindLocation::Channel(2)).unwrap();
        Expect::new(&channel.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x09)
            .string("Bob")
            .byte(3)
            .int(2)
            .end();
    }
}
