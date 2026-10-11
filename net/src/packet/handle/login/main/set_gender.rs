use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build;
use db::account;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct SetGenderHandler;

impl LoginHandler for SetGenderHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        if reader.read_byte()? != 0x01 {
            return Err(NetworkError::PacketHandlerError(
                "Set Gender packet is invalid.",
            ));
        }
        let gender = reader.read_byte()?;
        if gender > 1 {
            return Err(NetworkError::PacketHandlerError("Invalid gender"));
        }

        let mut user = account::get_account_by_id(ctx.session()?.account_id)?;
        user.gender = i16::from(gender);
        let user = account::update_account(&user)?;

        let login_packet = build::login::status::build_successful_login_packet(&user)?;
        Ok(HandlerResult::reply(login_packet))
    }
}
