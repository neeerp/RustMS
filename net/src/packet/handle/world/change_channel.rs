use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::login_world::resolve_login_channel;
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct ChangeChannelHandler;

impl WorldHandler for ChangeChannelHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let target_channel_id = reader.read_byte()?;
        let _tick = reader.read_int()?;

        // The client waits on an answer before it sends another request, so
        // a refusal still answers.
        if target_channel_id == ctx.channel_id
            || resolve_login_channel(ctx.player.world_id, target_channel_id).is_err()
        {
            let enable_actions = build::world::stat::build_stat_update(true, &[])?;
            return Ok(HandlerResult::reply(enable_actions));
        }

        Ok(HandlerResult::empty().with(WorldAction::ChangeChannel {
            channel_id: target_channel_id,
        }))
    }
}
