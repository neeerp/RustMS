use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build;
use db::account;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct SetGenderHandler;

impl SetGenderHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for SetGenderHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let confirmed = reader.read_byte()?;
        if confirmed != 0x01 {
            return Err(NetworkError::PacketHandlerError("Set Gender packet is invalid."));
        }

        let gender = reader.read_byte()?;

        // Get account_id from session
        let account_id = ctx.session.session.as_ref()
            .map(|s| s.account_id)
            .ok_or(NetworkError::NotLoggedIn)?;

        // Load and update account
        let mut user = account::get_account_by_id(account_id)?;
        user.gender = gender as i16;
        let user = account::update_account(&user)?;

        let login_packet = build::login::status::build_successful_login_packet(&user)?;
        Ok(HandlerResult::reply(login_packet))
    }
}
