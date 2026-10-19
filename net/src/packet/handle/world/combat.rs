use crate::combat::{self, AttackKind};
use crate::error::NetworkError;
use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::mob::{self, FROM_MIST, FROM_TOUCH};
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::packet::build::world::{buff, effect, inventory};
use crate::skills;
use game::buff::BuffStat;
use game::{Buff, Disease, InventoryType, MobStat, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::{Duration, Instant};
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

/// `MOB_BOMB`: a mob whose `selfDestruction` action has bit 2 (the
/// Darkstars) has the player in reach of an attack (FUN_0066e636). The
/// object id alone. A dead player's client sends none.
pub struct MobBombHandler;

impl WorldHandler for MobBombHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let object_id = r.read_int()?;
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::MobBomb { object_id })))
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
        // A skill with a cooltime, as Snipe, can't attack again until it is
        // over, and the client hears how long.
        let now = Instant::now();
        let cooldown = uses.skill.as_ref().map_or(0, |skill| skill.effect.cooldown);
        if cooldown > 0 {
            if ctx.player.buffs.cooling_down(attack.skill, now) {
                debug!(skill = attack.skill, "Attack refused while cooling down");
                return Ok(HandlerResult::empty());
            }
            let length = Duration::from_secs(cooldown as u64);
            ctx.player.buffs.start_cooldown(attack.skill, now, length);
        }
        if let Some(skill) = uses.skill {
            let mut updates = vec![StatUpdate::Mp(
                ctx.player.add_mp(-combat::attack_mp_cost(&skill)),
            )];
            if uses.mesos > 0 {
                ctx.player.meso -= uses.mesos;
                ctx.player.mark_dirty();
                updates.push(StatUpdate::Meso(ctx.player.meso));
            }
            result = HandlerResult::reply(stat::build_stat_update(false, &updates)?);
            if cooldown > 0 {
                result = result.with_reply(buff::build_cooldown(skill.skill_id, cooldown as i16)?);
            }
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
        let skill = attack.skill;
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
        let counter = if attack.kind == AttackKind::Magic {
            MobStat::MCounter
        } else {
            MobStat::PCounter
        };
        result = result.with(WorldAction::Field(FieldRequest::Attack {
            broadcast,
            skill,
            counter,
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
/// attacks (FUN_009581a9), or by a mob's mist.
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
        if from == FROM_MIST && player.stats.hp > 0 {
            let code = r.read_short()? as u16;
            let kind = r.read_byte()?;
            return mist_damage(player, code, kind, damage);
        }
        if from < FROM_TOUCH || player.stats.hp <= 0 {
            return Ok(HandlerResult::empty());
        }
        let mob_id = r.read_int()?;
        let object_id = r.read_int()?;
        let left = r.read_byte()?;
        let tail = HitTail::read(&packet.bytes[r.position() as usize..]);
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
        // own, or all but 1 HP for a deadly attack. -1 is a dodge.
        let deadly = attack.is_some_and(|a| a.deadly);
        let most = if deadly {
            i32::from(player.stats.hp) - 1
        } else {
            most_damage(&player.stats, &mob, attack)
        };
        let dodged_by = if damage == -1 {
            dodged_by(player, tail.guard)
        } else {
            None
        };
        if dodged_by.is_none() && !(0..=most.max(1)).contains(&damage) {
            debug!(mob_id, from, damage, "Mob damage out of range");
            return Ok(HandlerResult::empty());
        }
        // A touch under Power Guard, or a spell under Mana Reflection whose
        // chance came up, sends some back at the mob, if the client says so
        // (0x9587c6, 0x958d50). Only Power Guard's spares the player.
        let (reflected, spared) = match (tail.shown, attack) {
            (Some([1, ..]), None) if tail.reflect != 0 => {
                let reflected = combat::power_guard(player, &mob, damage);
                (reflected, reflected)
            }
            (Some([0, ..]), Some(attack)) if attack.magic && tail.reflect != 0 => {
                (combat::mana_reflection(player, &mob, damage), 0)
            }
            _ => (0, 0),
        };
        // As Cosmic does, Magic Guard leaves alone an attack that leaves 1
        // MP or burns MP.
        let mp_burn = attack.map_or(0, |a| a.mp_burn);
        let taken = combat::taken(player, damage, !deadly && mp_burn == 0, true).less(spared);
        let hp = player.add_hp(-taken.hp);
        let mut updates = vec![StatUpdate::Hp(hp)];
        let mp_loss = match attack {
            Some(_) if deadly && damage > 0 => i32::from(player.stats.mp) - 1,
            Some(_) if damage > 0 && mp_burn > 0 => mp_burn,
            _ => taken.mp,
        };
        if mp_loss > 0 {
            updates.push(StatUpdate::Mp(player.add_mp(-mp_loss)));
        }
        if taken.mesos > 0 {
            player.meso -= taken.mesos;
            player.mark_dirty();
            updates.push(StatUpdate::Meso(player.meso));
        }
        let mut actions = HandlerResult::reply(stat::build_stat_update(false, &updates)?);
        let hurt = mob::Hurt {
            from,
            damage,
            mob_id,
            left,
            reflect: tail
                .shown
                .filter(|_| tail.reflect != 0)
                .map(|shown| (tail.reflect, shown)),
            stance: tail.stance,
            shown: taken.shown,
            dodged_by,
        };
        let shown = mob::build_damage_player(player.id(), &hurt)?;
        actions = actions.with(WorldAction::Field(FieldRequest::Broadcast(shown)));
        if reflected > 0 {
            actions = actions.with(WorldAction::Field(FieldRequest::Reflect {
                object_id,
                mob_id,
                damage: reflected.into(),
            }));
        }
        if hp == 0 {
            return Ok(actions.with(WorldAction::Died));
        }
        // Some attacks bring a disease (`disease`, `level`), taking hold
        // by its `prop` (Cosmic's TakeDamageHandler).
        let disease = attack.filter(|_| damage > 0).and_then(|attack| {
            let level = crate::mobs::skill_level(attack.disease, attack.disease_level)?;
            let disease = game::mob_skill::inflicted(
                attack.disease,
                attack.disease_level,
                level.x,
                level.duration,
            )?;
            (rand::random::<f64>() * 100.0 < f64::from(level.prop)).then_some(disease)
        });
        if let Some(disease) = disease {
            let caught = catch_disease(player, disease, std::time::Instant::now())?;
            actions.actions.extend(caught.actions);
        }
        Ok(actions)
    }
}

/// What a v83 client sends after a mob's hit (0x9594d0): the Power Guard
/// or Mana Reflection percent, the Guardian byte (1 an attack, 2 a touch
/// blocked), then, if either is set, 14 bytes (Power Guard or not, the
/// mob's object id and hit action, its position and the player's), and
/// whether Power Stance kept the player in place. A ridden Guardian can
/// send the 14 bytes with both unset, so they are told by the length.
/// Older clients send none of it.
#[derive(Debug, Default, PartialEq, Eq)]
struct HitTail {
    reflect: u8,
    guard: u8,
    /// The 14 bytes without the player's position, as the others read them.
    shown: Option<[u8; 10]>,
    stance: bool,
}

impl HitTail {
    fn read(rest: &[u8]) -> HitTail {
        match *rest {
            [reflect, guard, ref block @ .., stance] if block.len() == 14 => HitTail {
                reflect,
                guard,
                shown: block[..10].try_into().ok(),
                stance: stance != 0,
            },
            [reflect, guard, stance] => HitTail {
                reflect,
                guard,
                shown: None,
                stance: stance != 0,
            },
            _ => HitTail::default(),
        }
    }
}

/// The skill a v83 client dodged a hit with when it sends -1: Guardian for
/// a Hero or Paladin with a shield, when it sets the Guardian byte, or
/// else Shadow Shifter for a Night Lord or Shadower (FUN_009581a9, 0x958692
/// and 0x9584f6). None if the player has no such skill.
fn dodged_by(player: &Player, guard: u8) -> Option<i32> {
    let skill_id = match (player.stats.job, guard != 0) {
        (112, true) => 1120005,
        (122, true) => 1220006,
        (412, false) => 4120002,
        (422, false) => 4220002,
        _ => return None,
    };
    let shield = || {
        player
            .inventory
            .get(InventoryType::Equipped, game::inventory::equip_slot::SHIELD)
            .is_some()
    };
    (player.skills.level(skill_id) > 0 && (guard == 0 || shield())).then_some(skill_id)
}

// What the last byte of a v83 client's report with no mob says hurt the
// player (0x94b14a, 0x94b8d2, 0x94b28b).
const POISONED: u8 = 1;
const IN_MIST: u8 = 2;
const STOPPED: u8 = 3;

/// A v83 client reports hurting itself without a mob: in mob skill
/// `skill`'s mist (`code`: skill << 8 | level) it takes the skill's `x` a
/// second (FUN_00431783), less any resistance, and Achilles takes its share
/// off; standing still two seconds under Stop Motion, it takes its value
/// but never the last HP (0x94b1d7), with no code. Its own Poison ticks too,
/// but the server ticks Poison itself, so that report is ignored.
fn mist_damage(player: &mut Player, code: u16, kind: u8, damage: i32) -> WorldResult {
    let (skill, level) = (i32::from(code >> 8), i32::from(code & 0xff));
    let (most, achilles) = match kind {
        STOPPED => match player.buffs.value(BuffStat::StopMotion) {
            Some(x) if code == 0 => (i32::from(x).min(i32::from(player.stats.hp) - 1), false),
            _ => {
                debug!("Stop Motion damage without Stop Motion");
                return Ok(HandlerResult::empty());
            }
        },
        IN_MIST => match crate::mobs::skill_level(skill, level) {
            Some(level) if skill == game::mob_skill::POISON_MIST => (level.x, true),
            _ => {
                debug!(skill, level, "Mist damage from a skill that leaves none");
                return Ok(HandlerResult::empty());
            }
        },
        POISONED => return Ok(HandlerResult::empty()),
        _ => {
            debug!(kind, "Damage from something unknown");
            return Ok(HandlerResult::empty());
        }
    };
    if !(0..=most).contains(&damage) {
        debug!(skill, level, damage, "Mist damage out of range");
        return Ok(HandlerResult::empty());
    }
    let taken = if achilles {
        combat::taken(player, damage, false, false)
    } else {
        combat::Taken {
            hp: damage,
            shown: damage,
            ..combat::Taken::default()
        }
    };
    let hp = player.add_hp(-taken.hp);
    let hurt = mob::Hurt {
        from: FROM_MIST,
        damage,
        shown: taken.shown,
        ..mob::Hurt::default()
    };
    let shown = mob::build_damage_player(player.id(), &hurt)?;
    let actions = HandlerResult::reply(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?)
        .with(WorldAction::Field(FieldRequest::Broadcast(shown)));
    Ok(if hp == 0 {
        actions.with(WorldAction::Died)
    } else {
        actions
    })
}

/// Mobs the player attacked countered them for `damage`: a v83 client
/// shows its own roll and sends nothing (FUN_00953595), and nobody else
/// sees it. Achilles and Magic Guard take their shares as for any hit.
pub fn countered(player: &mut Player, damage: i32) -> WorldResult {
    if player.stats.hp <= 0 {
        return Ok(HandlerResult::empty());
    }
    let taken = combat::taken(player, damage, true, false);
    let hp = player.add_hp(-taken.hp);
    let mut updates = vec![StatUpdate::Hp(hp)];
    if taken.mp > 0 {
        updates.push(StatUpdate::Mp(player.add_mp(-taken.mp)));
    }
    let actions = HandlerResult::reply(stat::build_stat_update(false, &updates)?);
    Ok(if hp == 0 {
        actions.with(WorldAction::Died)
    } else {
        actions
    })
}

/// The map hurts the player every so often (its `decHP`, which the v83
/// client leaves to the server): nothing while they're dead or wear its
/// `protectItem`, or, in the cold, an equip scrolled against it. A thaw item
/// for its kind of hazard keeps that much off each hit, as Red Bean Porridge
/// (2022001) says.
pub fn hurt_by_hazard(player: &mut Player, hazard: &game_data::Hazard) -> WorldResult {
    let protected = hazard
        .protect_item
        .is_some_and(|id| player.inventory.equipped_ids().any(|(_, item)| item == id))
        || hazard.cold
            && (player.inventory.items(InventoryType::Equipped))
                .any(|(_, item)| item.flags & game::scroll::FLAG_COLD != 0);
    let thaw = player.buffs.value(BuffStat::Thaw).map_or(0, i32::from);
    let kept = if (thaw < 0) == hazard.water {
        thaw.abs()
    } else {
        0
    };
    if player.stats.hp <= 0 || protected || hazard.hp <= kept {
        return Ok(HandlerResult::empty());
    }
    let hp = player.add_hp(kept - hazard.hp);
    let actions = HandlerResult::reply(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?);
    Ok(if hp == 0 {
        actions.with(WorldAction::Died)
    } else {
        actions
    })
}

/// A mob gave the player a disease: unless they're dead or already have it,
/// it starts, and the others see it if it shows.
pub fn catch_disease(
    player: &mut Player,
    disease: Disease,
    now: std::time::Instant,
) -> WorldResult {
    let stat = disease.stat;
    if player.stats.hp <= 0 || player.buffs.has(stat) {
        return Ok(HandlerResult::empty());
    }
    player.buffs.give(Buff::new(
        disease.skill_id,
        stat,
        disease.value,
        now,
        disease.lasts,
    ));
    let lasts = i32::try_from(disease.lasts.as_millis()).unwrap_or(i32::MAX);
    let mut result = HandlerResult::reply(buff::build_give_buff(
        disease.skill_id,
        lasts,
        &[(stat, disease.value)],
    )?);
    let shown: Vec<_> = player
        .buffs
        .shown()
        .into_iter()
        .filter(|shown| shown.stat == stat)
        .collect();
    if !shown.is_empty() {
        result = result.with(WorldAction::BuffsChanged(buff::build_give_foreign_buff(
            player.id(),
            &shown,
        )?));
    }
    Ok(result)
}

const SAFETY_CHARM: i32 = 5130000;

/// The items that keep a dying player's EXP, in the order they're used
/// (Cosmic): the Safety Charm, the Easter Basket and the Easter Charm.
const CHARMS: [i32; 3] = [SAFETY_CHARM, 4031283, 4140903];

/// The player died. Beginners lose nothing; others lose 10% of the EXP
/// their level needs (v83's field death penalty), unless they hold a charm,
/// which is used up instead and lets them revive with 30% of their max HP
/// and MP (Cosmic).
pub fn death_penalty(player: &mut Player) -> Result<Vec<Packet>, NetworkError> {
    if player.stats.job % 1000 == 0 {
        return Ok(Vec::new());
    }
    let Some(needed) = game::exp::to_next_level(player.stats.level) else {
        return Ok(Vec::new());
    };
    if let Some(charm) = CHARMS
        .into_iter()
        .find(|&item_id| player.inventory.count(item_id) > 0)
    {
        let changes = player
            .inventory_mut()
            .remove(charm, 1)
            .map_err(|_| NetworkError::PacketHandlerError("Charm not found"))?;
        player.charmed = true;
        let used = if charm == SAFETY_CHARM {
            let next = player
                .inventory
                .items(InventoryType::Cash)
                .find(|(_, item)| item.item_id == charm);
            let days = next
                .and_then(|(_, item)| item.expiration)
                .and_then(|at| at.duration_since(std::time::SystemTime::now()).ok())
                .map_or(0, |left| left.as_secs().div_ceil(24 * 60 * 60));
            effect::CharmUsed::SafetyCharm {
                left: u8::try_from(player.inventory.count(charm)).unwrap_or(u8::MAX),
                days: u8::try_from(days).unwrap_or(u8::MAX),
            }
        } else {
            effect::CharmUsed::Item(charm)
        };
        return Ok(vec![
            inventory::build_inventory_operation(true, &changes)?,
            effect::build_charm_used(used)?,
        ]);
    }
    let loss = (needed / 10) as i32;
    player.stats.exp = (player.stats.exp - loss).max(0);
    player.mark_dirty();
    Ok(vec![stat::build_stat_update(
        false,
        &[StatUpdate::Exp(player.stats.exp)],
    )?])
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

    /// A v83 client hurt by mob skill `skill` at `level`'s mist.
    fn mist(damage: i32, skill: u8, level: u8) -> Packet {
        no_mob(damage, i16::from(skill) << 8 | i16::from(level), IN_MIST)
    }

    /// A v83 client hurt by Stop Motion.
    fn stopped(damage: i32) -> Packet {
        no_mob(damage, 0, STOPPED)
    }

    fn no_mob(damage: i32, code: i16, kind: u8) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_byte(FROM_MIST as u8).unwrap();
        p.write_byte(0).unwrap();
        p.write_int(damage).unwrap();
        p.write_short(code).unwrap();
        p.write_byte(kind).unwrap();
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
    fn dying_costs_a_tenth_of_the_level_unless_a_charm_is_held() {
        let mut player = crate::codec::stats::sample_player();
        let needed = game::exp::to_next_level(player.stats.level).unwrap() as i32;
        player.stats.exp = needed / 2;
        assert_eq!(death_penalty(&mut player).unwrap().len(), 1);
        assert_eq!(player.stats.exp, needed / 2 - needed / 10);
        assert!(!player.charmed);

        player
            .inventory
            .insert(InventoryType::Cash, 1, game::Item::stack(SAFETY_CHARM, 2));
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(4140903, 1));
        let exp = player.stats.exp;
        let packets = death_penalty(&mut player).unwrap();
        assert_eq!(player.stats.exp, exp);
        assert!(player.charmed);
        assert_eq!(player.inventory.count(SAFETY_CHARM), 1);
        // One charm left, which never expires.
        assert_eq!(&packets[1].bytes[2..], &[6, 1, 1, 0]);

        // The Safety Charm goes first; another charm is named.
        death_penalty(&mut player).unwrap();
        let packets = death_penalty(&mut player).unwrap();
        assert_eq!(player.inventory.count(4140903), 0);
        assert_eq!(&packets[1].bytes[2..6], &[6, 0, 0, 0]);
        assert_eq!(player.stats.exp, exp);
    }

    #[test]
    fn hazards_hurt_unless_their_item_is_worn() {
        let cold = game_data::Hazard {
            hp: 10,
            every: game_data::HAZARD_INTERVAL,
            protect_item: Some(1102109),
            water: false,
            cold: true,
        };
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        let actions = hurt_by_hazard(&mut player, &cold).unwrap().actions;
        assert!(matches!(actions.as_slice(), [WorldAction::Reply(_)]));
        assert_eq!(player.stats.hp, 90);

        player.inventory.insert(
            InventoryType::Equipped,
            -9,
            game::Item::equipment(1102109, game::EquipStats::default()),
        );
        assert!(hurt_by_hazard(&mut player, &cold)
            .unwrap()
            .actions
            .is_empty());
        assert_eq!(player.stats.hp, 90);

        let sharenian = game_data::Hazard {
            hp: 30000,
            every: game_data::HAZARD_INTERVAL,
            protect_item: Some(1032033),
            water: false,
            cold: false,
        };
        let actions = hurt_by_hazard(&mut player, &sharenian).unwrap().actions;
        assert_eq!(player.stats.hp, 0);
        assert!(matches!(actions.last(), Some(WorldAction::Died)));
        assert!(hurt_by_hazard(&mut player, &sharenian)
            .unwrap()
            .actions
            .is_empty());
    }

    #[test]
    fn thaw_keeps_its_share_of_its_own_hazard_off() {
        let seabed = game_data::Hazard {
            hp: 6,
            every: game_data::HAZARD_INTERVAL,
            protect_item: Some(1102061),
            water: true,
            cold: false,
        };
        let cold = game_data::Hazard {
            water: false,
            cold: true,
            hp: 10,
            ..seabed
        };
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        let now = Instant::now();
        let lasts = Duration::from_secs(900);
        // Air Bubble (-6) is for the water, not the cold.
        player
            .buffs
            .give(Buff::new(-2022040, BuffStat::Thaw, -6, now, lasts));
        assert!(hurt_by_hazard(&mut player, &seabed)
            .unwrap()
            .actions
            .is_empty());
        hurt_by_hazard(&mut player, &cold).unwrap();
        assert_eq!(player.stats.hp, 90);
        // Red Bean Porridge (10) is for the cold, and keeps 10 of 30 off.
        player
            .buffs
            .give(Buff::new(-2022001, BuffStat::Thaw, 10, now, lasts));
        assert!(hurt_by_hazard(&mut player, &cold)
            .unwrap()
            .actions
            .is_empty());
        hurt_by_hazard(&mut player, &seabed).unwrap();
        assert_eq!(player.stats.hp, 84);
        hurt_by_hazard(&mut player, &game_data::Hazard { hp: 30, ..cold }).unwrap();
        assert_eq!(player.stats.hp, 64);
    }

    #[test]
    fn equips_scrolled_against_the_cold_keep_only_the_cold_off() {
        let cold = game_data::Hazard {
            hp: 10,
            every: game_data::HAZARD_INTERVAL,
            protect_item: None,
            water: false,
            cold: true,
        };
        let other_dimension = game_data::Hazard {
            cold: false,
            ..cold
        };
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        let mut glove = game::Item::equipment(1082002, game::EquipStats::default());
        glove.flags = game::scroll::FLAG_COLD;
        player.inventory.insert(InventoryType::Equipped, -8, glove);
        assert!(hurt_by_hazard(&mut player, &cold)
            .unwrap()
            .actions
            .is_empty());
        hurt_by_hazard(&mut player, &other_dimension).unwrap();
        assert_eq!(player.stats.hp, 90);
    }

    #[test]
    fn beginners_keep_their_charms() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 0;
        player
            .inventory
            .insert(InventoryType::Cash, 1, game::Item::stack(SAFETY_CHARM, 1));
        assert!(death_penalty(&mut player).unwrap().is_empty());
        assert_eq!(player.inventory.count(SAFETY_CHARM), 1);
    }

    #[test]
    fn a_poison_mist_hurts_up_to_its_x() {
        let Some(level) = crate::mobs::skill_level(131, 2) else {
            return;
        };
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = (2 * level.x) as i16;
        let actions = handle(&mut player, mist(level.x, 131, 2));
        assert_eq!(i32::from(player.stats.hp), level.x);
        assert!(matches!(
            actions.as_slice(),
            [
                WorldAction::Reply(_),
                WorldAction::Field(FieldRequest::Broadcast(_))
            ]
        ));
        // More than its x, or a skill that leaves no mist, is refused.
        assert!(handle(&mut player, mist(level.x + 1, 131, 2)).is_empty());
        assert!(handle(&mut player, mist(1, 125, 1)).is_empty());
        let actions = handle(&mut player, mist(level.x, 131, 2));
        assert_eq!(player.stats.hp, 0);
        assert!(matches!(actions.last(), Some(WorldAction::Died)));
    }

    #[test]
    fn stop_motion_hurts_up_to_its_value_but_never_kills() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 500;
        // Without it, a report with no skill is refused.
        assert!(handle(&mut player, stopped(330)).is_empty());

        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(20);
        let stop_motion = 135 | 1 << 16;
        player.buffs.give(Buff::new(
            stop_motion,
            BuffStat::StopMotion,
            330,
            now,
            lasts,
        ));
        assert!(handle(&mut player, stopped(331)).is_empty());
        assert_eq!(handle(&mut player, stopped(330)).len(), 2);
        assert_eq!(player.stats.hp, 170);
        assert!(handle(&mut player, stopped(170)).is_empty());
        assert_eq!(handle(&mut player, stopped(169)).len(), 2);
        assert_eq!(player.stats.hp, 1);
        // A mist report with no skill isn't Stop Motion's, and the
        // client's own Poison ticks are left to the server's.
        player.stats.hp = 500;
        assert!(handle(&mut player, mist(10, 0, 0)).is_empty());
        assert!(handle(&mut player, no_mob(10, 0, POISONED)).is_empty());
        assert_eq!(player.stats.hp, 500);
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

    /// A touch as a v83 client sends it, with what follows the mob.
    fn touch_with(damage: i32, mob_id: i32, tail: &[u8]) -> Packet {
        let mut p = touch(damage, mob_id);
        p.write_bytes(tail).unwrap();
        p
    }

    #[test]
    fn reads_what_follows_the_mob_by_its_length() {
        assert_eq!(HitTail::read(&[]), HitTail::default());
        assert_eq!(
            HitTail::read(&[0, 1, 1]),
            HitTail {
                reflect: 0,
                guard: 1,
                shown: None,
                stance: true
            }
        );
        let mut rest = vec![30, 0];
        rest.extend(1..=14);
        rest.push(0);
        assert_eq!(
            HitTail::read(&rest),
            HitTail {
                reflect: 30,
                guard: 0,
                shown: Some([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
                stance: false
            }
        );
    }

    #[test]
    fn a_dodge_needs_the_skill_and_costs_nothing() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        assert!(handle(&mut player, touch_with(-1, 100100, &[0, 0, 0])).is_empty());
        let learn = |player: &mut game::Player, skill_id| {
            player.skills.set(game::SkillRecord {
                skill_id,
                level: 1,
                master_level: 30,
                expiration: None,
            })
        };
        let dodged_with = |actions: &[WorldAction]| match actions {
            [WorldAction::Reply(_), WorldAction::Field(FieldRequest::Broadcast(shown))] => {
                let skill = &shown.bytes[shown.bytes.len() - 4..];
                i32::from_le_bytes(skill.try_into().unwrap())
            }
            _ => panic!("not a dodge: {actions:?}"),
        };
        player.stats.job = 412;
        learn(&mut player, 4120002);
        let actions = handle(&mut player, touch_with(-1, 100100, &[0, 0, 0]));
        assert_eq!(dodged_with(&actions), 4120002);
        assert_eq!(player.stats.hp, 100);
        // Guardian needs a shield. A touch it blocks comes with the 14
        // bytes, not Power Guard's.
        player.stats.job = 112;
        learn(&mut player, 1120005);
        let mut blocked = vec![0, 2, 0, 0, 0, 0, 0, 6];
        blocked.extend([0; 9]);
        assert!(handle(&mut player, touch_with(-1, 100100, &blocked)).is_empty());
        player.inventory.insert(
            InventoryType::Equipped,
            game::inventory::equip_slot::SHIELD,
            game::Item::equipment(1092000, game::EquipStats::default()),
        );
        let actions = handle(&mut player, touch_with(-1, 100100, &blocked));
        assert_eq!(dodged_with(&actions), 1120005);
        assert_eq!(player.stats.hp, 100);
    }

    #[test]
    fn guards_take_their_share_of_a_touch() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        (player.stats.mp, player.stats.max_mp) = (100, 100);
        player.meso = 1000;
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        for (skill_id, stat) in [
            (2001002, BuffStat::MagicGuard),
            (4211005, BuffStat::MesoGuard),
        ] {
            player.buffs.give(Buff::new(skill_id, stat, 30, now, lasts));
        }
        // Stumpy's body: 40 is 12 to MP and 20 saved for 6 mesos.
        assert!(game::mob_damage::most_physical(&sample_defender(), 125) >= 40);
        handle(&mut player, touch(40, 3220000));
        assert_eq!(
            (player.stats.hp, player.stats.mp, player.meso),
            (92, 88, 994)
        );
    }

    #[test]
    fn power_guard_sends_some_of_a_touch_back() {
        if crate::mobs::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        // Enough accuracy to reach Stumpy's 12 avoid 25 levels up.
        player.stats.dex = 100;
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        player
            .buffs
            .give(Buff::new(1101007, BuffStat::PowerGuard, 30, now, lasts));
        // The client's percent, no Guardian, then Power Guard's 14 bytes.
        let mut tail = vec![30, 0, 1];
        tail.extend(2_000_000_001i32.to_le_bytes());
        tail.extend([6, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // Stumpy is a boss: 30% of 40 is 12, halved.
        let actions = handle(&mut player, touch_with(40, 3220000, &tail));
        assert_eq!(player.stats.hp, 66);
        assert!(actions.iter().any(|action| matches!(
            action,
            WorldAction::Field(FieldRequest::Reflect {
                object_id: 2_000_000_001,
                mob_id: 3220000,
                damage: 6
            })
        )));
        // A client that doesn't say so sends nothing back.
        let actions = handle(&mut player, touch(40, 3220000));
        assert_eq!(player.stats.hp, 26);
        assert!(!actions
            .iter()
            .any(|action| matches!(action, WorldAction::Field(FieldRequest::Reflect { .. }))));
    }

    #[test]
    fn mana_reflection_sends_some_of_a_spell_back() {
        if crate::mobs::data().is_err() || crate::skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 100;
        // Enough INT to reach the mob's 8 avoid 10 levels up.
        player.stats.int = 100;
        // The buff is the skill's level.
        let mana = crate::skills::at_level(2321002, 10).unwrap();
        assert_eq!(
            crate::skills::buffs(&mana),
            Some(vec![(BuffStat::ManaReflection, 10)])
        );
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        player
            .buffs
            .give(Buff::new(2121002, BuffStat::ManaReflection, 10, now, lasts));
        // Its percent, no Guardian, then the 14 bytes, not Power Guard's.
        let mut tail = vec![100, 0, 0];
        tail.extend(2_000_000_001i32.to_le_bytes());
        tail.extend([6, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // 9400568's spell: all 40 goes back, but at most 350 / 20, and the
        // player still takes all of it.
        let mut p = hit(0, 40, 9400568);
        p.write_bytes(&tail).unwrap();
        let actions = handle(&mut player, p);
        assert_eq!(player.stats.hp, 60);
        assert!(actions.iter().any(|action| matches!(
            action,
            WorldAction::Field(FieldRequest::Reflect {
                object_id: 2_000_000_001,
                mob_id: 9400568,
                damage: 17
            })
        )));
        // A touch says nothing of Mana Reflection.
        let actions = handle(&mut player, touch_with(40, 9400568, &tail));
        assert_eq!(player.stats.hp, 20);
        assert!(!actions
            .iter()
            .any(|action| matches!(action, WorldAction::Field(FieldRequest::Reflect { .. }))));
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
