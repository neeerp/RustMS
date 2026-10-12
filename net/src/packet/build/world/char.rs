use crate::codec::{encode_character_data, time::filetime};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::Player;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// SET_FIELD for world entry, carrying the full character data block.
pub fn build_char_info(player: &Player, channel_id: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetField as i16)?;
    packet.write_int(i32::from(channel_id))?;
    packet.write_byte(1)?;
    packet.write_byte(1)?;
    packet.write_short(0)?;

    // Seeds for the client's damage RNG.
    for _ in 0..3 {
        packet.write_int(rand::random::<i32>())?;
    }

    encode_character_data(&mut packet, player)?;
    packet.write_long(filetime(SystemTime::now()))?;
    Ok(packet)
}
