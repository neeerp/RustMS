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
