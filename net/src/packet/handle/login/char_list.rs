use crate::packet::build::login::char;
use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;

pub struct CharListHandler {}

impl CharListHandler {
    pub fn new() -> Self {
        CharListHandler {}
    }
}

impl PacketHandler for CharListHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        to_hex_string(&packet.bytes);

        client.send(&mut char::build_char_list()).unwrap();

        Ok(())
    }
}
