use crate::packet::{build, handle::PacketHandler};
use packet::io::read::PktRead;
use std::io::BufReader;

pub struct CharacterSelectHandler {}

impl CharacterSelectHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl PacketHandler for CharacterSelectHandler {
    fn handle(
        &self,
        packet: &mut packet::Packet,
        client: &mut crate::io::client::MapleClient,
    ) -> Result<(), crate::error::NetworkError> {
        let mut reader = BufReader::new(&**packet);

        let _op = reader.read_short()?;
        let _mac = reader.read_str_with_length();
        let _hwid = reader.read_str_with_length();

        println!("Redirecting to port 8485!");
        client.send(&mut build::login::world::build_server_redirect()?)
    }
}
