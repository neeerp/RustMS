use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use packet::Packet;

/// 0xDF, `PARTY_SEARCH_UPDATE` in the reference server. HeavenClient sends it
/// (as `PlayerUpdatePacket`, no payload) after entering a field. Party search
/// is not implemented, so this is a no-op.
pub struct PartySearchUpdateHandler;

impl WorldHandler for PartySearchUpdateHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty())
    }
}
