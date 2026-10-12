use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginAction, LoginContext, LoginHandler, LoginResult};
use crate::login_world::resolve_login_channel;
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct CharacterSelectHandler;

impl LoginHandler for CharacterSelectHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let character_id = reader.read_int()?;

        let session = ctx.session()?;
        if !db::character::character_belongs_to(character_id, session.account_id)? {
            return Err(NetworkError::PacketHandlerError(
                "Selected character does not belong to this account",
            ));
        }

        let world_id = session
            .selected_world_id
            .ok_or(NetworkError::PacketHandlerError(
                "No world selected for character select",
            ))? as u8;
        let channel_id = session
            .selected_channel_id
            .ok_or(NetworkError::PacketHandlerError(
                "No channel selected for character select",
            ))? as u8;
        let channel = resolve_login_channel(world_id, channel_id)
            .map_err(|_| NetworkError::PacketHandlerError("Selected channel is not configured"))?;

        // The ticket must exist before the client is told where to connect.
        let redirect_packet =
            build::login::world::build_server_redirect(character_id, channel.host, channel.port)?;
        Ok(HandlerResult::empty()
            .with(LoginAction::EnterWorld { character_id })
            .with_reply(redirect_packet))
    }
}
