use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::names::{self, forbidden_names};
use crate::packet::build::login;
use crate::packet::build::login::char::NameCheck;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct CheckCharNameHandler;

/// Whether the account may give a new character `name`.
pub(super) fn check_name(name: &str, account_id: i32) -> Result<NameCheck, NetworkError> {
    let gm = db::account::get_account_by_id(account_id)?.gm_level > 0;
    let forbidden = forbidden_names().map_err(NetworkError::PacketHandlerError)?;
    if !names::allowed(name, gm, forbidden) {
        return Ok(NameCheck::Refused);
    }
    // A name waiting in a Name Change request is taken too.
    Ok(if db::name_change::claimed(name)? {
        NameCheck::Taken
    } else {
        NameCheck::Available
    })
}

impl LoginHandler for CheckCharNameHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        let check = check_name(&name, ctx.session()?.account_id)?;
        Ok(HandlerResult::reply(login::char::build_char_name_response(
            &name, check,
        )?))
    }
}
