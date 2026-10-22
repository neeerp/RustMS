use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::login_world::resolve_login_channel;
use crate::packet::build;
use crate::packet::build::world::map::BlockedServer;
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
        // a refusal still answers. A mini-dungeon's copy is kept on its
        // channel, so nobody leaves it that way (as in Cosmic).
        if target_channel_id == ctx.channel_id
            || resolve_login_channel(ctx.player.world_id, target_channel_id).is_err()
            || crate::dungeons::of_copy(ctx.player.stats.map_id).is_some()
        {
            return Ok(HandlerResult::reply(
                build::world::map::build_blocked_server(BlockedServer::Channel)?,
            ));
        }

        Ok(HandlerResult::empty().with(WorldAction::ChangeChannel {
            channel_id: target_channel_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn nobody_changes_channel_inside_a_mini_dungeon() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 100020100;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::ChangeChannel as i16)
            .unwrap();
        packet.write_byte(1).unwrap();
        packet.write_int(0).unwrap();
        let result = ChangeChannelHandler.handle(&mut packet, &mut ctx).unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::Reply(p)] if p.bytes == [0x84, 0, 1]
        ));
    }
}
