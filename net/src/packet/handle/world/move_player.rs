use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;
use std::io::BufReader;

pub struct PlayerMoveHandler {}

impl PlayerMoveHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for PlayerMoveHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut _reader = BufReader::new(&**packet);
        println!("Received packet: {}", to_hex_string(&packet.bytes));

        Ok(())
    }
}
