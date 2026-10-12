use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};
use std::net::Ipv4Addr;

pub fn build_channel_change(server_ip: Ipv4Addr, server_port: u16) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ChangeChannel as i16;

    packet.write_short(op)?;
    packet.write_byte(1)?;
    packet.write_bytes(&server_ip.octets())?;
    // Unsigned on the wire; the cast keeps the bits.
    packet.write_short(server_port as i16)?;

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn channel_change_matches_reference() {
        let packet = build_channel_change(Ipv4Addr::new(192, 168, 1, 9), 8486).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ChangeChannel as i16).byte(1);
        expect.bytes(&[192, 168, 1, 9]).short(8486);
        expect.end();
    }

    #[test]
    fn channel_change_accepts_high_ports() {
        let packet = build_channel_change(Ipv4Addr::LOCALHOST, 40000).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ChangeChannel as i16).byte(1);
        expect.bytes(&[127, 0, 0, 1]).short(40000u16 as i16);
        expect.end();
    }
}
