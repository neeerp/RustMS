use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Client → server: send a whisper.
pub const WHISPER_REQUEST_MODE: u8 = 0x06;
/// Server → client: the result of the client's whisper.
pub const WHISPER_RESULT_MODE: u8 = 0x0A;
/// Server → client: an incoming whisper.
pub const WHISPER_RECEIVE_MODE: u8 = 0x12;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

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
}
