use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use packet::Packet;

/// 0xCF: sent by the client after map transfers; nothing to do yet.
pub struct PlayerMapTransferHandler;

impl WorldHandler for PlayerMapTransferHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty())
    }
}
