use crate::handler::{HandlerResult, PreGameHandler, WorldAction, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// PLAYER_LOGGEDIN (0x14): `int character_id`, as in v83. The channel is the
/// one whose listener accepted the connection, never a client-supplied value.
pub struct PlayerLoggedInHandler;

impl PreGameHandler for PlayerLoggedInHandler {
    fn handle(&self, packet: &mut Packet) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let character_id = reader.read_int()?;
        Ok(HandlerResult::empty().with(WorldAction::EnterGame { character_id }))
    }
}
