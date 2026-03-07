use crate::{
    error::NetworkError,
    login_world::{load_login_worlds, LoginWorld},
    packet::op::SendOpcode,
};
use packet::{io::write::PktWrite, Packet};
use std::convert::TryFrom;
use std::net::Ipv4Addr;

pub fn build_end_of_world_list() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    packet.write_short(op)?;
    packet.write_byte(0xFF)?;

    Ok(packet)
}

pub fn build_world_list_packets() -> Result<Vec<Packet>, NetworkError> {
    let worlds = load_login_worlds()?;
    worlds.iter().map(build_world_details).collect()
}

pub fn build_world_details(world: &LoginWorld) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    packet.write_short(op)?;
    packet.write_byte(world.world_id)?;
    packet.write_str_with_length(world.name.as_str())?;
    packet.write_byte(world.flag)?;
    packet.write_str_with_length(world.event_message.as_str())?;
    packet.write_byte(100)?;
    packet.write_byte(0)?;
    packet.write_byte(100)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?;
    packet.write_byte(world.channels.len() as u8)?;

    for channel in &world.channels {
        packet.write_str_with_length(channel.name.as_str())?;
        packet.write_int(i32::from(channel.capacity))?;
        packet.write_byte(1)?;
        packet.write_byte(channel.channel_id)?;
        packet.write_byte(world.world_id)?;
    }

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

pub fn build_server_redirect(
    cid: i32,
    server_ip: Ipv4Addr,
    server_port: u16,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerIp as i16;

    packet.write_short(op)?;
    packet.write_short(0)?;

    packet.write_bytes(&server_ip.octets())?;
    packet.write_short(
        i16::try_from(server_port)
            .map_err(|_| NetworkError::PacketHandlerError("redirect port out of range"))?,
    )?;
    packet.write_int(cid)?;

    packet.write_bytes(&vec![0u8; 5])?;

    Ok(packet)
}
