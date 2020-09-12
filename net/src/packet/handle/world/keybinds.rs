use crate::{error::NetworkError, io::client::MapleClient, packet::handle::PacketHandler};
use db::keybinding::KeybindType;
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

        let character = client.session.get_character()?;
        let mut character = character.borrow_mut();
        if reader.read_int()? == 0 {
            for _ in 0..reader.read_int()? {
                let key = reader.read_int()? as i16;
                let bind_type: KeybindType = reader.read_byte()?.into();
                let action = reader.read_int()? as i16;

                let mut bind = character.key_binds.get(key);
                bind.bind_type = bind_type;
                bind.action = action;
                character.key_binds.set(bind);
            }
        }

        Ok(character.key_binds.save()?)
    }
}
