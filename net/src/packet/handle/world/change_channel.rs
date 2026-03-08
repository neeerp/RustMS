use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::login_world::resolve_login_channel;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct ChangeChannelHandler;

impl ChangeChannelHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for ChangeChannelHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;
        let target_channel_id = reader.read_byte()?;
        let _tick = reader.read_int()?;

        let session = ctx
            .session
            .session
            .as_ref()
            .ok_or(NetworkError::NotLoggedIn)?;
        let world_id = session
            .selected_world_id
            .ok_or(NetworkError::PacketHandlerError(
                "No world selected for channel change",
            ))? as u8;
        let current_channel_id = session
            .selected_channel_id
            .ok_or(NetworkError::PacketHandlerError(
                "No channel selected for channel change",
            ))? as u8;

        if target_channel_id == current_channel_id {
            return Ok(HandlerResult::empty());
        }

        resolve_login_channel(world_id, target_channel_id)
            .map_err(|_| NetworkError::PacketHandlerError("Selected channel is not configured"))?;

        Ok(HandlerResult::empty().with_change_channel(world_id, target_channel_id))
    }
}
