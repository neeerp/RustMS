use crate::combat::{self, AttackKind};
use crate::error::NetworkError;
use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::inventory;
use crate::packet::build::world::mob::{self, FROM_TOUCH};
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::skills;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `MOVE_LIFE`: the controller moved a mob (FUN_0066b6fc). Object id, move
/// id, flags (bit 0: it wants to attack next), the action (action << 1 |
/// facing left, 0xFF for none), skill data, 13 more bytes, the start
/// position, then the movement list.
pub struct MoveLifeHandler;

impl WorldHandler for MoveLifeHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        let move_id = r.read_short()?;
        let flags = r.read_byte()?;
        let action = r.read_byte()?;
        let skill_data = r.read_int()?;
        r.read_bytes(13)?;
        let start = (r.read_short()?, r.read_short()?);
        let movement = packet.bytes[r.position() as usize..].to_vec();
        if movement.first().copied().unwrap_or(0) == 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::MobMove {
                object_id,
                move_id,
                want_attack: flags & 1 != 0,
                action,
                skill_data,
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
        let ranged = attack.kind == AttackKind::Ranged;
        let inflicts = skills::on_hit(ctx.player, uses.skill.as_ref(), ranged, rand::random());
        let mist = uses.skill.as_ref().and_then(|skill| {
            Some(FieldRequest::Mist {
                skill_id: skill.skill_id,
                level: skill.level,
                left: attack.action & 0x8000 != 0,
                inflict: skills::mist(skill)?,
            })
        });
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
        result = result.with(WorldAction::Field(FieldRequest::Attack {
            broadcast,
            hits,
            inflicts,
        }));
        if let Some(mist) = mist {
            result = result.with(WorldAction::Field(mist));
        }
        Ok(result)
    }
}

/// `TAKE_DAMAGE`: the player was hurt by touching a mob or by one of its
/// attacks (FUN_009581a9).
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
        if from < FROM_TOUCH || player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        let mob_id = r.read_int()?;
        let direction = {
            r.read_int()?; // object id
            r.read_byte()?
        };
        let Some(mob) = crate::mobs::info(mob_id).filter(|m| !m.not_attack) else {
            debug!(mob_id, "Damage from a mob that can't attack");
            return Ok(HandlerResult::empty());
        };
        let attack = match usize::try_from(from) {
            Ok(index) => match mob.attacks.get(index) {
                Some(attack) => Some(attack),
                None => {
                    debug!(mob_id, from, "Damage from an attack the mob lacks");
                    return Ok(HandlerResult::empty());
                }
            },
            Err(_) if mob.body_attack => None,
            Err(_) => {
                debug!(mob_id, "Damage from a mob that can't touch");
                return Ok(HandlerResult::empty());
            }
        };
        // The client works out the damage; it can be at most the attack's
        // own, or all but 1 HP for a deadly attack.
        let deadly = attack.is_some_and(|a| a.deadly);
        let most = if deadly {
            i32::from(player.stats.hp) - 1
        } else {
            most_damage(&player.stats, &mob, attack)
        };
        if !(0..=most.max(1)).contains(&damage) {
            debug!(mob_id, from, damage, "Mob damage out of range");
            return Ok(HandlerResult::empty());
        }
        let hp = player.add_hp(-damage);
        let mut updates = vec![StatUpdate::Hp(hp)];
        // As Cosmic does: a deadly attack leaves 1 MP too, and mpBurn burns
        // MP.
        let mp_loss = match attack {
            Some(_) if deadly && damage > 0 => i32::from(player.stats.mp) - 1,
            Some(a) if damage > 0 => a.mp_burn,
            _ => 0,
        };
        if mp_loss > 0 {
            updates.push(StatUpdate::Mp(player.add_mp(-mp_loss)));
        }
        let mut actions = HandlerResult::reply(stat::build_stat_update(false, &updates)?);
        let shown = mob::build_damage_player(player.id(), from, mob_id, damage, direction)?;
        actions = actions.with(WorldAction::Field(FieldRequest::Broadcast(shown)));
        if hp == 0 {
            actions = actions.with(WorldAction::Died);
        }
        Ok(actions)
    }
}

/// The most a mob's touch (`attack` None) or attack can hurt `stats`'
/// owner. A v83 client rolls up to 85% of attack squared over 100, plus a
/// share of any W.DEF shortfall (FUN_0079309f), or 80% for magic
/// (FUN_0079345e).
fn most_damage(
    stats: &game::CharacterStats,
    mob: &game_data::mobs::MobInfo,
    attack: Option<&game_data::mobs::MobAttack>,
) -> i32 {
    use game::mob_damage::{most_magic, most_physical, Defender};
    let defender = Defender {
        level: i32::from(stats.level),
        job: i32::from(stats.job),
        str: i32::from(stats.str),
        dex: i32::from(stats.dex),
        int: i32::from(stats.int),
        luk: i32::from(stats.luk),
    };
    let magic = attack.is_some_and(|a| a.magic);
    let (added, percent) = own_buffs(mob, magic);
    let most = match attack {
        Some(_) if magic => most_magic(mob.magic_attack + added),
        Some(a) if a.attack > 0 => most_physical(&defender, a.attack + added),
        _ => most_physical(&defender, mob.attack + added),
    };
    ((i64::from(most) * i64::from(percent) + 99) / 100) as i32
}

