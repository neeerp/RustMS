use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use db::character::CharacterDTO;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct PlayerLoggedInHandler {}

impl PlayerLoggedInHandler {
    pub fn new() -> PlayerLoggedInHandler {
        PlayerLoggedInHandler {}
    }

    pub fn send_keybinds(
        &self,
        client: &mut MapleClient,
        chr: &mut CharacterDTO,
    ) -> Result<(), NetworkError> {
        client.send(&mut build::world::keymap::build_keymap(&mut chr.key_binds)?)
    }

    pub fn send_character_data(
        &self,
        client: &mut MapleClient,
        chr: &CharacterDTO,
    ) -> Result<(), NetworkError> {
        client.send(&mut build::world::char::build_char_info(&chr.character)?)
    }

    pub fn send_loaded_data(
        &self,
        client: &mut MapleClient,
        chr: &mut CharacterDTO,
    ) -> Result<(), NetworkError> {
        self.send_keybinds(client, chr)?;
        self.send_character_data(client, chr)
    }
}

impl PacketHandler for PlayerLoggedInHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?; // prune opcode

        let character_id = reader.read_int()?;
        client.reattach(character_id)?;

        match client.session.get_character() {
            Ok(character) => self.send_loaded_data(client, &mut character.borrow_mut()),
            Err(_) => Err(NetworkError::NotLoggedIn),
        }
    }
}
