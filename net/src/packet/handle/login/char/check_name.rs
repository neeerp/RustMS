use super::create::name_is_valid;
use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build::login;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct CheckCharNameHandler;

/// Whether `name` is well-formed and not taken.
pub(super) fn name_is_available(name: &str) -> Result<bool, NetworkError> {
    if !name_is_valid(name) {
        return Ok(false);
    }
    match db::character::get_character_by_name(name) {
        Ok(_) => Ok(false),
        Err(db::Error::NotFound) => Ok(true),
        Err(e) => Err(e.into()),
    }
}

impl LoginHandler for CheckCharNameHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        let available = name_is_available(&name)?;
        Ok(HandlerResult::reply(login::char::build_char_name_response(
            &name, available,
        )?))
    }
}
