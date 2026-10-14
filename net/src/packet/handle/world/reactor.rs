use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `DAMAGE_REACTOR`: the reactor's object id, the player's position, their
/// stance, 4 unused bytes, and the skill used (0 for a plain attack). The
/// field checks the player is in reach by its own idea of where they are.
pub struct DamageReactorHandler;

impl WorldHandler for DamageReactorHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        r.read_short()?;
        r.read_short()?;
        let stance = r.read_short()?;
        r.read_int()?;
        let skill_id = r.read_int()?;
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::HitReactor {
                object_id,
                skill_id,
                stance,
            })),
        )
    }
}
