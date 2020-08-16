use crate::packet::build::login::world;
use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;

pub struct WorldListHandler {}

impl WorldListHandler {
    pub fn new() -> Self {
        WorldListHandler {}
    }
}

impl PacketHandler for WorldListHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        to_hex_string(&packet.bytes);
        let mut server_list_packet = world::build_world_details();
        let mut end_of_list_packet = world::build_end_of_world_list();
        let mut select_world_packet = world::build_select_world();
        let mut recommended_packet = world::build_send_recommended_worlds();

        client.send(&mut server_list_packet).unwrap();
        client.send(&mut end_of_list_packet).unwrap();
        client.send(&mut select_world_packet).unwrap(); // HeavenClient ignores this...
        client.send(&mut recommended_packet).unwrap();

        Ok(())
    }
}
