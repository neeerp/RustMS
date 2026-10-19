//! Player attacks: parsing, damage limits, and the attack broadcast.

use crate::skills::{self, Learned};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::inventory::equip_slot;
use game::{BuffStat, InventoryType, Player};
use game_data::mobs::MobInfo;
use packet::{io::read::PktRead, io::write::PktWrite, Packet};
use std::io::Cursor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackHit {
    pub object_id: i32,
    pub damage: Vec<i32>,
}

/// Which of v83's three attack packets an attack came in, and goes out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackKind {
    /// A swing or stab, including a ranged weapon's with no ammo.
    Close,
    /// A shot with stars, arrows, or bullets.
    Ranged,
    /// A spell.
    Magic,
}

impl AttackKind {
    fn send_opcode(self) -> SendOpcode {
        match self {
            AttackKind::Close => SendOpcode::CloseRangeAttack,
            AttackKind::Ranged => SendOpcode::RangedAttack,
            AttackKind::Magic => SendOpcode::MagicAttack,
        }
    }
}

/// Arrow Bomb, whose hits take their percent from its `x`.
const ARROW_BOMB: i32 = 3101005;

/// Rapid Fire's targets end without the usual 4 bytes.
const RAPID_FIRE: i32 = 5221004;

/// Shadow Meso, which throws mesos, and the Shadow Partner that raises
/// what it costs.
const SHADOW_MESO: i32 = 4111004;
const SHADOW_PARTNER: i32 = 4111002;

/// Snipe, whose shot hits for 199,999 less a roll from 0 to 10,000
/// (FUN_0078df87).
const SNIPE: i32 = 3221007;
const SNIPE_MOST: i32 = 199_999;

/// Skills sent as shots that fire no ammo (the v83 client's FUN_007667ee).
const AMMO_FREE: [i32; 12] = [
    4111004,  // Shadow Meso
    4121003,  // Taunt
    4221003,  // Taunt
    5121002,  // Energy Orb
    11101004, // Soul Blade
    13101005, // Storm Break
    14101006, // Vampire
    15111006, // Spark
    15111007, // Shark Wave
    21100004, // Combo Smash
    21110004, // Combo Fenrir
    21120006, // Combo Tempest
];

/// A claw that throws arrows instead of stars.
const ARROW_CLAW: i32 = 1472063;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attack {
    pub kind: AttackKind,
    pub skill: i32,
    pub mob_count: u8,
    pub hit_count: u8,
    pub display: u8,
    /// The action, with the facing in the top bit (v83 0x952974: `action &
    /// 0x7fff | left << 15`).
    pub action: u16,
    pub speed: u8,
    /// How long a held skill was charged, in ms; 0 for the rest.
    pub charge: i32,
    /// The USE slot of the star, arrow, or bullet a shot fires; 0 for none.
    pub ammo_slot: i16,
    pub hits: Vec<AttackHit>,
}

/// CLOSE_RANGE_ATTACK body (after the opcode), as the v83 client sends it.
pub fn parse_close_attack(packet: &Packet) -> Result<Attack, NetworkError> {
    parse_attack(packet, AttackKind::Close)
}

/// An attack packet's body (after the opcode). RANGED_ATTACK and
/// MAGIC_ATTACK share CLOSE_RANGE_ATTACK's layout; a shot adds 5 bytes
/// before its targets (an int, then its ammo slot and its cash ammo slot
/// as shorts, then a byte where the others have an int). A held skill's
/// charge (an int) follows the 8 bytes after the skill (v83 0x95292e,
/// 0x955210, 0x956e66).
pub fn parse_attack(packet: &Packet, kind: AttackKind) -> Result<Attack, NetworkError> {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short()?;
    r.read_byte()?;
    let counts = r.read_byte()?;
    let skill = r.read_int()?;
    r.read_bytes(8)?;
    let sends_charge = if kind == AttackKind::Magic {
        skills::CHARGED_SPELLS.contains(&skill)
    } else {
        skills::keydown(skill)
    };
    let charge = if sends_charge { r.read_int()? } else { 0 };
    let display = r.read_byte()?;
    let action = r.read_short()? as u16;
    r.read_byte()?;
    let speed = r.read_byte()?;
    let mut ammo_slot = 0;
    if kind == AttackKind::Ranged {
        r.read_int()?;
        ammo_slot = r.read_short()?;
        r.read_short()?;
        r.read_byte()?;
    } else {
        r.read_bytes(4)?;
    }
    let (mob_count, hit_count) = (counts >> 4, counts & 0xF);
    let mut hits = Vec::new();
    for _ in 0..mob_count {
        let object_id = r.read_int()?;
        r.read_bytes(14)?;
        let damage = (0..hit_count)
            .map(|_| r.read_int())
            .collect::<Result<_, _>>()?;
        if skill != RAPID_FIRE {
            r.read_bytes(4)?;
        }
        hits.push(AttackHit { object_id, damage });
    }
    Ok(Attack {
        kind,
        skill,
        mob_count,
        hit_count,
        display,
        action,
        speed,
        charge,
        ammo_slot,
        hits,
    })
}

/// v83 weapon multipliers (`WeaponType` in HeavenMS) and which stats count.
fn weapon_formula(weapon_id: i32) -> (f64, bool) {
    // (multiplier, luck-based)
    match weapon_id / 10_000 {
        130 => (4.0, false),
        131 | 132 => (4.4, false),
        133 => (4.0, false),
        137 | 138 => (3.6, false),
        140 => (4.6, false),
        141 | 142 => (4.8, false),
        143 | 144 => (5.0, false),
        145 => (3.4, false),
        146 => (3.6, false),
        147 => (3.6, true),
        148 => (4.8, false),
        149 => (3.6, false),
        _ => (1.0, false),
    }
}

/// What a running buff adds to a stat, or 0.
fn buff(player: &Player, stat: BuffStat) -> i32 {
    player.buffs.value(stat).map_or(0, i32::from)
}

