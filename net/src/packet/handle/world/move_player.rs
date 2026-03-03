use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::world::field::build_player_move;
use packet::Packet;

const MOVEMENT_HEADER_LEN: usize = 9;

pub struct PlayerMoveHandler;

impl PlayerMoveHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for PlayerMoveHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        if ctx.client_id == 0 || packet.bytes.len() <= 2 + MOVEMENT_HEADER_LEN {
            return Ok(HandlerResult::empty());
        }

        let movement_fragment = &packet.bytes[(2 + MOVEMENT_HEADER_LEN)..];
        if movement_fragment.is_empty() || movement_fragment[0] == 0 {
            return Ok(HandlerResult::empty());
        }

        let movement_bytes = movement_fragment.to_vec();
        let movement_packet = build_player_move(ctx.client_id, &movement_bytes)?;
        Ok(HandlerResult::empty().with_field_move(movement_packet, movement_bytes))
    }
}
