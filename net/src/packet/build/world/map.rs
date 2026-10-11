use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::CharacterStats;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// SET_FIELD for a map change: the client warps to `stats.map_id` at portal
/// `stats.spawn_point`.
pub fn build_warp_to_map(stats: &CharacterStats, channel_id: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SetField as i16)?;
    packet.write_int(i32::from(channel_id))?;
    packet.write_int(0)?;
    packet.write_byte(0)?;
    packet.write_int(stats.map_id)?;
    packet.write_byte(stats.spawn_point)?;
    packet.write_short(stats.hp)?;
    packet.write_byte(0)?; // no explicit spawn position follows
    packet.write_long(filetime(SystemTime::now()))?;
    Ok(packet)
}