/// The most one basic hit can do (`calculateMaxBaseDamage`).
pub fn max_base_damage(player: &Player) -> i32 {
    let stats = &player.stats;
    let worn = |slot: i16| player.inventory.get(InventoryType::Equipped, slot);
    let watk: i32 = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .map(|equip| i32::from(equip.watk))
        .sum::<i32>()
        + buff(player, BuffStat::Pad);
    let weapon = worn(equip_slot::WEAPON).map_or(0, |item| item.item_id);
    let (multiplier, luck_based) = weapon_formula(weapon);
    let (main, secondary) = match weapon / 10_000 {
        145 | 146 | 149 => (stats.dex, stats.str),
        _ if luck_based => (stats.luk, stats.dex + stats.str),
        _ => (stats.str, stats.dex),
    };
    let base =
        (multiplier * f64::from(main) + f64::from(secondary)) / 100.0 * f64::from(watk.max(1));
    base.ceil() as i32
}

/// The most one hit of a spell with this magic attack (`mad`) can do: the
/// v83 formula ((M^2 / 1000 + M) / 30 + INT / 200) * mad, where M is the
/// player's magic, their INT plus their equipment's magic attack.
pub fn max_magic_damage(player: &Player, spell: i32) -> i32 {
    let (int, matk) = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .fold((i32::from(player.stats.int), 0), |(int, matk), equip| {
            (int + i32::from(equip.int), matk + i32::from(equip.matk))
        });
    let magic = f64::from(int + matk + buff(player, BuffStat::Mad));
    let max = ((magic * magic / 1000.0 + magic) / 30.0 + f64::from(int) / 200.0) * f64::from(spell);
    max.ceil() as i32
}

/// Whether `weapon` shoots `item_id`: throwing stars from a claw, arrows
/// from a bow, crossbow arrows from a crossbow, bullets from a gun.
fn shoots(weapon: i32, item_id: i32) -> bool {
    if weapon == ARROW_CLAW {
        return item_id / 1000 == 2060;
    }
    match weapon / 10_000 {
        145 => item_id / 1000 == 2060,
        146 => item_id / 1000 == 2061,
        147 => item_id / 10_000 == 207,
        149 => item_id / 10_000 == 233,
        _ => false,
    }
}

/// The ammo a shot uses up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ammo {
    pub slot: i16,
    pub item_id: i32,
    pub quantity: i16,
}

/// What a valid attack uses: its skill, whose MP the caller spends, a
/// shot's ammo, which the caller takes, and the mesos it throws.
#[derive(Clone, Debug, PartialEq)]
pub struct Uses {
    pub skill: Option<Learned>,
    pub ammo: Option<Ammo>,
    pub mesos: i32,
}

/// A shot fires the stack in the slot the client names, which the weapon
/// must shoot and which must hold the skill's `bulletConsume`, else its
/// `bulletCount`, else one. A few skills fire none.
fn shot_ammo(
    attack: &Attack,
    player: &Player,
    skill: Option<&Learned>,
) -> Result<Option<Ammo>, AttackRejection> {
    if attack.kind != AttackKind::Ranged || AMMO_FREE.contains(&attack.skill) {
        return Ok(None);
    }
    let weapon = player
        .inventory
        .get(InventoryType::Equipped, equip_slot::WEAPON)
        .ok_or(AttackRejection::Ammo)?
        .item_id;
    // Soul Arrow shoots a bow or crossbow without arrows.
    if matches!(weapon / 10_000, 145 | 146) && player.buffs.value(BuffStat::SoulArrow).is_some() {
        return Ok(None);
    }
    let quantity = skill.map_or(1, |skill| {
        let effect = &skill.effect;
        if effect.bullet_consume != 0 {
            effect.bullet_consume
        } else {
            effect.bullet_count
        }
    });
    let item = player
        .inventory
        .get(InventoryType::Use, attack.ammo_slot)
        .filter(|item| shoots(weapon, item.item_id) && i32::from(item.quantity) >= quantity)
        .ok_or(AttackRejection::Ammo)?;
    Ok(Some(Ammo {
        slot: attack.ammo_slot,
        item_id: item.item_id,
        quantity: quantity as i16,
    }))
}

/// Why an attack is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackRejection {
    Dead,
    /// A skill the player hasn't learned, or one that doesn't attack.
    Skill,
    NotEnoughMp,
    NotEnoughMesos,
    TooManyTargets,
    Damage,
    /// A shot without enough of an ammo its weapon fires.
    Ammo,
    /// Stun or Seduce stops every attack, Seal every skill.
    Diseased,
}

/// The MP a skill's attack spends: none for Heal, whose USE_SKILL paid.
pub fn attack_mp_cost(skill: &Learned) -> i32 {
    if skill.skill_id == skills::HEAL {
        0
    } else {
        skill.effect.mp_cost
    }
}

/// The mesos a skill's attack throws: its `moneyCon`, raised by Shadow
/// Partner's `y` percent while Shadow Partner is up (the v83 client's
/// FUN_00764256).
fn meso_cost(skill: &Learned, player: &Player) -> i32 {
    let cost = skill.effect.money_cost;
    if player.buffs.value(BuffStat::ShadowPartner).is_none() {
        return cost;
    }
    let raise = skills::learned(player, SHADOW_PARTNER).map_or(0, |partner| partner.effect.y);
    cost + raise * cost / 100
}

