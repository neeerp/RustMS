//! The Owl of Minerva's windows (report c024). The runtime searches the
//! shops and answers.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::owl::OwlRequest;
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
        let item_id = r.read_int()?;
        let descending = r.read_byte()? != 0;
        Ok(
            HandlerResult::empty().with(WorldAction::Owl(OwlRequest::Search {
                slot,
                owl_id,
                item_id,
                descending,
            })),
        )
    }
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
