use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build::login;
use db::character;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct DeleteCharHandler;

impl LoginHandler for DeleteCharHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        // PIC is disabled (see build_char_list), so the client sends it empty.
        let _pic = reader.read_str_with_length()?;
        let character_id = reader.read_int()?;

        // Deletion is scoped to the session's account.
        match character::delete_character(character_id, ctx.session()?.account_id)? {
            0 => Err(NetworkError::PacketHandlerError(
                "Could not delete character",
            )),
            _ => Ok(HandlerResult::reply(login::char::build_char_delete(
                character_id,
                0x00,
            )?)),
        }
    }
}