/// The most the mob's own skills can raise its attacks: what Pad (Mad for
/// `magic`) adds to its attack, and the percent PowerUp (MagicUp) makes of
/// the damage, 100 without. The field doesn't tell sessions which statuses
/// a mob has, so a buff from a mob nearby isn't allowed for.
fn own_buffs(mob: &game_data::mobs::MobInfo, magic: bool) -> (i32, i32) {
    use game::mob_skill::{effect, MobSkillEffect};
    use game::MobStat;
    let (add_stat, percent_stat) = if magic {
        (MobStat::Mad, MobStat::MagicUp)
    } else {
        (MobStat::Pad, MobStat::PowerUp)
    };
    let (mut added, mut percent) = (0, 100);
    for entry in &mob.skills {
        let Some(level) = crate::mobs::skill_level(entry.skill, entry.level) else {
            continue;
        };
        let Some(MobSkillEffect::Statuses(statuses)) = effect(entry.skill, level.x) else {
            continue;
        };
        for (stat, value) in statuses {
            if stat == add_stat {
                added = added.max(i32::from(value));
            } else if stat == percent_stat {
                percent = percent.max(i32::from(value));
            }
        }
    }
    (added, percent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn touch(damage: i32, mob_id: i32) -> Packet {
        hit(FROM_TOUCH, damage, mob_id)
    }

    fn hit(from: i8, damage: i32, mob_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(from as u8).unwrap();
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

    fn sample_defender() -> game::mob_damage::Defender {
        // `sample_stats`: a level 10 warrior.
        game::mob_damage::Defender {
            level: 10,
            job: 100,
            str: 35,
            dex: 4,
            int: 4,
            luk: 4,
        }
    }

    #[test]
    fn snails_hurt_up_to_their_bound() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        // A snail's 12 rolls 2 at most, but a level 10 warrior below the
        // expected 54 W.DEF can take more.
        let most = game::mob_damage::most_physical(&sample_defender(), 12);
        assert!(most > 2);
        player.stats.hp = (2 * most) as i16;
        handle(&mut player, touch(most, 100100));
        assert_eq!(i32::from(player.stats.hp), most);
        // More than the bound is refused.
        assert!(handle(&mut player, touch(most + 1, 100100)).is_empty());
        // The tutorial sentinel never attacks.
        assert!(handle(&mut player, touch(1, 9300018)).is_empty());
        let actions = handle(&mut player, touch(most, 100100));
        assert_eq!(player.stats.hp, 0);
        assert!(matches!(
            actions.as_slice(),
            [
                WorldAction::Reply(_),
                WorldAction::Field(FieldRequest::Broadcast(_)),
                WorldAction::Died
            ]
        ));
    }

    #[test]
    fn attacks_hurt_by_their_index() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 1000;
        player.stats.max_hp = 1000;
        // Stumpy (3220000): PADamage 125 for its body, 135 for attack2.
        let defender = sample_defender();
        let body = game::mob_damage::most_physical(&defender, 125);
        let attack2 = game::mob_damage::most_physical(&defender, 135);
        assert!(handle(&mut player, hit(1, attack2 + 1, 3220000)).is_empty());
        handle(&mut player, hit(1, attack2, 3220000));
        assert_eq!(i32::from(player.stats.hp), 1000 - attack2);
        assert!(handle(&mut player, touch(body + 1, 3220000)).is_empty());
        handle(&mut player, touch(body, 3220000));
        assert_eq!(i32::from(player.stats.hp), 1000 - attack2 - body);
        // It has no attack3.
        assert!(handle(&mut player, hit(2, 1, 3220000)).is_empty());
        // A snail has no attacks.
        assert!(handle(&mut player, hit(0, 1, 100100)).is_empty());
    }

    #[test]
    fn a_mob_that_powers_up_may_hurt_more() {
        if crate::mobs::data().is_err() || crate::skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 10_000;
        player.stats.max_hp = 10_000;
        // 4230103 casts Power Up (100), 115% damage.
        let mob = crate::mobs::info(4230103).unwrap();
        let plain = game::mob_damage::most_physical(&sample_defender(), mob.attack);
        let most = (plain * 115 + 99) / 100;
        assert!(most > plain);
        assert!(handle(&mut player, touch(most + 1, 4230103)).is_empty());
        handle(&mut player, touch(most, 4230103));
        assert_eq!(i32::from(player.stats.hp), 10_000 - most);
    }
}
