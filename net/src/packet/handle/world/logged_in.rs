use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct PlayerLoggedInHandler {}

impl PlayerLoggedInHandler {
    pub fn new() -> PlayerLoggedInHandler {
        PlayerLoggedInHandler {}
    }
}

impl PacketHandler for PlayerLoggedInHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?; // prune opcode

        let character_id = reader.read_int()?;
        client.reattach(character_id)?;

        match client.get_character() {
            Some(character) => client.send(&mut build::world::char::build_char_info(&character)?),
            None => Err(NetworkError::NotLoggedIn),
        }
    }
}
