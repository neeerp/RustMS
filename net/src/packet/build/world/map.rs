use db::character::Character;
use packet::{io::write::PktWrite, Packet};

use crate::{error::NetworkError, helpers, packet::op::SendOpcode};

pub fn build_warp_to_map(character: &Character, map_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::SetField as i16;
    packet.write_short(op)?;

    let channel = 0;
    let spawn_point = 0;
    let hp = character.hp;
    let use_spawn_pos = 0; // If this was non 0, we'd need to provide an x,y

    packet.write_int(channel)?;

    // padding?
    packet.write_int(0)?;
    packet.write_byte(0)?;

    packet.write_int(map_id)?;
    packet.write_byte(spawn_point)?;
    packet.write_short(hp)?;
    packet.write_byte(use_spawn_pos)?;
    packet.write_long(helpers::current_time_i64()?)?;

    Ok(packet)
}

pub fn build_empty_stat_update() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::StatChange as i16;
    packet.write_short(op)?;
    packet.write_byte(1)?;
    packet.write_int(0)?;

    Ok(packet)
}
