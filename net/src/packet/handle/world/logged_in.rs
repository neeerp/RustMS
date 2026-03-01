use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct PlayerLoggedInHandler;

impl PlayerLoggedInHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for PlayerLoggedInHandler {
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
                Ok(HandlerResult::empty().with_reattach_session(character_id))
            }
        }
    }
}
