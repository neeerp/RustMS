use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `ITEM_PICKUP`: tick, a byte, the player's position, the drop's object id.
pub struct ItemPickupHandler;

impl WorldHandler for ItemPickupHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?;
        r.read_byte()?;
        let x = r.read_short()?;
        let y = r.read_short()?;
        let object_id = r.read_int()?;
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::Pickup {
                object_id,
                x,
                y,
            })),
        )
    }
}
