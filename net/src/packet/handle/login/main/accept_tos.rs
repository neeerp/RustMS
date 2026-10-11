use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginAction, LoginContext, LoginHandler, LoginResult};
use crate::packet::build;
use db::{account, session::SessionState};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct AcceptTOSHandler;

impl LoginHandler for AcceptTOSHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        if reader.read_byte()? != 0x01 {
            return Err(NetworkError::PacketHandlerError(
                "Accept TOS packet is invalid.",
            ));
        }

        let mut user = account::get_account_by_id(ctx.session()?.account_id)?;
        user.accepted_tos = true;
        let user = account::update_account(&user)?;

        let login_packet = build::login::status::build_successful_login_packet(&user)?;
        Ok(HandlerResult::empty()
            .with(LoginAction::SetSessionState(SessionState::AfterLogin))
            .with_reply(login_packet))
    }
}
