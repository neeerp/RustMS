use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use packet::Packet;

/// PLAYER_UPDATE (0xDF): the client's periodic "please save me" nudge.
/// Players are saved by the runtime on its own schedule, so this is a no-op.
pub struct PlayerUpdateHandler;

impl WorldHandler for PlayerUpdateHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty())
    }
}
