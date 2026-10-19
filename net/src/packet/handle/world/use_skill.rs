use super::combat::set_combo;
use crate::combo;
use crate::error::NetworkError;
use crate::handler::{
    FieldRequest, HandlerResult, PartySkill, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::stat::{self, StatUpdate};
use crate::packet::build::world::summon::Gone;
use crate::packet::build::world::{buff, inventory, skill};
use crate::skills::{self, Inflict, Learned};
use crate::summons;
use game::{Buff, BuffStat, InventoryChange, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::{Duration, Instant};
use tracing::debug;

/// `USE_SKILL`: a skill that doesn't attack: the buffs (see
/// `skills::buffs`), the summons, Heal, and Monster Magnet. Every answer
/// unlocks the client, which waits on it before using another skill or
/// item. A skill that reaches the party gives its buffs, or Heal its HP, to
/// the members the client found around the player too.
pub struct UseSkillHandler;

impl WorldHandler for UseSkillHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        r.read_int()?; // tick
        let skill_id = r.read_int()?;
        let level = i32::from(r.read_byte()?);
        if skills::is_magnet(skill_id) {
            let caught = read_magnet(&mut r)?;
            return match use_magnet(ctx.player, skill_id, level, caught.as_ref()) {
                Ok(skill) => {
                    let (caught, toleft) = caught.unwrap_or_default();
                    announce_magnet(ctx.player, &skill, caught, toleft)
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Monster Magnet refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        if skill_id == skills::HEAL {
            let party = r.read_byte()?;
            return match heal(ctx.player, level, party, rand::random()) {
                Ok((hp, amount)) => {
                    let mp = ctx.player.stats.mp;
                    let result = HandlerResult::reply(stat::build_stat_update(
                        true,
                        &[StatUpdate::Hp(hp), StatUpdate::Mp(mp)],
                    )?);
                    Ok(with_party(result, skill_id, level, Some(party), amount))
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Heal refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        if summons::kind(skill_id).is_some() {
            // Where the client put it (FUN_0096b140) and which way it faces.
            let at = (r.read_short()?, r.read_short()?);
            let left = r.read_byte()? != 0;
            return match use_skill(ctx.player, skill_id, level, Instant::now()) {
                Ok(used) => {
                    let lasts = Duration::from_secs(used.skill.effect.duration.max(0) as u64);
                    Ok(announce(ctx.player, &used)?.with(WorldAction::Field(
                        FieldRequest::Summon {
                            skill_id,
                            level,
                            at,
                            left,
                            lasts,
                            appear: true,
                        },
                    )))
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Summon refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        if skill_id == skills::MYSTIC_DOOR {
            // Where the player stands (0x969bf9).
            let at = (r.read_short()?, r.read_short()?);
            let town = crate::game_data::get()?
                .field(ctx.player.stats.map_id)
                .and_then(|field| skills::door_town(field, at));
            let opened = match town {
                Some(town) => {
                    use_skill(ctx.player, skill_id, level, Instant::now()).map(|used| (town, used))
                }
                None => Err(Refusal::NoDoorHere),
            };
            return match opened {
                Ok((town, used)) => {
                    let lasts = Duration::from_secs(used.skill.effect.duration.max(0) as u64);
                    Ok(
                        announce(ctx.player, &used)?.with(WorldAction::OpenDoor {
                            town,
                            at,
                            lasts,
                        }),
                    )
                }
                Err(refusal) => {
                    debug!(skill_id, level, ?refusal, "Mystic Door refused");
                    Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
                }
            };
        }
        let extras = read_extras(&mut r, skill_id);
        let now = Instant::now();
        match use_skill(ctx.player, skill_id, level, now) {
            Ok(used) => {
                if skill_id == skills::CHAKRA {
                    let heal = skills::chakra_heal(ctx.player, used.skill.effect.y, rand::random());
                    ctx.player.recover_hp(heal);
                }
                let mut result = announce(ctx.player, &used)?;
                if skill_id == skills::TIME_LEAP {
                    result = leap(ctx.player, result)?;
                }
                if skill_id == skills::DISPEL {
                    result = dispel(ctx.player, &used.skill, result, rand::random())?;
                }
                if skills::HEROS_WILL.contains(&skill_id) {
                    result = cure(ctx.player, &skills::PURGED, result)?;
                }
                if skill_id == combo::ENRAGE {
                    result = set_combo(ctx.player, 1, now, result)?;
                }
                let result = with_party(result, skill_id, level, extras.party, 0);
                let Some(inflict) = used.debuffs.clone() else {
                    return Ok(result);
                };
                Ok(result.with(WorldAction::Field(FieldRequest::MobDebuff {
                    skill_id,
                    level,
                    origin: extras.position,
                    mobs: extras.mobs,
                    inflict,
                })))
            }
            Err(refusal) => {
                debug!(skill_id, level, ?refusal, "Skill use refused");
                Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?))
            }
        }
    }
}

/// `SKILL_EFFECT`: the player started preparing a skill, held down or
/// timed. Show it to the rest of the map.
pub struct SkillEffectHandler;

impl WorldHandler for SkillEffectHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let skill_id = r.read_int()?;
        let level = r.read_byte()?;
        let action_and_facing = r.read_short()? as u16;
        let speed = r.read_byte()?;
        let learned = skills::learned(ctx.player, skill_id)
            .is_some_and(|skill| skill.level == i32::from(level));
        if ctx.player.stats.hp <= 0 || !skills::prepares(skill_id) || !learned {
            return Ok(HandlerResult::empty());
        }
        let shown =
            skill::build_skill_prepare(ctx.player.id(), skill_id, level, action_and_facing, speed)?;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

/// `THROW_GRENADE`: the player let go of Grenade or Poison Bomb, throwing
/// it from where they stood (0x95efc5). Show the rest of the map the throw,
/// and end the prepare it showed: the client sends no `CANCEL_BUFF` for a
/// throw, and the attack comes only once the grenade goes off. The attack
/// is checked and spends MP then, as any other.
pub struct ThrowGrenadeHandler;

impl WorldHandler for ThrowGrenadeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let x = r.read_int()?;
        let y = r.read_int()?;
        let charge = r.read_int()?;
        let skill_id = r.read_int()?;
        if ctx.player.stats.hp <= 0
            || !skills::throws(skill_id)
            || skills::learned(ctx.player, skill_id).is_none()
        {
            return Ok(HandlerResult::empty());
        }
        let id = ctx.player.id();
        let ended = skill::build_cancel_skill_prepare(id, skill_id)?;
        let thrown = skill::build_throw_grenade(id, x, y, charge, skill_id)?;
        Ok(HandlerResult::empty()
            .with(WorldAction::Field(FieldRequest::Broadcast(ended)))
            .with(WorldAction::Field(FieldRequest::Broadcast(thrown))))
    }
}

/// `CANCEL_BUFF`: the player right-clicked a skill's buff icon, or let go
/// of a held skill without using it. End the buffs and tell the client,
/// which then drops the icon; or show the rest of the map the skill was
/// let go. Also `CANCEL_ITEM_EFFECT` (0x49, v83 FUN_00a096af), whose int
/// is an item buff's id, `-item_id`.
pub struct CancelBuffHandler;

impl WorldHandler for CancelBuffHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let skill_id = r.read_int()?;
        if skills::keydown(skill_id) {
            let shown = skill::build_cancel_skill_prepare(ctx.player.id(), skill_id)?;
            return Ok(
                HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown)))
            );
        }
        let ended = ctx.player.buffs.cancel(skill_id);
        if ended.is_empty() {
            return Ok(HandlerResult::empty());
        }
        let mut result = HandlerResult::reply(buff::build_cancel_buff(&ended)?);
        if summons::kind(skill_id).is_some() {
            result = result.with(WorldAction::Field(FieldRequest::Unsummon {
                skill_id: Some(skill_id),
                why: Gone::Dismissed,
            }));
        }
        let unseen: Vec<_> = ended.into_iter().filter(|stat| stat.shown()).collect();
        if unseen.is_empty() {
            return Ok(result);
        }
        let foreign = buff::build_cancel_foreign_buff(ctx.player.id(), &unseen)?;
        Ok(result.with(WorldAction::BuffsChanged(foreign)))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Dead,
    /// Stunned, or sealed and not casting Dispel, which the client allows
    /// (0x96758f).
    Diseased,
    /// Not learned, or not at the level the client named.
    NotLearned,
    /// Monster Magnet reaching more mobs than it can.
    TooManyMobs,
    Unsupported,
    CoolingDown,
    NotEnoughHp,
    NotEnoughMp,
    /// Without the item the skill costs.
    NoItem,
    /// Mystic Door where no door may open.
    NoDoorHere,
    /// Enrage without the orbs to spend.
    TooFewOrbs,
}

/// What USE_SKILL carries after the level (0x96d399), each part only for
/// the skills that send it.
#[derive(Debug, Default, PartialEq, Eq)]
struct Extras {
    /// Where the player stood, for a skill that places something or
    /// reaches those around them.
    position: Option<(i16, i16)>,
    /// A bit for each party member reached, for a skill that reaches the
    /// party.
    party: Option<u8>,
    /// The mobs reached, for a skill that reaches mobs.
    mobs: Vec<i32>,
}

/// Read what follows the level: where the player stands, Shadow Stars'
/// star, the party bits (with Dispel's delay), and the mobs reached, each
/// for the skills that send it. As much as was there, for a packet cut
/// short.
fn read_extras(r: &mut Cursor<&Vec<u8>>, skill_id: i32) -> Extras {
    let mut extras = Extras::default();
    let _ = read_extras_into(r, skill_id, &mut extras);
    extras
}

fn read_extras_into(
    r: &mut Cursor<&Vec<u8>>,
    skill_id: i32,
    extras: &mut Extras,
) -> Result<(), NetworkError> {
    if skills::sends_position(skill_id) {
        extras.position = Some((r.read_short()?, r.read_short()?));
    }
    if skill_id == skills::SHADOW_STARS {
        r.read_int()?;
    }
    if skills::reaches_party(skill_id) {
        extras.party = Some(r.read_byte()?);
        if skill_id == skills::DISPEL {
            r.read_short()?;
        }
    }
    if skills::reaches_mobs(skill_id) {
        let count = r.read_byte()?;
        for _ in 0..count {
            let object_id = r.read_int()?;
            if object_id != 0 {
                extras.mobs.push(object_id);
            }
        }
    }
    Ok(())
}

/// Pass a used skill on to the party members it reached besides the
/// player, if any.
fn with_party(
    result: HandlerResult<WorldAction>,
    skill_id: i32,
    level: i32,
    party: Option<u8>,
    heal: i32,
) -> HandlerResult<WorldAction> {
    match party {
        Some(slots) if slots != skills::ALONE && slots != 0 => {
            result.with(WorldAction::PartySkill(PartySkill {
                skill_id,
                level,
                slots,
                heal,
            }))
        }
        _ => result,
    }
}

/// Monster Magnet reaches at most this many mobs (0x96c215).
const MAGNET_MOBS: i32 = 15;

/// What Monster Magnet sends after its level (0x96c62a): each mob it
/// reached, 0 for one out of sight, and whether it was caught; then which
/// way the player faced. None if it claims more mobs than it can reach.
fn read_magnet(r: &mut Cursor<&Vec<u8>>) -> Result<Option<(Vec<(i32, bool)>, bool)>, NetworkError> {
    let found = r.read_int()?;
    if !(0..=MAGNET_MOBS).contains(&found) {
        return Ok(None);
    }
    let mut caught = Vec::with_capacity(found as usize);
    for _ in 0..found {
        let object_id = r.read_int()?;
        let success = r.read_byte()? != 0;
        if object_id != 0 {
            caught.push((object_id, success));
        }
    }
    let toleft = r.read_byte()? != 0;
    Ok(Some((caught, toleft)))
}

/// Dispel reached the player: by its chance (`prop`, a roll in 0..1), it
/// cures their diseases but Stun, Seduce, and Confuse (Cosmic's
/// StatEffect.applyTo).
fn dispel(
    player: &mut Player,
    skill: &Learned,
    result: HandlerResult<WorldAction>,
    roll: f64,
) -> Result<HandlerResult<WorldAction>, NetworkError> {
    if roll * 100.0 >= f64::from(skill.effect.prop) {
        return Ok(result);
    }
    cure(player, &skills::DISPELLED, result)
}

/// End the player's diseases among `stats`, telling them and, for those it
/// sees, the map.
fn cure(
    player: &mut Player,
    stats: &[BuffStat],
    result: HandlerResult<WorldAction>,
) -> Result<HandlerResult<WorldAction>, NetworkError> {
    let ended = player.buffs.cure(stats);
    if ended.is_empty() {
        return Ok(result);
    }
    let mut result = result.with_reply(buff::build_cancel_buff(&ended)?);
    let unseen: Vec<_> = ended.into_iter().filter(|stat| stat.shown()).collect();
    if !unseen.is_empty() {
        let foreign = buff::build_cancel_foreign_buff(player.id(), &unseen)?;
        result = result.with(WorldAction::BuffsChanged(foreign));
    }
    Ok(result)
}

/// Whether a disease keeps the player from using the skill: Stun, or Seal
/// but for Dispel.
fn kept_from(player: &Player, skill_id: i32) -> bool {
    let buffs = &player.buffs;
    buffs.has(BuffStat::Stun) || (buffs.has(BuffStat::Seal) && skill_id != skills::DISPEL)
}

/// Check Monster Magnet can be used, then spend its MP.
fn use_magnet(
    player: &mut Player,
    skill_id: i32,
    level: i32,
    caught: Option<&(Vec<(i32, bool)>, bool)>,
) -> Result<Learned, Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    if kept_from(player, skill_id) {
        return Err(Refusal::Diseased);
    }
    let skill = skills::learned(player, skill_id)
        .filter(|skill| skill.level == level && level > 0)
        .ok_or(Refusal::NotLearned)?;
    if caught.is_none() {
        return Err(Refusal::TooManyMobs);
    }
    if i32::from(player.stats.mp) < skill.effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }
    player.add_mp(-skill.effect.mp_cost);
    Ok(skill)
}

