use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

pub fn build_end_of_world_list() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    packet.write_short(op)?;
    packet.write_byte(0xFF)?;

    Ok(packet)
}

pub fn build_world_details() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    let world_name = "Scania";
    let server_id = 0;
    let flag = 0;
    let event_msg = "Test!";

    packet.write_short(op)?;
    packet.write_byte(server_id)?;
    packet.write_str_with_length(world_name)?;
    packet.write_byte(flag)?;
    packet.write_str_with_length(event_msg)?;
    packet.write_byte(100)?;
    packet.write_byte(0)?;
    packet.write_byte(100)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?;
    packet.write_byte(3)?;

    // Channel 1 info
    packet.write_str_with_length("Scania-1")?;
    packet.write_int(700)?;
    packet.write_byte(1)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?;

    packet.write_str_with_length("Scania-2")?;
    packet.write_int(700)?;
    packet.write_byte(1)?;
    packet.write_byte(1)?;
    packet.write_byte(0)?;

    packet.write_str_with_length("Scania-3")?;
    packet.write_int(700)?;
    packet.write_byte(1)?;
    packet.write_byte(2)?;
    packet.write_byte(0)?;

    packet.write_short(0)?;

    Ok(packet)
}

pub fn build_select_world() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::LastConnectedWorld as i16;

    packet.write_short(op)?;
    packet.write_int(0)?;

    Ok(packet)
}

pub fn build_send_recommended_worlds() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::RecommendedWorlds as i16;

    packet.write_short(op)?;
    packet.write_byte(1)?; // No worlds recommended
    packet.write_int(0)?;
    packet.write_int(0)?;

    Ok(packet)
}

pub fn build_server_status(status: i16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerStatus as i16;

    packet.write_short(op)?;
    packet.write_short(status)?; // Highly populated status!

    Ok(packet)
}

pub fn build_server_redirect() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerIp as i16;

    let server_ip = vec![127, 0, 0, 1];
    let server_port = 8485;
    let client_id = 1;

    packet.write_short(op)?;
    packet.write_short(0)?;

    packet.write_bytes(&server_ip)?;
    packet.write_short(server_port)?;
    packet.write_int(client_id)?;

    packet.write_bytes(&vec![0u8; 5])?;

    Ok(packet)
}
