use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginAction, LoginContext, LoginHandler, LoginResult};
use crate::login_world::resolve_login_channel;
use crate::packet::build::login::char;
use db::account;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct CharListHandler;

impl LoginHandler for CharListHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_byte()?;
        let world_id = reader.read_byte()?;
        let channel_id = reader.read_byte()?;

        resolve_login_channel(world_id, channel_id)
            .map_err(|_| NetworkError::PacketHandlerError("Selected channel is not configured"))?;

        let account_id = ctx.session()?.account_id;
        let slots = account::get_account_by_id(account_id)?.character_slots;
        let entries = db::player::load_character_entries(account_id)?;
        Ok(HandlerResult::empty()
            .with(LoginAction::SelectChannel {
                world_id,
                channel_id,
            })
            .with_reply(char::build_char_list(&entries, i32::from(slots))?))
    }
}
