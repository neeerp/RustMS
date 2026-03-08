use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};
use std::convert::TryFrom;
use std::net::Ipv4Addr;

pub fn build_channel_change(server_ip: Ipv4Addr, server_port: u16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ChangeChannel as i16;

    packet.write_short(op)?;
    packet.write_byte(1)?;
    packet.write_bytes(&server_ip.octets())?;
    packet.write_short(
        i16::try_from(server_port)
            .map_err(|_| NetworkError::PacketHandlerError("channel-change port out of range"))?,
    )?;

    Ok(packet)
}
