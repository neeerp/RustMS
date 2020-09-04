use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use db::{character::Character, keybinding};
use keybinding::Keybinding;
use packet::{io::read::PktRead, Packet};
use std::{collections::HashMap, io::BufReader};

pub struct PlayerLoggedInHandler {}

impl PlayerLoggedInHandler {
    pub fn new() -> PlayerLoggedInHandler {
        PlayerLoggedInHandler {}
    }

    pub fn send_keybinds(
        &self,
        client: &mut MapleClient,
        chr: &Character,
    ) -> Result<(), NetworkError> {
        let bind_vec = keybinding::get_keybindings_by_characterid(chr.id)?;
        let mut bind_map = Keybinding::vec_to_map(bind_vec);

        client.send(&mut build::world::keymap::build_keymap(bind_map)?)
    }

    pub fn send_character_data(
        &self,
        client: &mut MapleClient,
        chr: &Character,
    ) -> Result<(), NetworkError> {
        client.send(&mut build::world::char::build_char_info(&chr)?)
    }

    pub fn send_loaded_data(
        &self,
        client: &mut MapleClient,
        chr: &Character,
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

        match client.get_character() {
            Some(character) => self.send_loaded_data(client, &character),
            None => Err(NetworkError::NotLoggedIn),
        }
    }
}