/// The new MP for the player; for everyone else on the map, the skill's
/// effect and each mob it caught or failed to.
fn announce_magnet(
    player: &Player,
    skill: &Learned,
    caught: Vec<(i32, bool)>,
    toleft: bool,
) -> WorldResult {
    let effect = buff::build_magnet_effect(
        player.id(),
        skill.skill_id,
        player.stats.level,
        skill.level as u8,
        toleft,
    )?;
    Ok(HandlerResult::reply(stat::build_stat_update(
        true,
        &[StatUpdate::Mp(player.stats.mp)],
    )?)
    .with(WorldAction::Field(FieldRequest::Magnet { effect, caught })))
}

/// Heal comes before its attack, with a bit for each party member it
/// reaches (0x96a69d), or 0x80 without a party. It heals the cleric, and
/// each member it reached, by one's share: the cleric's new HP and the
/// share. It spends the MP, so the attack doesn't.
fn heal(player: &mut Player, level: i32, party: u8, roll: f64) -> Result<(i16, i32), Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    if kept_from(player, skills::HEAL) {
        return Err(Refusal::Diseased);
    }
    let skill = skills::learned(player, skills::HEAL)
        .filter(|skill| skill.level == level)
        .ok_or(Refusal::NotLearned)?;
    if i32::from(player.stats.mp) < skill.effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }
    player.add_mp(-skill.effect.mp_cost);
    let reached = party.count_ones() as i32;
    let amount = skills::heal_amount(player, skill.effect.hp, reached, roll) as i32;
    Ok((player.recover_hp(amount), amount))
}