/// Basic attacks hit one mob once; each hit must stay under twice the base
/// maximum (room for the client's own rolls, never a multiple of it). A
/// skill attack takes its limits from the skill's level: its mob and hit
/// counts (a shot's bullets), and its fixed damage, its magic attack, its
/// damage percent, or the mesos it throws. A spell must be a skill, and a
/// shot needs ammo.
pub fn validate(attack: &Attack, player: &Player) -> Result<Uses, AttackRejection> {
    if player.stats.hp <= 0 {
        return Err(AttackRejection::Dead);
    }
    // v83 attacks only without Stun or Seduce, and skills fail under Seal
    // (FUN_0094be52, FUN_00966f7a).
    let buffs = &player.buffs;
    if buffs.has(BuffStat::Stun)
        || buffs.has(BuffStat::Seduce)
        || (attack.skill != 0 && buffs.has(BuffStat::Seal))
    {
        return Err(AttackRejection::Diseased);
    }
    let base_cap = (max_base_damage(player) * 2).max(10);
    let mut mesos = 0;
    let (skill, mob_count, hit_count, cap) = if attack.skill == 0 {
        if attack.kind == AttackKind::Magic {
            return Err(AttackRejection::Skill);
        }
        (None, 1, 1, base_cap)
    } else {
        let skill = skills::learned(player, attack.skill).ok_or(AttackRejection::Skill)?;
        let effect = &skill.effect;
        let cap = if effect.fixed_damage != 0 {
            effect.fixed_damage
        } else if effect.magic != 0 {
            // Twice the formula for the roll, Element Amplification, and half
            // again for a monster weak to the spell's element (FUN_00790782).
            let max = max_magic_damage(player, effect.magic) * skills::amplification(player) / 100;
            (max * 3).max(10)
        } else if skill.skill_id == skills::HEAL {
            // Heal hurts the undead most when it reaches only one, by the
            // highest roll, and half again on one weak to it.
            let max = skills::heal_amount(player, effect.hp, 1, 1.0)
                * f64::from(skills::amplification(player))
                / 100.0;
            ((max * 1.5).ceil() as i32).max(10)
        } else if skill.skill_id == SHADOW_MESO {
            // Ten times between half and one and a half its `moneyCon`,
            // raised by its `x` percent on a critical (FUN_0078df87).
            effect.money_cost * 3 / 2 * 10 * (100 + effect.x) / 100
        } else if skill.skill_id == SNIPE && attack.kind == AttackKind::Ranged {
            SNIPE_MOST
        } else if let Some(percent) = damage_percent(&skill) {
            (base_cap * percent / 100).max(10)
        } else {
            return Err(AttackRejection::Skill);
        };
        if i32::from(player.stats.mp) < attack_mp_cost(&skill) {
            return Err(AttackRejection::NotEnoughMp);
        }
        mesos = meso_cost(&skill, player);
        if player.meso < mesos {
            return Err(AttackRejection::NotEnoughMesos);
        }
        let hit_count = match attack.kind {
            AttackKind::Ranged => effect.bullet_count,
            _ => effect.attack_count,
        };
        let mob_count = effect.mob_count;
        (Some(skill), mob_count, hit_count, cap)
    };
    // Shadow Partner throws every star again, up to 15 lines in all, but
    // not for Taunt or Vampire (0x953b2a).
    let hit_count = if attack.kind == AttackKind::Ranged
        && player.buffs.value(BuffStat::ShadowPartner).is_some()
        && !matches!(attack.skill, 4121003 | 14101006)
    {
        (hit_count * 2).min(15)
    } else {
        hit_count
    };
    if i32::from(attack.mob_count) > mob_count
        || i32::from(attack.hit_count) > hit_count
        || attack.hits.len() > mob_count as usize
    {
        return Err(AttackRejection::TooManyTargets);
    }
    if attack
        .hits
        .iter()
        .flat_map(|hit| &hit.damage)
        .any(|&damage| !(0..=cap).contains(&damage))
    {
        return Err(AttackRejection::Damage);
    }
    let ammo = shot_ammo(attack, player, skill.as_ref())?;
    Ok(Uses { skill, ammo, mesos })
}

/// How much of the base damage a skill's hits deal, in percent, or `None`
/// for a skill that doesn't attack that way. Arrow Bomb, Disorder and Taunt
/// have no `damage`, which the client takes as 100%; Arrow Bomb scales its
/// hits by its `x` (FUN_0078df87, FUN_0075bf50).
fn damage_percent(skill: &Learned) -> Option<i32> {
    match (skill.effect.damage, skill.skill_id) {
        (0, ARROW_BOMB) => Some(skill.effect.x),
        (0, 4001002 | 14001002 | 4121003 | 4221003) => Some(100),
        (0, _) => None,
        (damage, _) => Some(damage),
    }
}

/// The attack as other players see it (`addAttackBody`), under the opcode
/// of its kind. A shot names the star, arrow, or bullet it fired.
pub fn build_attack(
    character_id: i32,
    attack: &Attack,
    skill_level: u8,
    projectile: i32,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(attack.kind.send_opcode() as i16)?;
    p.write_int(character_id)?;
    p.write_byte((attack.mob_count << 4) | attack.hit_count)?;
    p.write_byte(0x5B)?;
    p.write_byte(skill_level)?;
    if skill_level > 0 {
        p.write_int(attack.skill)?;
    }
    p.write_byte(attack.display)?;
    p.write_short(attack.action as i16)?;
    p.write_byte(attack.speed)?;
    p.write_byte(0x0A)?;
    p.write_int(projectile)?;
    for hit in &attack.hits {
        p.write_int(hit.object_id)?;
        p.write_byte(0)?;
        for damage in &hit.damage {
            p.write_int(*damage)?;
        }
    }
    if attack.kind == AttackKind::Ranged {
        p.write_int(0)?;
    }
    // A held spell's charge, after its targets (v83 0x98066e).
    if skills::CHARGED_SPELLS.contains(&attack.skill) {
        p.write_int(attack.charge)?;
    }
    Ok(p)
}

/// What a hit costs the player once their skills have taken their share.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    pub hp: i32,
    pub mp: i32,
    pub mesos: i32,
    /// The number the player's own client shows: the hit less the shares
    /// of all but Magic Guard (0x9593b8).
    pub shown: i32,
}

