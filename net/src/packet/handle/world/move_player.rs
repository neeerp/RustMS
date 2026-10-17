use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::field::build_player_move;
use packet::Packet;

/// Field key byte and CRC int before the path.
const PATH_OFFSET: usize = 2 + 5;
/// The path's start position before its elements.
const MOVEMENT_HEADER_LEN: usize = 9;

pub struct PlayerMoveHandler;

impl WorldHandler for PlayerMoveHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if packet.bytes.len() <= 2 + MOVEMENT_HEADER_LEN {
            return Ok(HandlerResult::empty());
        }

        let movement_fragment = &packet.bytes[(2 + MOVEMENT_HEADER_LEN)..];
        if movement_fragment[0] == 0 {
            return Ok(HandlerResult::empty());
        }

        let path = &packet.bytes[PATH_OFFSET..];
        let start = (
            i16::from_le_bytes([path[0], path[1]]),
            i16::from_le_bytes([path[2], path[3]]),
        );
        let movement_bytes = movement_fragment.to_vec();
        let movement_packet = build_player_move(ctx.client_id, start, &movement_bytes)?;
        Ok(HandlerResult::empty().with(WorldAction::FieldMove {
            packet: movement_packet,
            start,
            movement_bytes,
        }))
    }
}