/// A skill that was used: the buffs it gave, and the items it cost. A
/// skill that `debuffs` the mobs it reached gives no buffs.
#[derive(Debug)]
struct Used {
    skill: Learned,
    /// Whether the skill buffs: those that act alone neither buff nor
    /// debuff.
    buffs: bool,
    stats: Vec<(BuffStat, i16)>,
    items: Vec<InventoryChange>,
    debuffs: Option<Inflict>,
}

/// Check the skill can be used now, then spend its HP, MP and items and
/// start its buffs and cooldown.
fn use_skill(
    player: &mut Player,
    skill_id: i32,
    level: i32,
    now: Instant,
) -> Result<Used, Refusal> {
    if player.stats.hp <= 0 {
        return Err(Refusal::Dead);
    }
    if kept_from(player, skill_id) {
        return Err(Refusal::Diseased);
    }
    let skill = skills::learned(player, skill_id)
        .filter(|skill| skill.level == level)
        .ok_or(Refusal::NotLearned)?;
    if combo::enrage_refused(player, skill_id) {
        return Err(Refusal::TooFewOrbs);
    }
    let stats = skills::buffs(&skill);
    let buffs = stats.is_some();
    let debuffs = match stats {
        Some(_) => None,
        None if skills::acts_alone(skill_id) => None,
        None => Some(skills::mob_debuffs(&skill, player).ok_or(Refusal::Unsupported)?),
    };
    if player.buffs.cooling_down(skill_id, now) {
        return Err(Refusal::CoolingDown);
    }
    let effect = &skill.effect;
    let hp_cost = skills::hp_cost(player, &skill).ok_or(Refusal::NotEnoughHp)?;
    if i32::from(player.stats.mp) < effect.mp_cost {
        return Err(Refusal::NotEnoughMp);
    }
    let cost = effect.item_cost_count.max(1);
    if effect.item_cost != 0 && player.inventory.count(effect.item_cost) < cost {
        return Err(Refusal::NoItem);
    }

    player.add_hp(-hp_cost);
    player.add_mp(-effect.mp_cost);
    let items = if effect.item_cost != 0 {
        player
            .inventory_mut()
            .remove(effect.item_cost, cost as i16)
            .map_err(|_| Refusal::NoItem)?
    } else {
        Vec::new()
    };
    let stats = stats.map_or_else(Vec::new, |stats| give(player, &skill, stats, now));
    if effect.cooldown > 0 {
        let length = Duration::from_secs(effect.cooldown as u64);
        player.buffs.start_cooldown(skill_id, now, length);
    }
    Ok(Used {
        skill,
        buffs,
        stats,
        items,
        debuffs,
    })
}

/// Start a skill's buffs on the player: the stats it gives, as GIVE_BUFF
/// carries them.
fn give(
    player: &mut Player,
    skill: &Learned,
    stats: Vec<(BuffStat, i32)>,
    now: Instant,
) -> Vec<(BuffStat, i16)> {
    let duration = Duration::from_secs(skill.effect.duration.max(0) as u64);
    let stats: Vec<_> = stats
        .into_iter()
        .map(|(stat, value)| (stat, value as i16))
        .collect();
    for &(stat, value) in &stats {
        player
            .buffs
            .give(Buff::new(skill.skill_id, stat, value, now, duration));
    }
    stats
}

/// The GIVE_FOREIGN_BUFF showing the others the buffs among `stats` they
/// see, if any.
pub(super) fn show_given(
    player: &Player,
    stats: &[(BuffStat, i16)],
) -> Result<Option<Packet>, NetworkError> {
    let given: Vec<_> = player
        .buffs
        .shown()
        .into_iter()
        .filter(|shown| stats.iter().any(|(given, _)| *given == shown.stat))
        .collect();
    if given.is_empty() {
        return Ok(None);
    }
    Ok(Some(buff::build_give_foreign_buff(player.id(), &given)?))
}

