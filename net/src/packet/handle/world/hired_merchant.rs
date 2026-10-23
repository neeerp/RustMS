//! ENTRUSTED_SHOP_REQUEST (0x3F): the player used a hired merchant permit.
//! STORE_BANK_REQUEST (0x40): the window of Fredrick's Store Bank.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::hired_merchant::HiredMerchantRequest;
use crate::store_bank::StoreBankRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// The permit's use (0xa27d17), `u8 0` and the permit's cash serial, after
/// the client's own checks (report 165 §5). The runtime finds the permit
/// in the bag itself, and answers with ENTRUSTED_SHOP_CHECK_RESULT.
pub struct EntrustedShopRequestHandler;

impl WorldHandler for EntrustedShopRequestHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::HiredMerchant(HiredMerchantRequest::Check)))
    }
}

/// The Store Bank's `u8` mode: Get All, its confirm, or the window
/// closing. The runtime answers.
pub struct StoreBankRequestHandler;

impl WorldHandler for StoreBankRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let mode = reader.read_byte()?;
        let Some(request) = StoreBankRequest::from_mode(mode) else {
            debug!(mode, "Ignoring Store Bank request");
            return Ok(HandlerResult::empty());
        };
        Ok(HandlerResult::empty().with(WorldAction::StoreBank(request)))
    }
}
