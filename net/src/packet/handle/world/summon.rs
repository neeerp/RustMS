//! What the owner's client sends about their summons: it moves them,
//! attacks with them, and reports a Puppet hurt. The field checks the
//! summon is theirs.

use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::summons;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// MOVE_SUMMON (0xAF, FUN_009c84e9): the summon's object id, then a path
/// as MOVE_PLAYER's: where it starts, then its elements.
pub struct MoveSummonHandler;

impl WorldHandler for MoveSummonHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let start = (r.read_short()?, r.read_short()?);
        let movement = packet.bytes[r.position() as usize..].to_vec();
        if movement.first().is_none_or(|&count| count == 0) {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::SummonMove {
                object_id,
                start,
                movement,
            })),
        )
    }
}

/// SUMMON_ATTACK (0xB0): one hit on each mob, held to the limits of the
/// summons the player has up.
pub struct SummonAttackHandler;

impl WorldHandler for SummonAttackHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let attack = summons::parse_attack(packet)?;
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        let limits = summons::limits(ctx.player);
        if limits.is_empty() {
            debug!(
                object_id = attack.object_id,
                "Summon attack without a summon"
            );
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::SummonAttack {
                attack,
                limits,
            })),
        )
    }
}

/// DAMAGE_SUMMON (0xB1): a mob hurt the player's Puppet.
pub struct DamageSummonHandler;

impl WorldHandler for DamageSummonHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let damage = summons::parse_damage(packet)?;
        if damage.damage < 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::SummonDamage(damage))))
    }
}
