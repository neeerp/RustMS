use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use packet::Packet;

pub struct PlayerMoveHandler;

impl PlayerMoveHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for PlayerMoveHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        // TODO: Update player position and broadcast to map
        Ok(HandlerResult::empty())
    }
}
