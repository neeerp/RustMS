use crate::error::NetworkError;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::login_world::resolve_login_channel;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct ChangeChannelHandler;

impl WorldHandler for ChangeChannelHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let target_channel_id = reader.read_byte()?;
        let _tick = reader.read_int()?;

        if target_channel_id == ctx.channel_id {
            return Ok(HandlerResult::empty());
        }

        resolve_login_channel(ctx.player.world_id, target_channel_id)
            .map_err(|_| NetworkError::PacketHandlerError("Selected channel is not configured"))?;

        Ok(HandlerResult::empty().with(WorldAction::ChangeChannel {
            channel_id: target_channel_id,
        }))
    }
}
