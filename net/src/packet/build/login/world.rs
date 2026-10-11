use crate::{
    error::NetworkError,
    login_world::{load_login_worlds, LoginWorld},
    packet::op::SendOpcode,
};
use packet::{io::write::PktWrite, Packet};
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
        packet.write_byte(0)?; // not an adult-only channel
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
    packet.write_byte(0)?; // no recommended worlds

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
    // Unsigned on the wire; the cast keeps the bits.
    packet.write_short(server_port as i16)?;
    packet.write_int(cid)?;

    packet.write_bytes(&vec![0u8; 5])?;

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::login_world::LoginChannel;

    fn world(world_id: u8) -> LoginWorld {
        let channel = |channel_id: u8, capacity: u16| LoginChannel {
            world_id,
            channel_id,
            name: format!("Scania-{}", channel_id + 1),
            host: Ipv4Addr::new(127, 0, 0, 1),
            port: 8485 + u16::from(channel_id),
            capacity,
        };
        LoginWorld {
            world_id,
            name: "Scania".to_string(),
            flag: 2,
            event_message: "Hello".to_string(),
            channels: vec![channel(0, 700), channel(1, 600)],
        }
    }

    #[test]
    fn end_of_world_list_is_a_lone_ff() {
        let packet = build_end_of_world_list().unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::ServerList as i16)
            .byte(0xFF)
            .end();
    }

    #[test]
    fn world_details_match_reference() {
        let packet = build_world_details(&world(0)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ServerList as i16);
        expect.byte(0).string("Scania").byte(2).string("Hello");
        expect.byte(100).byte(0).byte(100).byte(0).byte(0); // rate modifiers
        expect.byte(2); // channel count
        expect.string("Scania-1").int(700).byte(1).byte(0).byte(0);
        expect.string("Scania-2").int(600).byte(1).byte(1).byte(0);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn channel_entries_are_never_adult_only() {
        let packet = build_world_details(&world(2)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ServerList as i16);
        expect.byte(2).string("Scania").byte(2).string("Hello");
        expect.byte(100).byte(0).byte(100).byte(0).byte(0);
        expect.byte(2);
        expect.string("Scania-1").int(700).byte(1).byte(0).byte(0);
        expect.string("Scania-2").int(600).byte(1).byte(1).byte(0);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn world_list_has_one_details_packet_per_world() {
        let packets = build_world_list_packets().unwrap();
        let worlds = load_login_worlds().unwrap();
        assert_eq!(packets.len(), worlds.len());
        for (packet, world) in packets.iter().zip(&worlds) {
            assert_eq!(packet.bytes, build_world_details(world).unwrap().bytes);
        }
    }

    #[test]
    fn select_world_carries_the_world_id() {
        let packet = build_select_world().unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::LastConnectedWorld as i16)
            .int(0)
            .end();
    }

    #[test]
    fn no_recommended_worlds_is_a_zero_count() {
        let packet = build_send_recommended_worlds().unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::RecommendedWorlds as i16)
            .byte(0)
            .end();
    }


    #[test]
    fn server_status_is_a_short() {
        let packet = build_server_status(1).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::ServerStatus as i16)
            .short(1)
            .end();
    }

    #[test]
    fn server_redirect_matches_reference() {
        let packet = build_server_redirect(0x0A0B_0C0D, Ipv4Addr::new(10, 1, 2, 3), 8485).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ServerIp as i16).short(0);
        expect.bytes(&[10, 1, 2, 3]).short(8485).int(0x0A0B_0C0D);
        expect.bytes(&[0; 5]);
        expect.end();
    }

    #[test]
    fn server_redirect_accepts_high_ports() {
        let packet = build_server_redirect(7, Ipv4Addr::LOCALHOST, 40000).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ServerIp as i16).short(0);
        expect.bytes(&[127, 0, 0, 1]).short(40000u16 as i16).int(7);
        expect.bytes(&[0; 5]);
        expect.end();
    }
}