/// What a hit of `damage` takes from the player. A v83 client sends the
/// damage before these skills and leaves them to the server, working them
/// out only to show the number and to drink a pet's potion in time
/// (FUN_009581a9 at 0x959321 and 0x959595). Achilles always keeps its
/// share off; with `magic_guard`, Magic Guard moves its share to MP (as
/// much as there is, or all of it and no MP under Infinity); with
/// `meso_guard`, Meso Guard halves the hit for mesos.
pub fn taken(player: &Player, damage: i32, magic_guard: bool, meso_guard: bool) -> Taken {
    if damage <= 0 {
        return Taken {
            shown: damage,
            ..Taken::default()
        };
    }
    let achilles = achilles(player).map_or(0, |kept| (1000 - kept) * damage / 1000);
    let guarded = if magic_guard {
        buff(player, BuffStat::MagicGuard) * damage / 100
    } else {
        0
    };
    let (guarded, mp) = if player.buffs.has(BuffStat::Infinity) {
        (guarded, 0)
    } else {
        let guarded = guarded.min(i32::from(player.stats.mp));
        (guarded, guarded)
    };
    // Mesos pay `x` per 100 HP saved; short of them, they save what
    // they can (FUN_00792fa8).
    let (saved, mesos) = match buff(player, BuffStat::MesoGuard) {
        x if meso_guard && x > 0 => {
            let saved = damage / 2;
            let cost = saved * x / 100;
            if player.meso < cost {
                (player.meso * 100 / x, player.meso)
            } else {
                (saved, cost)
            }
        }
        _ => (0, 0),
    };
    Taken {
        hp: (damage - achilles - guarded - saved).max(0),
        mp,
        mesos,
        shown: damage - achilles - saved,
    }
}

impl Taken {
    /// The hit less what Power Guard sent back at the mob, which neither
    /// takes HP nor shows (0x959595).
    pub fn less(self, reflected: i32) -> Taken {
        Taken {
            hp: (self.hp - reflected).max(0),
            shown: self.shown - reflected,
            ..self
        }
    }
}

/// What Power Guard sends back at a mob that touched the player for
/// `damage` (FUN_009581a9 at 0x9587c6): its `x` percent, at most a tenth of
/// the mob's HP, halved for a boss, and at most a fixedDamage mob's fixed
/// damage. Nothing goes back at an invincible mob, or at one whose avoid
/// beats the player's accuracy, weighed down for each level the mob has on
/// them (0x9588cd).
pub fn power_guard(player: &Player, mob: &MobInfo, damage: i32) -> i32 {
    let percent = buff(player, BuffStat::PowerGuard);
    if percent <= 0 || damage <= 0 || mob.invincible {
        return 0;
    }
    let mut reflected = (damage * percent / 100).min(mob.max_hp / 10);
    if mob.boss {
        reflected /= 2;
    }
    if mob.fixed_damage > 0 {
        reflected = reflected.min(mob.fixed_damage);
    }
    let above = (i32::from(mob.level) - i32::from(player.stats.level)).max(0);
    let reach =
        f64::from(accuracy(player).clamp(0, 999)) * 100.0 / (f64::from(above) * 10.0 + 255.0);
    if f64::from(mob.avoid.clamp(0, 999)) > reach {
        return 0;
    }
    reflected
}

/// What Mana Reflection sends back at a mob whose magic attack hit the
/// player for `damage`, once the client's roll of its chance came up
/// (0x958d50): its `x` percent, at most a twentieth of the mob's HP.
/// Nothing goes back at a mob whose avoid beats 5 for each 10 of the
/// player's INT and of their LUK, weighed down for each level the mob has
/// on them and by 0.85. Unlike Power Guard's, it spares the player nothing.
pub fn mana_reflection(player: &Player, mob: &MobInfo, damage: i32) -> i32 {
    let level = buff(player, BuffStat::ManaReflection);
    if level <= 0 || damage <= 0 {
        return 0;
    }
    let Some(skill) = skills::at_level(skills::MANA_REFLECTION, level) else {
        return 0;
    };
    let reflected = (damage * skill.effect.x / 100).min(mob.max_hp / 20);
    let (int, luk, _) = skills::spell_stats(player);
    let above = (i32::from(mob.level) - i32::from(player.stats.level)).max(0);
    let reach =
        f64::from((int / 10 + luk / 10) * 5) * 100.0 / (f64::from(above) * 10.0 + 255.0) * 0.85;
    if f64::from(mob.avoid.clamp(0, 999)) > reach {
        return 0;
    }
    reflected
}

/// The player's accuracy: what they wear, 0.8 a point of DEX and 0.5 of
/// LUK, and their buffs'. Power Guard's Fighters and Pages have no skill
/// that adds to it.
fn accuracy(player: &Player) -> i32 {
    let (_, dex, luk) = skills::total_stats(player);
    let worn: i32 = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .map(|equip| i32::from(equip.acc))
        .sum();
    worn + (f64::from(dex) * 0.8 + f64::from(luk) * 0.5) as i32 + buff(player, BuffStat::Acc)
}

