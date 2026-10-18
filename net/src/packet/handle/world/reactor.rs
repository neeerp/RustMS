use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `DAMAGE_REACTOR`: the reactor's object id, how it was hit (0 a swing, 1
/// a skill), flags (bit 0 facing left, bit 1 on a foothold), the delay
/// before the hit lands in ms, and the skill used. The field checks the
/// player is in reach by its own idea of where they are.
pub struct DamageReactorHandler;

/// Only guild quest reactors take skill hits (kind 1), and they aren't
/// served yet.
const SWING: i32 = 0;

impl WorldHandler for DamageReactorHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let kind = r.read_int()?;
        let flags = r.read_int()?;
        let delay = r.read_short()? as u16;
        r.read_int()?; // skill
        if kind != SWING || ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::HitReactor {
                object_id,
                flags,
                delay,
            })),
        )
    }
}
