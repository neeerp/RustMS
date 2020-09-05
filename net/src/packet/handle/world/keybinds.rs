use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use db::keybinding::{KeybindType, NewKeybinding};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct ChangeKeybindsHandler {}

impl ChangeKeybindsHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for ChangeKeybindsHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?; // prune op

        if packet.len() < 2 {
            return Ok(());
        }

        let character;
        match client.get_character() {
            None => {
                return Err(NetworkError::NotLoggedIn);
            }
            Some(chr) => {
                character = chr;
            }
        }

        let character_id = character.id;

        let mut bind_vec = Vec::new();
        if reader.read_int()? == 0 {
            for i in 0..reader.read_int()? {
                let key = reader.read_int()? as i16;
                let bind_type: KeybindType = reader.read_byte()?.into();
                let action = reader.read_int()? as i16;
                bind_vec.push(NewKeybinding {
                    character_id,
                    key,
                    bind_type,
                    action,
                });
            }
        }

        character.upsert_binds(bind_vec)?;

        Ok(())
    }
}