/// The per mille of a hit a Hero's, Paladin's, Dark Knight's, or Aran's
/// Achilles leaves, if they have it.
fn achilles(player: &Player) -> Option<i32> {
    let skill_id = match player.stats.job {
        112 => 1120004,
        122 => 1220005,
        132 => 1320005,
        2112 => 21120004,
        _ => return None,
    };
    skills::learned(player, skill_id).map(|skill| skill.effect.x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::{EquipStats, Item};

    fn attack_packet(object_id: i32, damage: i32) -> Packet {
        skill_attack_packet(0, object_id, damage)
    }

    fn skill_attack_packet(skill: i32, object_id: i32, damage: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0x2C).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(0x11).unwrap();
        p.write_int(skill).unwrap();
        p.write_bytes(&[0; 8]).unwrap();
        p.write_byte(3).unwrap(); // display
        p.write_short(0x8005u16 as i16).unwrap(); // action 5, facing left
        p.write_byte(0).unwrap();
        p.write_byte(4).unwrap(); // speed
        p.write_bytes(&[0; 4]).unwrap();
        p.write_int(object_id).unwrap();
        p.write_bytes(&[0; 14]).unwrap();
        p.write_int(damage).unwrap();
        p.write_bytes(&[0; 4]).unwrap();
        p
    }

    fn beginner_with_sword() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.str = 12;
        player.stats.dex = 5;
        player.stats.hp = 50;
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1302000,
                EquipStats {
                    watk: 17,
                    ..EquipStats::default()
                },
            ),
        );
        player
    }

    #[test]
    fn parses_a_basic_attack() {
        let attack = parse_close_attack(&attack_packet(2_000_000_001, 9)).unwrap();
        assert_eq!(
            (attack.mob_count, attack.hit_count, attack.skill),
            (1, 1, 0)
        );
        assert_eq!(
            attack.hits,
            [AttackHit {
                object_id: 2_000_000_001,
                damage: vec![9]
            }]
        );
        assert_eq!(
            (attack.display, attack.action, attack.speed),
            (3, 0x8005, 4)
        );
    }

    #[test]
    fn damage_is_capped_by_stats_and_weapon() {
        let player = beginner_with_sword();
        // ceil((4.0 * 12 + 5) / 100 * 17) = 10
        assert_eq!(max_base_damage(&player), 10);
        assert_eq!(
            validate(&parse_close_attack(&attack_packet(1, 20)).unwrap(), &player),
            Ok(Uses {
                skill: None,
                ammo: None,
                mesos: 0
            })
        );
        assert_eq!(
            validate(&parse_close_attack(&attack_packet(1, 21)).unwrap(), &player),
            Err(AttackRejection::Damage)
        );
        let mut dead = beginner_with_sword();
        dead.stats.hp = 0;
        assert_eq!(
            validate(&parse_close_attack(&attack_packet(1, 1)).unwrap(), &dead),
            Err(AttackRejection::Dead)
        );
    }

    #[test]
    fn stun_and_seduce_stop_attacks_and_seal_stops_skills() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let basic = parse_close_attack(&attack_packet(1, 1)).unwrap();
        let snails = parse_close_attack(&skill_attack_packet(1000, 1, 1)).unwrap();
        for (stat, skill_only) in [
            (BuffStat::Stun, false),
            (BuffStat::Seduce, false),
            (BuffStat::Seal, true),
        ] {
            let mut player = beginner_with_sword();
            player.buffs.give(game::Buff::new(-123, stat, 1, now, long));
            assert_eq!(validate(&basic, &player).is_ok(), skill_only, "{stat:?}");
            assert_eq!(
                validate(&snails, &player).unwrap_err(),
                AttackRejection::Diseased
            );
        }
    }

    #[test]
    fn three_snails_needs_the_skill_its_mp_and_its_fixed_damage() {
        if skills::data().is_err() {
            return;
        }
        let snails = |damage| parse_close_attack(&skill_attack_packet(1000, 1, damage)).unwrap();
        let mut player = beginner_with_sword();
        assert_eq!(
            validate(&snails(10), &player).unwrap_err(),
            AttackRejection::Skill
        );
        player.skills.set(game::SkillRecord {
            skill_id: 1000,
            level: 2,
            master_level: 0,
            expiration: None,
        });
        player.stats.mp = 4;
        assert_eq!(
            validate(&snails(10), &player).unwrap_err(),
            AttackRejection::NotEnoughMp
        );
        player.stats.mp = 5;
        // Level 2 does 25, more than twice a basic hit.
        let used = validate(&snails(25), &player).unwrap().skill.unwrap();
        assert_eq!(
            (used.skill_id, used.level, used.effect.mp_cost),
            (1000, 2, 5)
        );
        assert_eq!(
            validate(&snails(26), &player).unwrap_err(),
            AttackRejection::Damage
        );
        // v83 casts it, so it arrives as MAGIC_ATTACK.
        let cast = |damage| {
            parse_attack(
                &kind_attack_packet(AttackKind::Magic, 1000, 0, &[damage]),
                AttackKind::Magic,
            )
            .unwrap()
        };
        assert_eq!(
            validate(&cast(25), &player)
                .unwrap()
                .skill
                .unwrap()
                .skill_id,
            1000
        );
        assert_eq!(
            validate(&cast(26), &player).unwrap_err(),
            AttackRejection::Damage
        );
        // Recovery doesn't attack.
        player.skills.set(game::SkillRecord {
            skill_id: 1001,
            level: 1,
            master_level: 0,
            expiration: None,
        });
        let recovery = parse_close_attack(&skill_attack_packet(1001, 1, 1)).unwrap();
        assert_eq!(
            validate(&recovery, &player).unwrap_err(),
            AttackRejection::Skill
        );
    }

    /// A shot or spell at one mob: CLOSE_RANGE_ATTACK's layout, plus a
    /// shot's 5 bytes before its targets, with its ammo slot.
    fn kind_attack_packet(kind: AttackKind, skill: i32, ammo_slot: i16, damage: &[i32]) -> Packet {
        charged_attack_packet(kind, skill, None, ammo_slot, damage)
    }

    /// The same, with a held skill's charge.
    fn charged_attack_packet(
        kind: AttackKind,
        skill: i32,
        charge: Option<i32>,
        ammo_slot: i16,
        damage: &[i32],
    ) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0x2D).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(0x10 | damage.len() as u8).unwrap();
        p.write_int(skill).unwrap();
        p.write_bytes(&[0; 8]).unwrap();
        if let Some(charge) = charge {
            p.write_int(charge).unwrap();
        }
        p.write_byte(3).unwrap(); // display
        p.write_short(0x8005u16 as i16).unwrap(); // action 5, facing left
        p.write_byte(0).unwrap();
        p.write_byte(4).unwrap(); // speed
        if kind == AttackKind::Ranged {
            p.write_int(0).unwrap();
            p.write_short(ammo_slot).unwrap();
            p.write_short(0).unwrap();
            p.write_byte(0).unwrap();
        } else {
            p.write_bytes(&[0; 4]).unwrap();
        }
        p.write_int(77).unwrap();
        p.write_bytes(&[0; 14]).unwrap();
        for &d in damage {
            p.write_int(d).unwrap();
        }
        p.write_bytes(&[0; 4]).unwrap();
        p
    }

    fn learn(player: &mut Player, skill_id: i32, level: i32) {
        player.skills.set(game::SkillRecord {
            skill_id,
            level,
            master_level: 0,
            expiration: None,
        });
    }

    #[test]
    fn parses_a_shot() {
        let attack = parse_attack(
            &kind_attack_packet(AttackKind::Ranged, 0, 1, &[12]),
            AttackKind::Ranged,
        )
        .unwrap();
        assert_eq!((attack.kind, attack.ammo_slot), (AttackKind::Ranged, 1));
        assert_eq!(
            attack.hits,
            [AttackHit {
                object_id: 77,
                damage: vec![12]
            }]
        );
    }

    #[test]
    fn a_held_skill_says_how_long_it_was_charged() {
        let parse = |kind, skill, charge| {
            parse_attack(
                &charged_attack_packet(kind, skill, charge, 1, &[12, 13]),
                kind,
            )
            .unwrap()
        };
        // Piercing Arrow, Corkscrew Blow and Big Bang carry it.
        for (kind, skill) in [
            (AttackKind::Ranged, 3221001),
            (AttackKind::Close, 5101004),
            (AttackKind::Magic, 2221001),
        ] {
            let attack = parse(kind, skill, Some(640));
            assert_eq!((attack.charge, attack.display, attack.speed), (640, 3, 4));
            assert_eq!(attack.hits[0].damage, [12, 13]);
        }
        // So does Hurricane, at 0.
        let hurricane = parse(AttackKind::Ranged, 3121004, Some(0));
        assert_eq!((hurricane.ammo_slot, hurricane.hits[0].object_id), (1, 77));
        // Explosion and the rest don't.
        assert_eq!(parse(AttackKind::Magic, 2111002, None).charge, 0);
        assert_eq!(parse(AttackKind::Ranged, 3001004, None).ammo_slot, 1);
    }

    #[test]
    fn a_spell_is_capped_by_magic_and_needs_a_skill() {
        if skills::data().is_err() {
            return;
        }
        let bolt = |damage| {
            parse_attack(
                &kind_attack_packet(AttackKind::Magic, 2001004, 0, &[damage]),
                AttackKind::Magic,
            )
            .unwrap()
        };
        let mut player = beginner_with_sword();
        player.stats.int = 200;
        player.stats.mp = 100;
        learn(&mut player, 2001004, 20);
        // M = 200: ((40 + 200) / 30 + 1) * 55 = 495, twice that and half
        // again for a weak monster at most.
        assert_eq!(max_magic_damage(&player, 55), 495);
        assert!(validate(&bolt(1485), &player).is_ok());
        assert_eq!(
            validate(&bolt(1486), &player).unwrap_err(),
            AttackRejection::Damage
        );
        // Element Amplification at level 30 makes spells 140%, for a fire
        // and poison wizard and not a cleric.
        player.stats.job = 211;
        learn(&mut player, 2110001, 30);
        assert_eq!(skills::amplification(&player), 140);
        assert!(validate(&bolt(2079), &player).is_ok());
        assert_eq!(
            validate(&bolt(2080), &player).unwrap_err(),
            AttackRejection::Damage
        );
        player.stats.job = 230;
        assert_eq!(skills::amplification(&player), 100);
        let unskilled = parse_attack(
            &kind_attack_packet(AttackKind::Magic, 0, 0, &[1]),
            AttackKind::Magic,
        )
        .unwrap();
        assert_eq!(
            validate(&unskilled, &player).unwrap_err(),
            AttackRejection::Skill
        );
    }

    #[test]
    fn a_shot_skill_hits_once_per_bullet() {
        if skills::data().is_err() {
            return;
        }
        let seven = |damage: &[i32]| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 4001344, 1, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = rogue_with_stars(100);
        player.stats.mp = 100;
        learn(&mut player, 4001344, 20);
        assert!(validate(&seven(&[1, 1]), &player).is_ok());
        assert_eq!(
            validate(&seven(&[1, 1, 1]), &player).unwrap_err(),
            AttackRejection::TooManyTargets
        );
    }

    #[test]
    fn shadow_partner_doubles_a_shot_and_soul_arrow_needs_no_arrows() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let shot = |damage: &[i32], slot| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 0, slot, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = rogue_with_stars(100);
        assert_eq!(
            validate(&shot(&[1, 1], 1), &player).unwrap_err(),
            AttackRejection::TooManyTargets
        );
        let partner = game::Buff::new(4111002, BuffStat::ShadowPartner, 80, now, long);
        player.buffs.give(partner);
        assert!(validate(&shot(&[1, 1], 1), &player).is_ok());

        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1452002, EquipStats::default()),
        );
        assert_eq!(
            validate(&shot(&[1], 0), &player).unwrap_err(),
            AttackRejection::Ammo
        );
        let soul_arrow = game::Buff::new(3101004, BuffStat::SoulArrow, 1, now, long);
        player.buffs.give(soul_arrow);
        assert_eq!(validate(&shot(&[1], 0), &player).unwrap().ammo, None);
    }

    #[test]
    fn attack_buffs_raise_the_damage_cap() {
        let player = beginner_with_sword();
        let mut buffed = player.clone();
        let rage = game::Buff::new(
            1101006,
            BuffStat::Pad,
            12,
            std::time::Instant::now(),
            std::time::Duration::from_secs(60),
        );
        buffed.buffs.give(rage);
        assert!(max_base_damage(&buffed) > max_base_damage(&player));
    }

    #[test]
    fn arrow_bomb_hits_for_its_x_percent() {
        if skills::data().is_err() {
            return;
        }
        let bomb = |damage: &[i32]| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, ARROW_BOMB, 1, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1452002,
                EquipStats {
                    watk: 50,
                    ..EquipStats::default()
                },
            ),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2060000, 100));
        player.stats.dex = 200;
        player.stats.mp = 100;
        learn(&mut player, ARROW_BOMB, 30);
        // No `damage` at any level; level 30's x is 130.
        let cap = (max_base_damage(&player) * 2).max(10) * 130 / 100;
        assert!(validate(&bomb(&[cap]), &player).is_ok());
        assert_eq!(
            validate(&bomb(&[cap + 1]), &player).unwrap_err(),
            AttackRejection::Damage
        );
    }

    #[test]
    fn disorder_and_taunt_hit_for_the_base_cap() {
        if skills::data().is_err() {
            return;
        }
        let hit = |kind, skill, damage| {
            parse_attack(&kind_attack_packet(kind, skill, 1, &[damage]), kind).unwrap()
        };
        let mut player = beginner_with_sword();
        player.stats.mp = 100;
        learn(&mut player, 4001002, 20);
        learn(&mut player, 4121003, 30);
        let cap = (max_base_damage(&player) * 2).max(10);
        for (kind, skill) in [(AttackKind::Close, 4001002), (AttackKind::Ranged, 4121003)] {
            assert!(validate(&hit(kind, skill, cap), &player).is_ok());
            assert_eq!(
                validate(&hit(kind, skill, cap + 1), &player).unwrap_err(),
                AttackRejection::Damage
            );
        }
    }

    /// A claw, with `stars` Subis in USE slot 1.
    #[test]
    fn shadow_meso_costs_mesos_and_hits_by_them() {
        if skills::data().is_err() {
            return;
        }
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let toss = |damage: &[i32]| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, SHADOW_MESO, 0, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = rogue_with_stars(100);
        learn(&mut player, SHADOW_MESO, 30);
        // Level 30 throws 570 mesos and hits for at most 855 * 10, half
        // again on a critical.
        player.meso = 569;
        assert_eq!(
            validate(&toss(&[1]), &player).unwrap_err(),
            AttackRejection::NotEnoughMesos
        );
        player.meso = 570;
        let uses = validate(&toss(&[12825]), &player).unwrap();
        assert_eq!((uses.mesos, uses.ammo), (570, None));
        assert_eq!(
            validate(&toss(&[12826]), &player).unwrap_err(),
            AttackRejection::Damage
        );
        // Shadow Partner's level 30 raises the cost by its y, 50 percent.
        learn(&mut player, SHADOW_PARTNER, 30);
        let partner = game::Buff::new(SHADOW_PARTNER, BuffStat::ShadowPartner, 80, now, long);
        player.buffs.give(partner);
        assert_eq!(
            validate(&toss(&[1, 1]), &player).unwrap_err(),
            AttackRejection::NotEnoughMesos
        );
        player.meso = 855;
        assert_eq!(validate(&toss(&[1, 1]), &player).unwrap().mesos, 855);
    }

    #[test]
    fn snipe_hits_for_up_to_199999() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1462000, EquipStats::default()),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2061000, 10));
        learn(&mut player, SNIPE, 30);
        let snipe = |damage| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, SNIPE, 1, &[damage]),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let uses = validate(&snipe(199_999), &player).unwrap();
        assert_eq!(uses.ammo.map(|ammo| ammo.quantity), Some(1));
        assert_eq!(
            validate(&snipe(200_000), &player).unwrap_err(),
            AttackRejection::Damage
        );
    }

    fn rogue_with_stars(stars: i16) -> Player {
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1472000, EquipStats::default()),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2070000, stars));
        player
    }

    #[test]
    fn a_shot_uses_ammo_from_its_slot() {
        let mut player = rogue_with_stars(1);
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2060000, 100));
        let shot = |skill, slot| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, skill, slot, &[1]),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        assert_eq!(
            validate(&shot(0, 1), &player).unwrap(),
            Uses {
                skill: None,
                ammo: Some(Ammo {
                    slot: 1,
                    item_id: 2070000,
                    quantity: 1
                }),
                mesos: 0
            }
        );
        // Arrows don't fit a claw, and an empty slot holds nothing.
        for slot in [2, 3] {
            assert_eq!(
                validate(&shot(0, slot), &player).unwrap_err(),
                AttackRejection::Ammo
            );
        }
        // A spell or a swing needs none.
        let swing = parse_close_attack(&attack_packet(1, 1)).unwrap();
        assert_eq!(validate(&swing, &player).unwrap().ammo, None);

        if skills::data().is_err() {
            return;
        }
        // Lucky Seven throws two stars.
        player.stats.mp = 100;
        learn(&mut player, 4001344, 20);
        assert_eq!(
            validate(&shot(4001344, 1), &player).unwrap_err(),
            AttackRejection::Ammo
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2070000, 2));
        assert_eq!(
            validate(&shot(4001344, 1), &player)
                .unwrap()
                .ammo
                .unwrap()
                .quantity,
            2
        );
    }

    #[test]
    fn a_shot_broadcast_names_its_ammo() {
        let attack = parse_attack(
            &kind_attack_packet(AttackKind::Ranged, 0, 1, &[12]),
            AttackKind::Ranged,
        )
        .unwrap();
        let packet = build_attack(5, &attack, 0, 2070000).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::RangedAttack as i16)
            .int(5)
            .byte(0x11)
            .byte(0x5B)
            .byte(0);
        expect
            .byte(3)
            .short(0x8005u16 as i16)
            .byte(4)
            .byte(0x0A)
            .int(2070000);
        expect.int(77).byte(0).int(12).int(0);
        expect.end();
    }

    #[test]
    fn a_skill_broadcast_names_the_skill() {
        let attack = parse_close_attack(&skill_attack_packet(1000, 77, 25)).unwrap();
        let packet = build_attack(5, &attack, 2, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CloseRangeAttack as i16)
            .int(5)
            .byte(0x11)
            .byte(0x5B)
            .byte(2)
            .int(1000);
        expect
            .byte(3)
            .short(0x8005u16 as i16)
            .byte(4)
            .byte(0x0A)
            .int(0);
        expect.int(77).byte(0).int(25);
        expect.end();
    }

    #[test]
    fn a_held_spell_broadcast_ends_with_its_charge() {
        let attack = parse_attack(
            &charged_attack_packet(AttackKind::Magic, 2121001, Some(640), 0, &[12]),
            AttackKind::Magic,
        )
        .unwrap();
        let packet = build_attack(5, &attack, 30, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MagicAttack as i16)
            .int(5)
            .byte(0x11)
            .byte(0x5B)
            .byte(30)
            .int(2121001);
        expect
            .byte(3)
            .short(0x8005u16 as i16)
            .byte(4)
            .byte(0x0A)
            .int(0);
        expect.int(77).byte(0).int(12).int(640);
        expect.end();
    }

    #[test]
    fn the_broadcast_matches_add_attack_body() {
        let attack = parse_close_attack(&attack_packet(77, 9)).unwrap();
        let packet = build_attack(5, &attack, 0, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CloseRangeAttack as i16)
            .int(5)
            .byte(0x11)
            .byte(0x5B)
            .byte(0);
        expect
            .byte(3)
            .short(0x8005u16 as i16)
            .byte(4)
            .byte(0x0A)
            .int(0);
        expect.int(77).byte(0).int(9);
        expect.end();
    }
}

