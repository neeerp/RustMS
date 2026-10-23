//! ENTRUSTED_SHOP_REQUEST (0x3F): the player used a hired merchant permit.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::hired_merchant::HiredMerchantRequest;
use packet::Packet;

/// The permit's use (0xa27d17), `u8 0` and the permit's cash serial, after
/// the client's own checks (report 165 §5). The runtime finds the permit
/// in the bag itself, and answers with ENTRUSTED_SHOP_CHECK_RESULT.
pub struct EntrustedShopRequestHandler;

impl WorldHandler for EntrustedShopRequestHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::HiredMerchant(HiredMerchantRequest::Check)))
    }
}
