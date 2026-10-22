//! The Owl of Minerva's windows (report c024). The runtime searches the
//! shops and answers.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::owl::OwlRequest;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// OWL_OPEN (0x42, FUN_008a0e9a): the search window opened and asks for
/// the most searched items with a 5.
pub struct OwlOpenHandler;

impl WorldHandler for OwlOpenHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::Owl(OwlRequest::Popular)))
    }
}

/// USE_OWL (0x53, FUN_00a0a25e and FUN_008a2407): the Owl's USE slot and
/// id, the item to look for, the order (0 cheapest first), and the tick.
pub struct UseOwlHandler;

impl WorldHandler for UseOwlHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let slot = r.read_short()?;
        let owl_id = r.read_int()?;
        owl_search(&mut r, InventoryType::Use, slot, owl_id)
    }
}

/// USE_CASH_ITEM (0x4F) type 0x1d, the Cash Owl (FUN_00a0a63f then
/// FUN_008a2407): after its Cash slot and id, the same fields as 0x53.
pub fn cash_owl(r: &mut Cursor<&Vec<u8>>, slot: i16, owl_id: i32) -> WorldResult {
    owl_search(r, InventoryType::Cash, slot, owl_id)
}

/// The item to look for and the order (0 cheapest first); the tick is
/// left.
fn owl_search(
    r: &mut Cursor<&Vec<u8>>,
    inventory: InventoryType,
    slot: i16,
    owl_id: i32,
) -> WorldResult {
    let item_id = r.read_int()?;
    let descending = r.read_byte()? != 0;
    Ok(
        HandlerResult::empty().with(WorldAction::Owl(OwlRequest::Search {
            inventory,
            slot,
            owl_id,
            item_id,
            descending,
        })),
    )
}

/// OWL_GO (0x43, FUN_008a4423): GO on a row, which gives back the shop and
/// its map as the results named them.
pub struct OwlGoHandler;

impl WorldHandler for OwlGoHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let room = r.read_int()?;
        let map_id = r.read_int()?;
        Ok(HandlerResult::empty().with(WorldAction::Owl(OwlRequest::Go { room, map_id })))
    }
}