/// A party member's skill reached the player, who stands at `at`; it was
/// used at `origin`. If they are alive and in its area, give them its
/// buffs, or heal them, and show them and the others its `affected`
/// animation on them.
pub fn party_skill_reached(
    player: &mut Player,
    used: &PartySkill,
    origin: (i16, i16),
    at: (i16, i16),
    now: Instant,
) -> WorldResult {
    let Some(skill) = skills::at_level(used.skill_id, used.level) else {
        return Ok(HandlerResult::empty());
    };
    // Resurrection reaches only the dead, the rest only the living.
    let dead = player.stats.hp <= 0;
    if dead != (used.skill_id == skills::RESURRECTION)
        || !skills::in_area(&skill.effect, origin, at)
    {
        return Ok(HandlerResult::empty());
    }
    let mut result = if used.skill_id == skills::HEAL || used.skill_id == skills::RESURRECTION {
        // Resurrection raises them with full HP (OdinMS-family servers).
        let hp = if dead {
            player.add_hp(i32::from(player.stats.max_hp))
        } else {
            player.recover_hp(used.heal.max(0))
        };
        HandlerResult::reply(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?)
    } else if used.skill_id == skills::TIME_LEAP {
        leap(player, HandlerResult::empty())?
    } else if used.skill_id == skills::DISPEL {
        dispel(player, &skill, HandlerResult::empty(), rand::random())?
    } else {
        let Some(stats) = skills::buffs(&skill) else {
            return Ok(HandlerResult::empty());
        };
        let stats = give(player, &skill, stats, now);
        let mut result = HandlerResult::reply(buff::build_give_buff(
            skill.skill_id,
            skill.effect.duration * 1000,
            &stats,
        )?);
        if let Some(foreign) = show_given(player, &stats)? {
            result = result.with(WorldAction::BuffsChanged(foreign));
        }
        result
    };
    let level = skill.level as u8;
    result = result.with_reply(buff::build_own_affected(skill.skill_id, level)?);
    let shown = buff::build_foreign_affected(player.id(), skill.skill_id, level)?;
    Ok(result.with(WorldAction::Field(FieldRequest::Broadcast(shown))))
}

/// Time Leap ends the player's cooldowns but its own, and tells their client.
fn leap(player: &mut Player, mut result: HandlerResult<WorldAction>) -> WorldResult {
    for skill_id in player.buffs.end_cooldowns_except(skills::TIME_LEAP) {
        result = result.with_reply(buff::build_cooldown(skill_id, 0)?);
    }
    Ok(result)
}

