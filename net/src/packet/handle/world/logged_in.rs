use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use db::character::CharacterWrapper;
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
        chr: &mut CharacterWrapper,
    ) -> Result<(), NetworkError> {
        client.send(&mut build::world::keymap::build_keymap(&mut chr.key_binds)?)
    }

    pub fn send_character_data(
        &self,
        client: &mut MapleClient,
        chr: &CharacterWrapper,
    ) -> Result<(), NetworkError> {
        client.send(&mut build::world::char::build_char_info(&chr.character)?)
    }

    pub fn send_loaded_data(
        &self,
        client: &mut MapleClient,
        chr: &mut CharacterWrapper,
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
            Ok(character) => self.send_loaded_data(client, &mut character.lock().unwrap()),
            Err(_) => Err(NetworkError::NotLoggedIn),
        }
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncPlayerLoggedInHandler;

impl AsyncPlayerLoggedInHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncPlayerLoggedInHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?; // prune opcode

        let character_id = reader.read_int()?;

        // Get character from session (should have been loaded during reattach)
        match ctx.session.get_character() {
            Ok(character) => {
                let mut chr = character.lock().unwrap();
                let keymap_packet = build::world::keymap::build_keymap(&mut chr.key_binds)?;
                let char_info_packet = build::world::char::build_char_info(&chr.character)?;

                Ok(HandlerResult::empty()
                    .with_reattach_session(character_id)
                    .with_reply(keymap_packet)
                    .with_reply(char_info_packet))
            }
            Err(_) => {
                // Session doesn't have character loaded yet - request reattach first
                // The actor will load the character and call us again
                Ok(HandlerResult::empty()
                    .with_reattach_session(character_id))
            }
        }
    }
}
