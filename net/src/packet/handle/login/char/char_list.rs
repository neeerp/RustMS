use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::login::char;
use db::character;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CharListHandler;

impl CharListHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for CharListHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        let world = reader.read_byte()?;
        let channel = reader.read_byte()?;

        // Get account_id from session
        let account_id = ctx
            .session
            .session
            .as_ref()
            .map(|s| s.account_id)
            .ok_or(NetworkError::NotLoggedIn)?;

        let chars = character::get_characters_by_accountid(account_id)?;
        let char_list_packet = char::build_char_list(chars)?;
        Ok(HandlerResult::reply(char_list_packet).with_update_session_selection(world, channel))
    }
}