/// The new MP (and HP), the items spent, the buff, and the cooldown for the player;
/// the skill's effect and the buffs they show for everyone else on the map.
fn announce(player: &Player, used: &Used) -> WorldResult {
    let Used {
        skill,
        buffs,
        stats,
        items,
        ..
    } = used;
    let effect = &skill.effect;
    let mut updates = vec![StatUpdate::Mp(player.stats.mp)];
    // Chakra's heal comes with its MP, as does the HP a Booster costs.
    if skill.skill_id == skills::CHAKRA || effect.hp_cost > 0 {
        updates.push(StatUpdate::Hp(player.stats.hp));
    }
    let mut result = HandlerResult::reply(stat::build_stat_update(true, &updates)?);
    if !items.is_empty() {
        result = result.with_reply(inventory::build_inventory_operation(false, items)?);
    }
    if *buffs {
        result = result.with_reply(buff::build_give_buff(
            skill.skill_id,
            effect.duration * 1000,
            stats,
        )?);
    }
    if effect.cooldown > 0 {
        result = result.with_reply(buff::build_cooldown(
            skill.skill_id,
            effect.cooldown as i16,
        )?);
    }
    let shown = buff::build_skill_effect(
        player.id(),
        skill.skill_id,
        player.stats.level,
        skill.level as u8,
    )?;
    result = result.with(WorldAction::Field(FieldRequest::Broadcast(shown)));
    if let Some(foreign) = show_given(player, stats)? {
        result = result.with(WorldAction::BuffsChanged(foreign));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use crate::skills::Amount;
    use game::{MobStat, SkillRecord};
    use packet::io::write::PktWrite;

    fn beginner_with(skill_id: i32, level: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 50;
        player.stats.mp = 20;
        player.stats.max_mp = 20;
        player.skills.set(SkillRecord {
            skill_id,
            level,
            master_level: 0,
            expiration: None,
        });
        player
    }

    fn handle(player: &mut Player, skill_id: i32, level: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(skill_id).unwrap();
        packet.write_byte(level).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    /// Whether the action is a stat update that unlocks the client.
    fn unlocks(action: &WorldAction) -> bool {
        matches!(action, WorldAction::Reply(p)
            if p.opcode() == SendOpcode::StatChange as i16 && p.bytes[2] == 1)
    }

    fn opcodes(result: WorldResult) -> Vec<i16> {
        result
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => p.opcode(),
                WorldAction::Field(FieldRequest::Broadcast(p)) => p.opcode(),
                WorldAction::BuffsChanged(p) => p.opcode(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_booster_spends_hp_too() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        // Sword Booster at 1 costs 29 HP and 29 MP.
        let mut player = beginner_with(1101004, 1);
        (player.stats.mp, player.stats.max_mp) = (40, 40);
        let used = use_skill(&mut player, 1101004, 1, now).unwrap();
        assert_eq!((player.stats.hp, player.stats.mp), (21, 11));
        let sent = announce(&player, &used).unwrap().actions.into_iter().next();
        let Some(WorldAction::Reply(stats)) = sent else {
            panic!("no stats");
        };
        let mut r = Cursor::new(&stats.bytes);
        assert_eq!(r.read_short().unwrap(), SendOpcode::StatChange as i16);
        r.read_byte().unwrap();
        assert_eq!(r.read_int().unwrap(), 0x400 | 0x1000, "HP and MP");
        player.buffs = Default::default();
        (player.stats.hp, player.stats.mp) = (29, 40);
        assert_eq!(
            use_skill(&mut player, 1101004, 1, now).unwrap_err(),
            Refusal::NotEnoughHp
        );
    }

    #[test]
    fn nimble_feet_spends_mp_and_gives_speed_until_it_cools_down() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(1002, 2);
        assert_eq!(
            use_skill(&mut player, 1002, 1, now).unwrap_err(),
            Refusal::NotLearned
        );
        let used = use_skill(&mut player, 1002, 2, now).unwrap();
        let sent = opcodes(announce(&player, &used));
        assert_eq!(
            sent,
            [
                SendOpcode::StatChange as i16,
                SendOpcode::GiveBuff as i16,
                SendOpcode::Cooldown as i16,
                SendOpcode::ShowForeignEffect as i16,
                SendOpcode::GiveForeignBuff as i16,
            ]
        );
        assert_eq!(player.stats.mp, 13);
        let buff = &player.buffs.active()[0];
        assert_eq!((buff.stat, buff.value), (BuffStat::Speed, 15));
        assert_eq!(buff.expires, now + Duration::from_secs(8));
        assert_eq!(
            use_skill(&mut player, 1002, 2, now + Duration::from_secs(59)).unwrap_err(),
            Refusal::CoolingDown
        );
        player.stats.mp = 6;
        assert_eq!(
            use_skill(&mut player, 1002, 2, now + Duration::from_secs(60)).unwrap_err(),
            Refusal::NotEnoughMp
        );
    }

    #[test]
    fn cancelling_a_buff_ends_it_and_tells_the_client() {
        use crate::packet::op::RecvOpcode;
        use packet::io::write::PktWrite;

        let mut player = beginner_with(1002, 2);
        let now = Instant::now();
        player.buffs.give(Buff::new(
            1002,
            BuffStat::Speed,
            15,
            now,
            Duration::from_secs(8),
        ));
        let mut cancel = |skill_id: i32| {
            let mut packet = Packet::new_empty();
            packet.write_short(RecvOpcode::CancelBuff as i16).unwrap();
            packet.write_int(skill_id).unwrap();
            let mut ctx = WorldContext {
                client_id: 1,
                channel_id: 0,
                player: &mut player,
            };
            opcodes(CancelBuffHandler.handle(&mut packet, &mut ctx))
        };
        assert_eq!(cancel(1001), Vec::<i16>::new());
        assert_eq!(
            cancel(1002),
            [
                SendOpcode::CancelBuff as i16,
                SendOpcode::CancelForeignBuff as i16
            ]
        );
        assert_eq!(cancel(1002), Vec::<i16>::new());
        assert!(player.buffs.active().is_empty());
    }

    #[test]
    fn job_buffs_give_every_stat_of_their_level() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(4101004, 20);
        (player.stats.mp, player.stats.max_mp) = (200, 200);
        let used = use_skill(&mut player, 4101004, 20, now).unwrap();
        assert_eq!(used.stats, [(BuffStat::Speed, 40), (BuffStat::Jump, 20)]);
        assert_eq!(
            player.buffs.shown(),
            [game::ShownBuff {
                stat: BuffStat::Speed,
                value: 40,
                skill_id: 4101004,
            }]
        );

        // Hyper Body's own stats; nothing others see.
        player.skills.set(SkillRecord {
            skill_id: 1301007,
            level: 30,
            master_level: 0,
            expiration: None,
        });
        let used = use_skill(&mut player, 1301007, 30, now).unwrap();
        assert_eq!(used.stats, [(BuffStat::MaxHp, 60), (BuffStat::MaxMp, 60)]);
        let sent = opcodes(announce(&player, &used));
        assert!(!sent.contains(&(SendOpcode::GiveForeignBuff as i16)));
    }

    #[test]
    fn shadow_partner_costs_a_summoning_rock() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(4111002, 30);
        (player.stats.mp, player.stats.max_mp) = (200, 200);
        assert_eq!(
            use_skill(&mut player, 4111002, 30, now).unwrap_err(),
            Refusal::NoItem
        );
        assert_eq!(player.stats.mp, 200);
        player
            .inventory_mut()
            .add(game::Item::stack(4006001, 2), 100)
            .unwrap();
        let used = use_skill(&mut player, 4111002, 30, now).unwrap();
        assert_eq!(used.stats, [(BuffStat::ShadowPartner, 80)]);
        assert_eq!(player.inventory.count(4006001), 1);
        let sent = opcodes(announce(&player, &used));
        assert_eq!(
            sent,
            [
                SendOpcode::StatChange as i16,
                SendOpcode::InventoryOperation as i16,
                SendOpcode::GiveBuff as i16,
                SendOpcode::ShowForeignEffect as i16,
                SendOpcode::GiveForeignBuff as i16,
            ]
        );
    }

    #[test]
    fn teleport_spends_its_mp() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(2301001, 1);
        player.stats.max_mp = 100;
        player.stats.mp = 100;
        let used = use_skill(&mut player, 2301001, 1, Instant::now()).unwrap();
        assert!(used.skill.effect.mp_cost > 0);
        assert_eq!(i32::from(player.stats.mp), 100 - used.skill.effect.mp_cost);
    }

    #[test]
    fn flash_jump_spends_its_mp() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(4111006, 1);
        player.stats.max_mp = 100;
        player.stats.mp = 100;
        let used = use_skill(&mut player, 4111006, 1, Instant::now()).unwrap();
        assert!(!used.buffs);
        assert_eq!(player.stats.mp, 40);
    }

    #[test]
    fn recovery_heals_by_its_x_and_attacks_are_not_buffs() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(1001, 3);
        assert!(use_skill(&mut player, 1001, 3, Instant::now()).is_ok());
        let buff = &player.buffs.active()[0];
        assert_eq!((buff.stat, buff.value), (BuffStat::Recovery, 12));

        let mut player = beginner_with(1000, 1);
        assert_eq!(
            use_skill(&mut player, 1000, 1, Instant::now()).unwrap_err(),
            Refusal::Unsupported
        );
    }

    #[test]
    fn heal_heals_the_cleric_by_their_share() {
        if skills::data().is_err() {
            return;
        }
        let cleric = || {
            let mut player = beginner_with(skills::HEAL, 30);
            player.stats.int = 400;
            player.stats.luk = 100;
            player.stats.max_hp = 10_000;
            player.stats.mp = 100;
            player.stats.max_mp = 100;
            player
        };
        // Alone, the lowest roll: (0.3 + 1) * 300% * (80 * 1.5 + 100) * 400
        // * 0.005.
        let mut player = cleric();
        assert_eq!(heal(&mut player, 30, 0x80, 0.0), Ok((50 + 1716, 1716)));
        // Reaching two, each gets (0.6 + 1) / 2 of it.
        let mut player = cleric();
        assert_eq!(heal(&mut player, 30, 0x30, 0.0), Ok((50 + 1056, 1056)));
        // Heal spends the MP, not its attack.
        let cost = skills::learned(&player, skills::HEAL)
            .unwrap()
            .effect
            .mp_cost;
        assert!(cost > 0);
        assert_eq!(i32::from(player.stats.mp), 100 - cost);

        assert_eq!(heal(&mut player, 29, 0x80, 0.0), Err(Refusal::NotLearned));
        player.stats.mp = 0;
        assert_eq!(heal(&mut player, 30, 0x80, 0.0), Err(Refusal::NotEnoughMp));
    }

    /// USE_SKILL for a skill reaching the party in `slots`, as the client
    /// sends it.
    fn handle_party(player: &mut Player, skill_id: i32, level: u8, slots: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(skill_id).unwrap();
        packet.write_byte(level).unwrap();
        if skills::sends_position(skill_id) {
            packet.write_short(10).unwrap();
            packet.write_short(20).unwrap();
        }
        packet.write_byte(slots).unwrap();
        packet.write_short(0).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn passed_on(actions: &[WorldAction]) -> Vec<PartySkill> {
        actions
            .iter()
            .filter_map(|action| match action {
                WorldAction::PartySkill(used) => Some(used.clone()),
                _ => None,
            })
            .collect()
    }

    fn bishop_with(skill_id: i32, level: i32) -> Player {
        let mut player = beginner_with(skill_id, level);
        player.stats.int = 400;
        player.stats.luk = 100;
        player.stats.max_hp = 10_000;
        (player.stats.mp, player.stats.max_mp) = (1000, 1000);
        player
    }

    #[test]
    fn party_skills_are_passed_on_to_the_members_the_client_found() {
        if skills::data().is_err() {
            return;
        }
        // Bless, the user in slot 0 and another in slot 2.
        let mut player = bishop_with(2301004, 20);
        let actions = handle_party(&mut player, 2301004, 20, 0x28);
        assert!(actions.iter().any(unlocks));
        assert_eq!(
            passed_on(&actions),
            [PartySkill {
                skill_id: 2301004,
                level: 20,
                slots: 0x28,
                heal: 0
            }]
        );
        // Alone, or with only themselves around.
        let mut player = bishop_with(2301004, 20);
        assert!(passed_on(&handle_party(&mut player, 2301004, 20, skills::ALONE)).is_empty());
        let mut player = bishop_with(2301004, 20);
        assert!(passed_on(&handle_party(&mut player, 2301004, 20, 0)).is_empty());
        // A packet cut short after the level.
        let mut player = bishop_with(2301004, 20);
        assert!(passed_on(&handle(&mut player, 2301004, 20)).is_empty());

        // Heal passes on one's share.
        let mut player = bishop_with(skills::HEAL, 30);
        let used = passed_on(&handle_party(&mut player, skills::HEAL, 30, 0x30));
        assert_eq!(used.len(), 1);
        assert_eq!(used[0].slots, 0x30);
        assert_eq!(player.stats.hp as i32, 50 + used[0].heal);
    }

    #[test]
    fn debuff_skills_pass_the_mobs_they_reached_to_the_field() {
        if skills::data().is_err() {
            return;
        }
        // Threaten: where the page stands, then two mobs, then the delay.
        let mut player = beginner_with(1201006, 20);
        (player.stats.mp, player.stats.max_mp) = (100, 100);
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(1201006).unwrap();
        packet.write_byte(20).unwrap();
        packet.write_short(-120).unwrap();
        packet.write_short(35).unwrap();
        packet.write_byte(2).unwrap();
        packet.write_int(1000).unwrap();
        packet.write_int(1001).unwrap();
        packet.write_short(0).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        assert!(actions.iter().any(unlocks));
        let debuff = actions.iter().find_map(|action| match action {
            WorldAction::Field(FieldRequest::MobDebuff {
                skill_id,
                level,
                origin,
                mobs,
                inflict,
            }) => {
                assert_eq!(
                    inflict.statuses,
                    [
                        (MobStat::Pad, Amount::Value(-20)),
                        (MobStat::Pdr, Amount::Value(-20))
                    ]
                );
                assert_eq!(inflict.duration, Duration::from_secs(120));
                Some((*skill_id, *level, *origin, mobs.clone()))
            }
            _ => None,
        });
        assert_eq!(
            debuff,
            Some((1201006, 20, Some((-120, 35)), vec![1000, 1001]))
        );
        // No buff for the page, but the MP is spent.
        assert!(!actions.iter().any(|action| matches!(action,
            WorldAction::Reply(p) if p.opcode() == SendOpcode::GiveBuff as i16)));
        assert_eq!(player.stats.mp, 80);
    }

    #[test]
    fn extras_follow_the_client_order() {
        let read = |skill_id, bytes: &[u8]| {
            let bytes = bytes.to_vec();
            read_extras(&mut Cursor::new(&bytes), skill_id)
        };
        // Dispel: where, the party, its delay, then the mobs.
        let extras = read(
            skills::DISPEL,
            &[1, 0, 2, 0, 0x80, 9, 0, 1, 7, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            extras,
            Extras {
                position: Some((1, 2)),
                party: Some(0x80),
                mobs: vec![7],
            }
        );
        // Seal sends only the mobs, and an out-of-sight 0 is dropped.
        let extras = read(2111004, &[2, 0, 0, 0, 0, 5, 0, 0, 0]);
        assert_eq!(extras.mobs, [5]);
        assert_eq!(extras.position, None);
    }

    #[test]
    fn a_party_skill_reaches_members_alive_in_its_area() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let bless = PartySkill {
            skill_id: 2301004,
            level: 20,
            slots: 0x30,
            heal: 0,
        };
        let mut player = beginner_with(1000, 1);
        player.stats.max_hp = 10_000;
        let sent = opcodes(party_skill_reached(
            &mut player,
            &bless,
            (0, 0),
            (350, -250),
            now,
        ));
        assert_eq!(
            sent,
            [
                SendOpcode::GiveBuff as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowForeignEffect as i16,
            ]
        );
        assert!(player
            .buffs
            .active()
            .iter()
            .all(|buff| buff.skill_id == 2301004 && buff.expires > now));
        assert!(!player.buffs.active().is_empty());

        // Past its area and the slack, or dead.
        let mut player = beginner_with(1000, 1);
        assert!(opcodes(party_skill_reached(
            &mut player,
            &bless,
            (0, 0),
            (401, 0),
            now
        ))
        .is_empty());
        assert!(opcodes(party_skill_reached(
            &mut player,
            &bless,
            (0, 0),
            (0, 301),
            now
        ))
        .is_empty());
        player.stats.hp = 0;
        assert!(opcodes(party_skill_reached(
            &mut player,
            &bless,
            (0, 0),
            (0, 0),
            now
        ))
        .is_empty());
        assert!(player.buffs.active().is_empty());

        let heal = PartySkill {
            skill_id: skills::HEAL,
            level: 30,
            slots: 0x30,
            heal: 500,
        };
        let mut player = beginner_with(1000, 1);
        player.stats.max_hp = 10_000;
        let sent = opcodes(party_skill_reached(&mut player, &heal, (0, 0), (0, 0), now));
        assert_eq!(
            sent,
            [
                SendOpcode::StatChange as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowForeignEffect as i16,
            ]
        );
        assert_eq!(player.stats.hp, 550);
    }

    #[test]
    fn chakra_heals_by_the_users_luk_with_its_mp() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(skills::CHAKRA, 30);
        player.stats.luk = 100;
        player.stats.mp = 100;
        player.stats.max_mp = 100;
        player.stats.max_hp = 5_000;
        // y 300: from 2.3 x 3 x 100 to 3.5 x 3 x 100, with the low end
        // truncated from 689.99..., as OdinMS-family servers' doubles do.
        assert_eq!(skills::chakra_heal(&player, 300, 0.0), 689);
        assert_eq!(skills::chakra_heal(&player, 300, 0.999_999), 1050);

        let actions = handle(&mut player, skills::CHAKRA, 30);
        assert!(unlocks(&actions[0]));
        assert!((50 + 689..=50 + 1050).contains(&player.stats.hp));
        assert_eq!(player.stats.mp, 100 - 27);
    }

    #[test]
    fn time_leap_ends_the_cooldowns_but_its_own() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(skills::TIME_LEAP, 1);
        player.stats.mp = 200;
        player.stats.max_mp = 200;
        player
            .buffs
            .start_cooldown(5121003, now, Duration::from_secs(60));
        let actions = handle(&mut player, skills::TIME_LEAP, 1);
        let cooldowns: Vec<_> = actions
            .iter()
            .filter_map(|action| match action {
                WorldAction::Reply(p) if p.opcode() == SendOpcode::Cooldown as i16 => {
                    Some(p.bytes[2..].to_vec())
                }
                _ => None,
            })
            .collect();
        let mut ended = 5121003i32.to_le_bytes().to_vec();
        ended.extend(0i16.to_le_bytes());
        let mut own = skills::TIME_LEAP.to_le_bytes().to_vec();
        own.extend(2940i16.to_le_bytes());
        assert_eq!(cooldowns, [own, ended]);
        assert!(!player.buffs.cooling_down(5121003, now));
        assert!(player.buffs.cooling_down(skills::TIME_LEAP, now));

        // It does the same for the party members it reaches.
        let mut member = beginner_with(1000, 1);
        member
            .buffs
            .start_cooldown(1002, now, Duration::from_secs(60));
        let leap = PartySkill {
            skill_id: skills::TIME_LEAP,
            level: 1,
            slots: 0x30,
            heal: 0,
        };
        let sent = opcodes(party_skill_reached(&mut member, &leap, (0, 0), (0, 0), now));
        assert_eq!(
            sent,
            [
                SendOpcode::Cooldown as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowForeignEffect as i16,
            ]
        );
        assert!(!member.buffs.cooling_down(1002, now));
    }

    fn diseased(player: &mut Player, stats: &[BuffStat], now: Instant) {
        for &stat in stats {
            let disease = game::Disease {
                stat,
                value: 1,
                skill_id: 120 | 1 << 16,
                lasts: Duration::from_secs(30),
            };
            crate::packet::handle::world::catch_disease(player, disease, now).unwrap();
        }
    }

    #[test]
    fn stun_stops_every_skill_and_seal_all_but_dispel() {
        let now = Instant::now();
        let mut player = beginner_with(skills::DISPEL, 1);
        assert!(!kept_from(&player, 1002));
        diseased(&mut player, &[BuffStat::Seal], now);
        assert!(kept_from(&player, 1002));
        assert!(!kept_from(&player, skills::DISPEL));
        diseased(&mut player, &[BuffStat::Stun], now);
        assert!(kept_from(&player, skills::DISPEL));
    }

    #[test]
    fn dispel_cures_by_its_chance_but_not_stun() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(1000, 1);
        diseased(
            &mut player,
            &[BuffStat::Stun, BuffStat::Poison, BuffStat::Slow],
            now,
        );
        // Level 1 cures one time in three.
        let weak = skills::at_level(skills::DISPEL, 1).unwrap();
        assert_eq!(weak.effect.prop, 34);
        let missed = dispel(&mut player, &weak, HandlerResult::empty(), 0.34).unwrap();
        assert!(missed.actions.is_empty());

        let cured = opcodes(dispel(&mut player, &weak, HandlerResult::empty(), 0.33));
        // Slow isn't shown to others, so only Poison's end is.
        assert_eq!(
            cured,
            [
                SendOpcode::CancelBuff as i16,
                SendOpcode::CancelForeignBuff as i16
            ]
        );
        assert!(player.buffs.has(BuffStat::Stun));
        assert!(!player.buffs.has(BuffStat::Poison) && !player.buffs.has(BuffStat::Slow));

        // A party member's Dispel cures too.
        diseased(&mut player, &[BuffStat::Darkness], now);
        let reached = PartySkill {
            skill_id: skills::DISPEL,
            level: 20,
            slots: 0x30,
            heal: 0,
        };
        let sent = opcodes(party_skill_reached(
            &mut player,
            &reached,
            (0, 0),
            (0, 0),
            now,
        ));
        assert_eq!(
            sent,
            [
                SendOpcode::CancelBuff as i16,
                SendOpcode::CancelForeignBuff as i16,
                SendOpcode::ShowItemGainInChat as i16,
                SendOpcode::ShowForeignEffect as i16,
            ]
        );
        assert!(!player.buffs.has(BuffStat::Darkness));
    }

    #[test]
    fn heros_will_cures_seduce_and_dispels_diseases() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let mut player = beginner_with(1121011, 1);
        player.stats.max_mp = 30;
        player.stats.mp = 30;
        diseased(
            &mut player,
            &[BuffStat::Seduce, BuffStat::Poison, BuffStat::StopPotion],
            now,
        );
        let sent: Vec<_> = handle(&mut player, 1121011, 1)
            .into_iter()
            .filter_map(|action| match action {
                WorldAction::Reply(packet) => Some(packet.opcode()),
                _ => None,
            })
            .collect();
        assert!(sent.contains(&(SendOpcode::CancelBuff as i16)));
        assert!(sent.contains(&(SendOpcode::Cooldown as i16)));
        assert!(!player.buffs.has(BuffStat::Seduce) && !player.buffs.has(BuffStat::Poison));
        assert!(player.buffs.has(BuffStat::StopPotion));
        assert_eq!(player.stats.mp, 0);
    }

    #[test]
    fn resurrection_raises_only_the_dead() {
        if skills::data().is_err() {
            return;
        }
        let now = Instant::now();
        let raise = PartySkill {
            skill_id: skills::RESURRECTION,
            level: 1,
            slots: 0x10,
            heal: 0,
        };
        let mut player = beginner_with(1000, 1);
        assert!(opcodes(party_skill_reached(
            &mut player,
            &raise,
            (0, 0),
            (0, 0),
            now
        ))
        .is_empty());
        assert_eq!(player.stats.hp, 50);

        player.stats.hp = 0;
        assert!(opcodes(party_skill_reached(
            &mut player,
            &raise,
            (0, 0),
            (501, 0),
            now
        ))
        .is_empty());
        let sent = opcodes(party_skill_reached(
            &mut player,
            &raise,
            (0, 0),
            (0, 0),
            now,
        ));
        assert_eq!(sent[0], SendOpcode::StatChange as i16);
        assert_eq!(player.stats.hp, player.stats.max_hp);
    }

    #[test]
    fn using_or_refusing_a_skill_unlocks_the_client() {
        let mut player = beginner_with(1000, 1);
        let actions = handle(&mut player, 1000, 1);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));

        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(1002, 2);
        assert!(unlocks(&handle(&mut player, 1002, 2)[0]));
    }

    fn magnet(
        player: &mut Player,
        level: u8,
        found: i32,
        mobs: &[(i32, bool)],
    ) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseSkill as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(1121001).unwrap();
        packet.write_byte(level).unwrap();
        packet.write_int(found).unwrap();
        for &(object_id, success) in mobs {
            packet.write_int(object_id).unwrap();
            packet.write_byte(u8::from(success)).unwrap();
        }
        packet.write_byte(1).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseSkillHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn hero_with_magnet() -> Player {
        let mut player = beginner_with(1121001, 30);
        player.stats.job = 112;
        player.stats.mp = 100;
        player.stats.max_mp = 100;
        player
    }

    #[test]
    fn monster_magnet_spends_mp_and_shows_the_mobs_it_reached() {
        if skills::data().is_err() {
            return;
        }
        let mut player = hero_with_magnet();
        let actions = magnet(&mut player, 30, 3, &[(101, true), (0, false), (102, false)]);
        let [unlock, WorldAction::Field(FieldRequest::Magnet { effect, caught })] =
            actions.as_slice()
        else {
            panic!("unexpected {actions:?}");
        };
        assert!(unlocks(unlock));
        assert_eq!(player.stats.mp, 79);
        assert_eq!(caught, &[(101, true), (102, false)]);
        assert_eq!(effect.opcode(), SendOpcode::ShowForeignEffect as i16);
        assert_eq!(effect.bytes.last(), Some(&1), "it ends with the facing");
    }

    #[test]
    fn monster_magnet_is_refused_at_another_level_or_past_15_mobs() {
        if skills::data().is_err() {
            return;
        }
        let mut player = hero_with_magnet();
        let actions = magnet(&mut player, 29, 0, &[]);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));
        let mobs: Vec<(i32, bool)> = (1..=16).map(|oid| (oid, false)).collect();
        let actions = magnet(&mut player, 30, 16, &mobs);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));
        assert_eq!(player.stats.mp, 100);
    }

    #[test]
    fn enrage_wants_nine_orbs_and_spends_them() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(combo::ENRAGE, 30);
        player.stats.job = 112;
        player.stats.mp = 100;
        player.stats.max_mp = 100;
        let combo_of = |value| {
            Buff::new(
                1111002,
                BuffStat::Combo,
                value,
                Instant::now(),
                Duration::from_secs(100),
            )
        };
        player.buffs.give(combo_of(9));
        let actions = handle(&mut player, combo::ENRAGE, 30);
        assert!(matches!(actions.as_slice(), [only] if unlocks(only)));
        assert_eq!(player.stats.mp, 100);

        player.buffs.give(combo_of(10));
        let actions = handle(&mut player, combo::ENRAGE, 30);
        assert_eq!(player.stats.mp, 60);
        assert_eq!(player.buffs.value(BuffStat::Pad), Some(26));
        assert_eq!(player.buffs.value(BuffStat::Combo), Some(1));
        // The others see the orbs go; the attack is the user's alone.
        let shown = |action: &WorldAction| {
            matches!(action, WorldAction::BuffsChanged(p)
                if p.opcode() == SendOpcode::GiveForeignBuff as i16)
        };
        assert_eq!(actions.iter().filter(|a| shown(a)).count(), 1);
    }

    fn prepare(player: &mut Player, skill_id: i32, level: u8) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::SkillEffect as i16).unwrap();
        packet.write_int(skill_id).unwrap();
        packet.write_byte(level).unwrap();
        packet.write_short(0x8016_u16 as i16).unwrap();
        packet.write_byte(4).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        SkillEffectHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn cancel(player: &mut Player, skill_id: i32) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::CancelBuff as i16).unwrap();
        packet.write_int(skill_id).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        CancelBuffHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn throw(player: &mut Player, skill_id: i32) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::ThrowGrenade as i16).unwrap();
        for value in [120, -45, 640, skill_id] {
            packet.write_int(value).unwrap();
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ThrowGrenadeHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    fn broadcast_opcodes(actions: &[WorldAction]) -> Vec<i16> {
        actions
            .iter()
            .map(|action| match action {
                WorldAction::Field(FieldRequest::Broadcast(p)) => p.opcode(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_prepared_skill_is_shown_to_the_map() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(3121004, 20);
        player.stats.job = 312;
        assert_eq!(
            broadcast_opcodes(&prepare(&mut player, 3121004, 20)),
            [SendOpcode::SkillEffect as i16]
        );
        assert!(prepare(&mut player, 3121004, 19).is_empty());
    }

    #[test]
    fn only_skills_that_prepare_are_shown() {
        let mut player = beginner_with(1002, 2);
        assert!(prepare(&mut player, 1002, 2).is_empty());
    }

    #[test]
    fn letting_go_of_a_held_skill_is_shown_to_the_map() {
        let mut player = beginner_with(1002, 2);
        assert_eq!(
            broadcast_opcodes(&cancel(&mut player, 5221004)),
            [SendOpcode::CancelSkillEffect as i16]
        );
        assert!(cancel(&mut player, 1002).is_empty());
    }

    #[test]
    fn a_throw_ends_the_prepare_and_is_shown_to_the_map() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(5201002, 10);
        player.stats.job = 520;
        assert_eq!(
            broadcast_opcodes(&throw(&mut player, 5201002)),
            [
                SendOpcode::CancelSkillEffect as i16,
                SendOpcode::ThrowGrenade as i16
            ]
        );
        player.stats.hp = 0;
        assert!(throw(&mut player, 5201002).is_empty());
    }

    #[test]
    fn only_a_learned_throw_is_shown() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with(5201002, 10);
        assert!(throw(&mut player, 14111006).is_empty());
        let mut player = beginner_with(5221004, 10);
        assert!(throw(&mut player, 5221004).is_empty());
    }
}