#[cfg(test)]
mod taken_tests {
    use super::*;
    use game::{Buff, SkillRecord};
    use std::time::{Duration, Instant};

    fn with_buff(player: &mut Player, skill_id: i32, stat: BuffStat, value: i16) {
        let lasts = Duration::from_secs(60);
        player
            .buffs
            .give(Buff::new(skill_id, stat, value, Instant::now(), lasts));
    }

    #[test]
    fn a_hit_with_no_guards_takes_it_all() {
        let player = crate::codec::stats::sample_player();
        assert_eq!(
            taken(&player, 100, true, true),
            Taken {
                hp: 100,
                mp: 0,
                mesos: 0,
                shown: 100
            }
        );
        // A miss or a dodge takes nothing.
        assert_eq!(taken(&player, -1, true, true).hp, 0);
    }

    #[test]
    fn magic_guard_moves_its_share_to_mp_while_there_is_any() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.mp = 50;
        with_buff(&mut player, 2001002, BuffStat::MagicGuard, 80);
        // 80 of 100 would go, but there are only 50 MP.
        assert_eq!(
            taken(&player, 100, true, true),
            Taken {
                hp: 50,
                mp: 50,
                mesos: 0,
                shown: 100
            }
        );
        assert_eq!(
            taken(&player, 50, true, true),
            Taken {
                hp: 10,
                mp: 40,
                mesos: 0,
                shown: 50
            }
        );
        assert_eq!(taken(&player, 50, false, true).hp, 50);
        // Under Infinity it takes all its share and no MP.
        with_buff(&mut player, 2121004, BuffStat::Infinity, 1);
        assert_eq!(
            taken(&player, 100, true, true),
            Taken {
                hp: 20,
                mp: 0,
                mesos: 0,
                shown: 100
            }
        );
    }

    #[test]
    fn meso_guard_halves_a_mobs_hit_for_mesos() {
        let mut player = crate::codec::stats::sample_player();
        player.meso = 1000;
        with_buff(&mut player, 4211005, BuffStat::MesoGuard, 78);
        // 50 saved at 78 mesos per 100.
        assert_eq!(
            taken(&player, 101, true, true),
            Taken {
                hp: 51,
                mp: 0,
                mesos: 39,
                shown: 51
            }
        );
        assert_eq!(taken(&player, 101, true, false).hp, 101);
        // Short of mesos it saves what they buy.
        player.meso = 20;
        assert_eq!(
            taken(&player, 100, true, true),
            Taken {
                hp: 75,
                mp: 0,
                mesos: 20,
                shown: 75
            }
        );
    }

    #[test]
    fn power_guard_sends_back_its_share_of_a_touch() {
        let mut player = crate::codec::stats::sample_player();
        // 80 + 2 accuracy from DEX and LUK.
        player.stats.dex = 100;
        let mob = MobInfo {
            level: 10,
            max_hp: 1000,
            avoid: 30,
            ..MobInfo::default()
        };
        assert_eq!(power_guard(&player, &mob, 100), 0);
        with_buff(&mut player, 1101007, BuffStat::PowerGuard, 30);
        assert_eq!(power_guard(&player, &mob, 100), 30);
        assert_eq!(power_guard(&player, &mob, -1), 0);
        // At most a tenth of the mob's HP, halved for a boss.
        let with = |change: fn(&mut MobInfo)| {
            let mut mob = mob.clone();
            change(&mut mob);
            power_guard(&player, &mob, 100)
        };
        assert_eq!(with(|mob| mob.max_hp = 200), 20);
        assert_eq!(with(|mob| mob.boss = true), 15);
        assert_eq!(with(|mob| mob.fixed_damage = 1), 1);
        assert_eq!(with(|mob| mob.invincible = true), 0);
        // 82 accuracy reaches 32 avoid at the player's level, 23 ten
        // levels below the mob.
        assert_eq!(with(|mob| mob.level = 20), 0);
        // The player is spared what went back.
        assert_eq!(
            taken(&player, 100, true, true).less(30),
            Taken {
                hp: 70,
                mp: 0,
                mesos: 0,
                shown: 70
            }
        );
    }

    #[test]
    fn mana_reflection_sends_back_its_share_of_a_spell() {
        if crate::skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        // 5 for each 10 of INT and of LUK: 50.
        player.stats.int = 100;
        let mob = MobInfo {
            level: 10,
            max_hp: 10000,
            avoid: 16,
            ..MobInfo::default()
        };
        assert_eq!(mana_reflection(&player, &mob, 100), 0);
        // Level 10 sends back all of it.
        with_buff(&mut player, 2121002, BuffStat::ManaReflection, 10);
        assert_eq!(mana_reflection(&player, &mob, 100), 100);
        assert_eq!(mana_reflection(&player, &mob, -1), 0);
        let with = |change: fn(&mut MobInfo)| {
            let mut mob = mob.clone();
            change(&mut mob);
            mana_reflection(&player, &mob, 100)
        };
        // At most a twentieth of the mob's HP, bosses alike.
        assert_eq!(with(|mob| mob.max_hp = 1000), 50);
        assert_eq!(with(|mob| mob.boss = true), 100);
        // 50 reaches 16 avoid at the player's level, but not 17, nor 16
        // two levels up.
        assert_eq!(with(|mob| mob.avoid = 17), 0);
        assert_eq!(with(|mob| mob.level = 11), 100);
        assert_eq!(with(|mob| mob.level = 12), 0);
    }

    #[test]
    fn achilles_keeps_its_share_off() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 112;
        player.skills.set(SkillRecord {
            skill_id: 1120004,
            level: 30,
            master_level: 30,
            expiration: None,
        });
        // A Hero at level 30 keeps 850 per mille.
        assert_eq!(
            taken(&player, 1000, true, true),
            Taken {
                hp: 850,
                mp: 0,
                mesos: 0,
                shown: 850
            }
        );
        // Another job's Achilles does nothing.
        player.stats.job = 122;
        assert_eq!(taken(&player, 1000, true, true).hp, 1000);
    }
}
