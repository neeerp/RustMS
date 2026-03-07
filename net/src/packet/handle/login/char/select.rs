use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::login_world::resolve_login_channel;
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CharacterSelectHandler;

impl CharacterSelectHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for CharacterSelectHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);

        let _op = reader.read_short()?;
        let cid = reader.read_int()?;
        let _mac = reader.read_str_with_length();
        let _hwid = reader.read_str_with_length();

        let session = ctx
            .session
            .session
            .as_ref()
            .ok_or(NetworkError::NotLoggedIn)?;
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

        let redirect_packet =
            build::login::world::build_server_redirect(cid, channel.host, channel.port)?;

        // Attach character to session and send redirect
        Ok(HandlerResult::empty()
            .with_attach_character(cid)
            .with_reply(redirect_packet))
    }
}
