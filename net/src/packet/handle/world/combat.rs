use crate::combat::{self, AttackKind};
use crate::error::NetworkError;
use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::inventory;
use crate::packet::build::world::mob::{self, FROM_TOUCH};
use crate::packet::build::world::stat::{self, StatUpdate};
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `MOVE_LIFE`: the controller moved a mob. Object id, move id, six bytes of
/// skill data, 13 more bytes, the start position, then the movement list.
pub struct MoveLifeHandler;

impl WorldHandler for MoveLifeHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let move_id = r.read_short()?;
        r.read_bytes(6 + 13)?;
        let start = (r.read_short()?, r.read_short()?);
        let movement = packet.bytes[r.position() as usize..].to_vec();
        if movement.first().copied().unwrap_or(0) == 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::MobMove {
                object_id,
                move_id,
                start,
                movement,
            })),
        )
    }
}

/// `MOB_DISTANCE`: once a second, every client says how far its player is
/// from each firstAttack mob. Object id, then the distance.
pub struct MobDistanceHandler;

impl WorldHandler for MobDistanceHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let distance = r.read_int()?;
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::MobDistance {
                object_id,
                distance,
            })),
        )
    }
}

/// `NPC_ACTION`: the NPCs' controller chose an idle action. Object id, the
/// action (an animation index from 2, or -1 for a line alone), the line
/// (-1 for none), then a path for an NPC that walks.
pub struct NpcActionHandler;

impl WorldHandler for NpcActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let action = packet.bytes[r.position() as usize..].to_vec();
        if action.len() < 2 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::NpcAction {
                object_id,
                action,
            })),
        )
    }
}

/// `CLOSE_RANGE_ATTACK`, `RANGED_ATTACK`, and `MAGIC_ATTACK`: a swing, a
/// shot, or a spell.
pub struct AttackHandler(pub AttackKind);

impl WorldHandler for AttackHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let attack = combat::parse_attack(packet, self.0)?;
        let uses = match combat::validate(&attack, ctx.player) {
            Ok(uses) => uses,
            Err(rejection) => {
                debug!(?rejection, skill = attack.skill, "Attack refused");
                return Ok(HandlerResult::empty());
            }
        };
        let mut result = HandlerResult::empty();
        let mut skill_level = 0;
        if let Some(skill) = uses.skill {
            let mp = ctx.player.add_mp(-combat::attack_mp_cost(&skill));
            result = HandlerResult::reply(stat::build_stat_update(false, &[StatUpdate::Mp(mp)])?);
            skill_level = skill.level as u8;
        }
        let mut projectile = 0;
        if let Some(ammo) = uses.ammo {
            let changes = ctx
                .player
                .inventory_mut()
                .take(InventoryType::Use, ammo.slot, ammo.quantity)
                .map_err(|_| NetworkError::PacketHandlerError("ammo vanished"))?;
            result = result.with_reply(inventory::build_inventory_operation(false, &changes)?);
            projectile = ammo.item_id;
        }
        let broadcast = combat::build_attack(ctx.player.id(), &attack, skill_level, projectile)?;
        let hits = attack
            .hits
            .into_iter()
            .map(|hit| {
                (
                    hit.object_id,
                    hit.damage.iter().map(|d| i64::from(*d)).sum(),
                )
            })
            .collect();
        Ok(result.with(WorldAction::Field(FieldRequest::Attack { broadcast, hits })))
    }
}

/// `TAKE_DAMAGE`: the player was hurt (here: by touching a mob).
pub struct TakeDamageHandler;

impl WorldHandler for TakeDamageHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?; // tick
        let from = r.read_byte()? as i8;
        r.read_byte()?; // element
        let damage = r.read_int()?;
        let player = &mut *ctx.player;
        if from != FROM_TOUCH || player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        let mob_id = r.read_int()?;
        let direction = {
            r.read_int()?; // object id
            r.read_byte()?
        };
        // A mob can hurt at most its attack; the client applies defense.
        let Some(mob) = crate::mobs::info(mob_id).filter(|m| m.body_attack && !m.not_attack) else {
            debug!(mob_id, "Damage from a mob that can't touch");
            return Ok(HandlerResult::empty());
        };
        if !(0..=mob.attack).contains(&damage) {
            debug!(mob_id, damage, "Touch damage out of range");
            return Ok(HandlerResult::empty());
        }
        let hp = player.add_hp(-damage);
        let mut actions =
            HandlerResult::reply(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?);
        let shown = mob::build_damage_player(player.id(), mob_id, damage, direction)?;
        actions = actions.with(WorldAction::Field(FieldRequest::Broadcast(shown)));
        if hp == 0 {
            actions = actions.with(WorldAction::Died);
        }
        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn touch(damage: i32, mob_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(FROM_TOUCH as u8).unwrap();
        p.write_byte(0).unwrap();
        p.write_int(damage).unwrap();
        p.write_int(mob_id).unwrap();
        p.write_int(2_000_000_001).unwrap();
        p.write_byte(1).unwrap();
        p
    }

    fn handle(player: &mut game::Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        TakeDamageHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn snails_hurt_up_to_their_attack() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 20;
        let actions = handle(&mut player, touch(12, 100100));
        assert_eq!(player.stats.hp, 8);
        assert!(matches!(
            actions.as_slice(),
            [
                WorldAction::Reply(_),
                WorldAction::Field(FieldRequest::Broadcast(_))
            ]
        ));
        // More than a snail's attack is refused.
        assert!(handle(&mut player, touch(13, 100100)).is_empty());
        // The tutorial sentinel never attacks.
        assert!(handle(&mut player, touch(1, 9300018)).is_empty());
        let actions = handle(&mut player, touch(12, 100100));
        assert_eq!(player.stats.hp, 0);
        assert!(matches!(actions.last(), Some(WorldAction::Died)));
    }
}
