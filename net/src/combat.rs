//! Player attacks: parsing, damage limits, and the attack broadcast.

use crate::beacon;
use crate::berserk;
use crate::charge;
use crate::combo;
use crate::energy;
use crate::meso_explosion::{self, Blast, Explosion, MESO_EXPLOSION};
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

/// Which of v83's attack packets an attack came in, and goes out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackKind {
    /// A swing or stab, including a ranged weapon's with no ammo.
    Close,
    /// A shot with stars, arrows, or bullets.
    Ranged,
    /// A spell.
    Magic,
    /// Energy Charge's hit on a mob that touches a player with a full
    /// gauge, laid out as a swing (FUN_0095f135).
    Body,
}

impl AttackKind {
    fn send_opcode(self) -> SendOpcode {
        match self {
            AttackKind::Close => SendOpcode::CloseRangeAttack,
            AttackKind::Ranged => SendOpcode::RangedAttack,
            AttackKind::Magic => SendOpcode::MagicAttack,
            AttackKind::Body => SendOpcode::EnergyAttack,
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

/// Stun Mastery, which crits on a stunned monster.
const STUN_MASTERY: i32 = 5110000;

/// Assassinate strikes twice from Dark Sight; the client marks the second
/// strike with bit 0x20 of `display` (v83 0x952956).
pub const ASSASSINATE: i32 = 4221001;
pub const SECOND_STRIKE: u8 = 0x20;

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

/// The display bit of a shot thrown under Shadow Stars.
const SHADOW_STARS_DISPLAY: u8 = 0x40;

/// A claw that throws arrows instead of stars.
const ARROW_CLAW: i32 = 1472063;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attack {
    pub kind: AttackKind,
    pub skill: i32,
    pub mob_count: u8,
    pub hit_count: u8,
    /// Flags: bit 0 marks a Final Attack after Slash Blast (0x952949), bit
    /// 0x20 Assassinate's second strike.
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
    /// What Meso Explosion sends after its targets.
    pub explosion: Option<Explosion>,
}

/// CLOSE_RANGE_ATTACK body (after the opcode), as the v83 client sends it.
pub fn parse_close_attack(packet: &Packet) -> Result<Attack, NetworkError> {
    parse_attack(packet, AttackKind::Close)
}

/// An attack packet's body (after the opcode). RANGED_ATTACK and
/// MAGIC_ATTACK share CLOSE_RANGE_ATTACK's layout; a shot adds 5 bytes
/// before its targets (an int, then its ammo slot and its cash ammo slot
/// as shorts, then a byte where the others have an int), and under Shadow
/// Stars the star it throws as an int (0x95514b). A held skill's
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
        if display & SHADOW_STARS_DISPLAY != 0 && !AMMO_FREE.contains(&skill) {
            r.read_int()?; // the star, which the slot holds too
        }
    } else {
        r.read_bytes(4)?;
    }
    let (mob_count, hit_count) = (counts >> 4, counts & 0xF);
    // Meso Explosion counts each target's hits where the others have a
    // short, and names the drops it sets off after where the player
    // stands (v83 0x96bafe).
    let explodes = kind == AttackKind::Close && skill == MESO_EXPLOSION;
    let mut hits = Vec::new();
    for _ in 0..mob_count {
        let object_id = r.read_int()?;
        let lines = if explodes {
            r.read_bytes(12)?;
            r.read_byte()?
        } else {
            r.read_bytes(14)?;
            hit_count
        };
        let damage = (0..lines).map(|_| r.read_int()).collect::<Result<_, _>>()?;
        if skill != RAPID_FIRE {
            r.read_bytes(4)?;
        }
        hits.push(AttackHit { object_id, damage });
    }
    let explosion = if explodes {
        let position = (r.read_short()?, r.read_short()?);
        let count = r.read_byte()?;
        let drops = (0..count)
            .map(|_| Ok((r.read_int()?, r.read_byte()?)))
            .collect::<Result<_, NetworkError>>()?;
        let delay = r.read_short()?;
        Some(Explosion {
            position,
            drops,
            delay,
        })
    } else {
        None
    };
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
        explosion,
    })
}

/// Lucky Seven and Triple Throw, which roll from LUK alone.
const LUCKY_SEVENS: [i32; 4] = [4001344, 4121007, 14001004, 14111005];

/// The weapon the player holds, or 0.
fn weapon(player: &Player) -> i32 {
    player
        .inventory
        .get(InventoryType::Equipped, equip_slot::WEAPON)
        .map_or(0, |item| item.item_id)
}

/// The most a hit's stats are worth with this weapon before its attack
/// (v83's switch at 0x78ef9c): picked by the weapon rather than the job, the
/// better of its swing and its stab, and LUK for a dagger only in a thief's
/// hands. Whacks and prone stabs do less.
fn weapon_stats(weapon: i32, job: i32, skill: i32, (str, dex, luk): (i32, i32, i32)) -> f64 {
    let (str, dex, luk) = (f64::from(str), f64::from(dex), f64::from(luk));
    let strong = |multiplier: f64| multiplier * str + dex;
    let nimble = |multiplier: f64| multiplier * dex + str;
    let lucky = |multiplier: f64| multiplier * luk + str + dex;
    match weapon / 10_000 {
        130 => strong(4.0),
        133 if job % 1000 / 100 == 4 => lucky(3.6),
        133 => strong(4.0),
        131 | 132 | 137 | 138 => strong(4.4),
        140 => strong(4.6),
        141 | 142 | 148 => strong(4.8),
        143 | 144 => strong(5.0),
        145 => nimble(3.4),
        146 | 149 => nimble(3.6),
        147 if LUCKY_SEVENS.contains(&skill) => 5.0 * luk,
        147 => lucky(3.6),
        // Bare hands.
        _ if job == 500 => strong(3.0),
        _ => strong(4.2),
    }
}

/// What a running buff adds to a stat, or 0, and to attack and accuracy
/// what a full Energy Charge does.
fn buff(player: &Player, stat: BuffStat) -> i32 {
    energy::raise(player, stat, player.buffs.value(stat).map_or(0, i32::from))
}

/// What Echo of Hero adds to a whole attack or magic: its percent, rounded
/// down (0x77e01c, 0x79167b).
fn echo_bonus(player: &Player, total: i32) -> i32 {
    total * buff(player, BuffStat::EchoOfHero).max(0) / 100
}

/// What an expert's mastery adds to attack with its weapon (SetFrom,
/// FUN_00764795): Bow Expert's, Marksman Boost's, or High Mastery's `x`.
fn expert_attack(player: &Player, weapon: i32) -> i32 {
    let experts: &[i32] = match weapon / 10_000 {
        144 => &[21120001],
        145 => &[3120005, 13110003],
        146 => &[3220004],
        _ => &[],
    };
    experts
        .iter()
        .filter_map(|&skill_id| skills::learned(player, skill_id))
        .map(|skill| skill.effect.x)
        .max()
        .unwrap_or(0)
}

/// What the star, arrow, or bullet a shot throws adds to its attack
/// (FUN_0077df48); a Gunslinger's capsules add nothing.
fn ammo_attack(attack: &Attack, player: &Player, weapon: i32) -> i32 {
    if attack.kind != AttackKind::Ranged {
        return 0;
    }
    player
        .inventory
        .get(InventoryType::Use, attack.ammo_slot)
        .filter(|item| shoots(weapon, item.item_id) && !matches!(item.item_id / 1000, 2331 | 2332))
        .and_then(|item| crate::items::info(item.item_id))
        .map_or(0, |info| info.pad)
}

/// What a pirate's bare hands add to attack (SetFrom, 0x77f89e): 10 and
/// 0.7 a level, at most 31.
fn bare_hands_attack(job: i32, level: i32) -> i32 {
    if job % 1000 / 100 != 5 {
        0
    } else if level > 30 {
        31
    } else {
        (f64::from(level) * 0.7 + 10.0) as i32
    }
}

/// The most one hit of an attack can do before its skill, its orbs, and a
/// critical (FUN_0078df87): the weapon's stats with what the player wears,
/// times the attack of their equipment, buffs, expert mastery, and the ammo
/// a shot throws, over 100; a pirate with no weapon has its bare hands'.
pub fn max_base_damage(player: &Player, attack: &Attack) -> i32 {
    let weapon = weapon(player);
    let job = i32::from(player.stats.job);
    let bare_hands = if weapon == 0 {
        bare_hands_attack(job, i32::from(player.stats.level))
    } else {
        0
    };
    let watk: i32 = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .map(|equip| i32::from(equip.watk))
        .sum::<i32>()
        + buff(player, BuffStat::Pad)
        + expert_attack(player, weapon)
        + ammo_attack(attack, player, weapon)
        + bare_hands;
    let watk = watk + echo_bonus(player, watk);
    let stats = weapon_stats(weapon, job, attack.skill, skills::total_stats(player));
    (stats / 100.0 * f64::from(watk.clamp(1, 1999))).ceil() as i32
}

/// The most a critical adds to a hit, in percent of it (FUN_0078df87): the
/// weapon's critical skill for a shot or a punch, or Assassinate's own
/// `criticalDamage` for its second strike, then Sharp Eyes as the server
/// gives it, then Stun Mastery's whole damage, as the monster may be
/// stunned.
fn critical_bonus(player: &Player, attack: &Attack) -> i32 {
    let shot = attack.kind == AttackKind::Ranged;
    let cygnus = player.stats.job / 1000 == 1;
    let source = match weapon(player) / 10_000 {
        145 | 146 if shot => {
            if cygnus {
                13000000
            } else {
                3000001
            }
        }
        147 if shot => {
            if cygnus {
                14100001
            } else {
                4100001
            }
        }
        148 => 15110000,
        _ => 0,
    };
    let mut damage = skills::learned(player, source).map_or(0, |skill| skill.effect.damage);
    if second_strike(attack) {
        damage =
            skills::learned(player, ASSASSINATE).map_or(0, |skill| skill.effect.critical_damage);
    }
    let sharp_eyes = buff(player, BuffStat::SharpEyes);
    if sharp_eyes > 0 {
        damage = if damage == 0 { 100 } else { damage } + (sharp_eyes & 0xFF);
    }
    damage += skills::learned(player, STUN_MASTERY).map_or(0, |skill| skill.effect.damage);
    (damage - 100).max(0)
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
    // The client caps magic at 1999 before and after Echo (0x79167b).
    let magic = (int + matk + buff(player, BuffStat::Mad)).clamp(0, 1999);
    let magic = f64::from((magic + echo_bonus(player, magic)).min(1999));
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

/// The ammo a shot throws, and how much of it it uses up: none under
/// Shadow Stars.
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
    /// The HP the skill costs, Sacrifice's share of its blow among it.
    pub hp: i32,
    /// The drops Meso Explosion sets off, and the damage each does.
    pub blast: Option<Blast>,
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
    // Shadow Stars throws its star from a claw, one of which the slot must
    // hold, and spends none (0x949b5d).
    if let Some(star) = skills::shadow_star(player).filter(|_| weapon / 10_000 == 147) {
        player
            .inventory
            .get(InventoryType::Use, attack.ammo_slot)
            .filter(|item| item.item_id == star && item.quantity >= 1)
            .ok_or(AttackRejection::Ammo)?;
        return Ok(Some(Ammo {
            slot: attack.ammo_slot,
            item_id: star,
            quantity: 0,
        }));
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
    NotEnoughHp,
    NotEnoughMp,
    NotEnoughMesos,
    TooManyTargets,
    Damage,
    /// A shot without enough of an ammo its weapon fires.
    Ammo,
    /// Stun or Seduce stops every attack, Seal every skill.
    Diseased,
    /// A finisher without an orb to spend.
    NoOrbs,
    /// An Energy skill without a full gauge.
    NotCharged,
    /// Morphed by an item, or by a skill that rules this attack out.
    Morphed,
}

/// Whether this is Assassinate's second strike.
pub fn second_strike(attack: &Attack) -> bool {
    attack.skill == ASSASSINATE && attack.display & SECOND_STRIKE != 0
}

/// How many times Assassinate multiplies its strikes at most: once more
/// for every 3 seconds in Dark Sight, up to its `time` (0x79028f).
fn assassinate_most(skill: &Learned) -> i32 {
    skill.effect.duration / 3 + 1
}

/// The MP a skill's attack spends: none for Heal, whose USE_SKILL paid,
/// for Final Attack, or for Assassinate's second strike, as the client
/// paid for both strikes on the key press (0x966f7a).
pub fn attack_mp_cost(player: &Player, skill: &Learned, attack: &Attack) -> i32 {
    if skill.skill_id == skills::HEAL
        || skills::FINAL_ATTACKS.contains(&skill.skill_id)
        || second_strike(attack)
    {
        0
    } else {
        skills::mp_cost(player, skill)
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
    // An item's morph can't attack (FUN_0094e256; Cosmic's
    // `isMorphWithoutAttack`), nor a skill's morph some skills.
    if buffs
        .value(BuffStat::Morph)
        .is_some_and(|morph| morph < 100)
        || crate::morph::refused(player, attack.skill)
    {
        return Err(AttackRejection::Morphed);
    }
    // v83 won't use a finisher without an orb (0x969623).
    if combo::finisher(attack.skill) && combo::orbs(player).unwrap_or(0) < 1 {
        return Err(AttackRejection::NoOrbs);
    }
    // Energy Charge hits only as the body attack, which nothing else is.
    if (attack.kind == AttackKind::Body) != energy::charges(attack.skill) {
        return Err(AttackRejection::Skill);
    }
    if energy::refused(player, attack.skill) || charge::refused(player, attack.skill) {
        return Err(AttackRejection::NotCharged);
    }
    // A hit is the base times the skill's percent and the orbs'
    // (FUN_007653fa), plus a critical's share of the base; twice that allows
    // for an element the monster is weak to.
    let base = i64::from(max_base_damage(player, attack));
    let critical = i64::from(critical_bonus(player, attack));
    let combo = match combo::damage_percent(player, attack.skill) {
        0 => 100,
        percent => i64::from(percent),
    };
    // A charge and Berserk raise every weapon hit.
    let charged = i64::from(charge::damage_percent(player));
    let berserk = i64::from(berserk::damage_percent(player));
    let cap_for = |percent: i32| {
        (2 * base * (i64::from(percent) * combo / 100 + critical) / 100 * charged / 100 * berserk
            / 100)
            .clamp(10, i64::from(i32::MAX)) as i32
    };
    let base_cap = cap_for(100);
    let mut mesos = 0;
    let mut hp = 0;
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
        } else if skill.skill_id == MESO_EXPLOSION {
            // Each line is worth less than 50·x (FUN_00791fbc).
            if attack.explosion.is_none() {
                return Err(AttackRejection::Skill);
            }
            50 * effect.x
        } else if skill.skill_id == ASSASSINATE {
            let percent = damage_percent(&skill).ok_or(AttackRejection::Skill)?;
            cap_for(percent).saturating_mul(assassinate_most(&skill))
        } else if let Some(percent) =
            charge::blow_percent(player, &skill).or_else(|| damage_percent(&skill))
        {
            let boost = skills::elemental_boost(player, skill.skill_id);
            cap_for(percent + boost.map_or(0, |boost| boost.effect.damage))
        } else {
            return Err(AttackRejection::Skill);
        };
        hp = skills::hp_cost(player, &skill).ok_or(AttackRejection::NotEnoughHp)?;
        if i32::from(player.stats.mp) < attack_mp_cost(player, &skill, attack) {
            return Err(AttackRejection::NotEnoughMp);
        }
        mesos = meso_cost(&skill, player);
        if player.meso < mesos {
            return Err(AttackRejection::NotEnoughMesos);
        }
        let hit_count = match attack.kind {
            AttackKind::Ranged => effect.bullet_count,
            // Assassinate's second strike hits once (0x950dc4).
            _ if second_strike(attack) => 1,
            _ => effect.attack_count,
        };
        // Final Attack hits one mob, but after Slash Blast as many as it
        // reaches.
        let mob_count =
            if skills::FINAL_ATTACKS.contains(&skill.skill_id) && attack.display & 1 != 0 {
                skills::learned(player, skills::slash_blast(player))
                    .map_or(1, |slash_blast| slash_blast.effect.mob_count)
            } else {
                effect.mob_count
            };
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
    // Bullseye raises every hit on its mark (FUN_0078dea6).
    if attack.hits.iter().any(|hit| {
        let percent = i64::from(beacon::damage_percent(player, hit.object_id));
        let cap = (i64::from(cap) * percent / 100).min(i64::from(i32::MAX)) as i32;
        (hit.damage.iter().enumerate())
            .any(|(line, &damage)| !(0..=barrage_cap(cap, attack.skill, line)).contains(&damage))
    }) {
        return Err(AttackRejection::Damage);
    }
    if let Some(sacrifice) = skill
        .as_ref()
        .filter(|skill| skill.skill_id == skills::SACRIFICE)
    {
        hp = sacrifice_cost(attack, sacrifice.effect.x, player.stats.hp);
    }
    let ammo = shot_ammo(attack, player, skill.as_ref())?;
    let blast = match (&attack.explosion, &skill) {
        (Some(explosion), Some(skill)) => Some(blast(attack, explosion, skill)?),
        _ => None,
    };
    Ok(Uses {
        skill,
        ammo,
        mesos,
        hp,
        blast,
    })
}

/// Barrage's fifth line does twice the others and its sixth four times
/// (FUN_0078df87, after Berserk).
fn barrage_cap(cap: i32, skill: i32, line: usize) -> i32 {
    match (skill, line) {
        (5121007 | 15111004, 4) => cap.saturating_mul(2),
        (5121007 | 15111004, 5) => cap.saturating_mul(4),
        _ => cap,
    }
}

/// Meso Explosion sets off as many drops as its `attackCount`, each once,
/// and each target's lines must come from the drops that hit it.
fn blast(
    attack: &Attack,
    explosion: &Explosion,
    skill: &Learned,
) -> Result<Blast, AttackRejection> {
    let effect = &skill.effect;
    let drops = &explosion.drops;
    let most = (effect.attack_count.max(1) as usize).min(meso_explosion::MOST_DROPS);
    let mut object_ids: Vec<i32> = drops.iter().map(|&(object_id, _)| object_id).collect();
    object_ids.sort_unstable();
    object_ids.dedup();
    if drops.len() > most || object_ids.len() < drops.len() {
        return Err(AttackRejection::TooManyTargets);
    }
    let lines: Vec<usize> = attack.hits.iter().map(|hit| hit.damage.len()).collect();
    let sources = meso_explosion::sources(drops, &lines).ok_or(AttackRejection::TooManyTargets)?;
    Ok(Blast {
        drops: drops.iter().map(|&(object_id, _)| object_id).collect(),
        delay: explosion.delay,
        lines: (attack.hits.iter().zip(sources))
            .map(|(hit, sources)| {
                sources
                    .into_iter()
                    .zip(hit.damage.iter().copied())
                    .collect()
            })
            .collect(),
        x: effect.x,
        lt: effect.lt,
        rb: effect.rb,
        left: attack.action & 0x8000 != 0,
    })
}

/// The HP Sacrifice costs: its `x` percent of its first blow, though never
/// the player's last HP. The v83 client leaves it to the server (its cost
/// check, FUN_00764256, knows only `hpCon` and Dragon Roar), so this is
/// Cosmic's.
fn sacrifice_cost(attack: &Attack, x: i32, hp: i16) -> i32 {
    let blow = attack.hits.first().and_then(|hit| hit.damage.first());
    let cost = blow.map_or(0, |&blow| i64::from(blow) * i64::from(x) / 100);
    cost.min(i64::from(hp) - 1).max(0) as i32
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
    // Bit 31 marks a critical (v83 0x980620). The client sends none, but
    // Snipe always is one (0x78e4d6).
    let critical = attack.kind == AttackKind::Ranged && attack.skill == SNIPE;
    for hit in &attack.hits {
        p.write_int(hit.object_id)?;
        p.write_byte(0)?;
        // Meso Explosion's targets each say how many hits they take
        // (0x980540).
        if attack.explosion.is_some() {
            p.write_byte(hit.damage.len() as u8)?;
        }
        for &damage in &hit.damage {
            p.write_int(if critical && damage > 0 {
                damage | i32::MIN
            } else {
                damage
            })?;
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

/// A Poison tick of `poison` less the player's resistance to poison, as a
/// v83 client works it out (FUN_00765a34, 0x94b075): Dragon Knights, F/P
/// Wizards, Priests and Blaze Wizards take their skill's `x` percent off;
/// I/L Wizards don't resist poison.
pub fn poison_resisted(player: &Player, poison: i32) -> i32 {
    let skill = match player.stats.job {
        131 | 132 => 1310000,
        211 | 212 => 2110000,
        231 | 232 => 2310000,
        1211 | 1212 => 12110000,
        _ => return poison,
    };
    let percent = skills::learned(player, skill).map_or(0, |skill| skill.effect.x);
    (100 - percent) * poison / 100
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
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        assert_eq!(max_base_damage(&player, &basic), 10);
        assert_eq!(
            validate(&parse_close_attack(&attack_packet(1, 20)).unwrap(), &player),
            Ok(Uses {
                skill: None,
                ammo: None,
                mesos: 0,
                hp: 0,
                blast: None
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
    fn berserk_doubles_the_cap_at_half_hp() {
        let mut player = beginner_with_sword();
        player.stats.job = 132;
        player.stats.max_hp = 1000;
        player.stats.hp = 501;
        player.skills.set(game::SkillRecord {
            skill_id: berserk::BERSERK,
            level: 30,
            master_level: 0,
            expiration: None,
        });
        let swing = |damage| parse_close_attack(&attack_packet(1, damage)).unwrap();
        assert!(validate(&swing(20), &player).is_ok());
        assert_eq!(validate(&swing(21), &player), Err(AttackRejection::Damage));
        player.stats.hp = 500;
        assert!(validate(&swing(40), &player).is_ok());
        assert_eq!(validate(&swing(41), &player), Err(AttackRejection::Damage));
    }

    #[test]
    fn combo_orbs_raise_the_cap_and_a_finisher_needs_one() {
        let mut player = beginner_with_sword();
        player.stats.job = 111;
        player.stats.mp = 100;
        learn(&mut player, 1111002, 30);
        learn(&mut player, 1111003, 30);
        let combo = |player: &mut Player, value| {
            let lasts = std::time::Duration::from_secs(60);
            let now = std::time::Instant::now();
            (player.buffs).give(game::Buff::new(1111002, BuffStat::Combo, value, now, lasts));
        };
        let check = |player: &Player, skill, damage| {
            validate(
                &parse_close_attack(&skill_attack_packet(skill, 1, damage)).unwrap(),
                player,
            )
            .map(|_| ())
        };
        // No orbs: 20 as before, and no Panic.
        combo(&mut player, 1);
        assert_eq!(check(&player, 0, 20), Ok(()));
        assert_eq!(check(&player, 0, 21), Err(AttackRejection::Damage));
        assert_eq!(check(&player, 1111003, 1), Err(AttackRejection::NoOrbs));
        // Five orbs: 140%, and Panic's 350% by 350%.
        combo(&mut player, 6);
        assert_eq!(check(&player, 0, 28), Ok(()));
        assert_eq!(check(&player, 0, 29), Err(AttackRejection::Damage));
        assert_eq!(check(&player, 1111003, 245), Ok(()));
        assert_eq!(check(&player, 1111003, 246), Err(AttackRejection::Damage));
    }

    #[test]
    fn skills_cost_hp_and_sacrifice_a_share_of_its_blow() {
        let mut player = beginner_with_sword();
        (player.stats.mp, player.stats.hp, player.stats.max_hp) = (100, 1000, 1000);
        learn(&mut player, 1001005, 1);
        learn(&mut player, skills::SACRIFICE, 30);
        learn(&mut player, skills::DRAGON_ROAR, 30);
        let hp = |player: &Player, skill, damage| {
            let attack = parse_close_attack(&skill_attack_packet(skill, 1, damage)).unwrap();
            validate(&attack, player).map(|uses| uses.hp)
        };
        // Slash Blast's hpCon, Dragon Roar's 30% of max HP, and Sacrifice's
        // 5% of its blow.
        assert_eq!(hp(&player, 1001005, 1), Ok(8));
        assert_eq!(hp(&player, skills::DRAGON_ROAR, 1), Ok(300));
        assert_eq!(hp(&player, skills::SACRIFICE, 60), Ok(3));
        player.stats.hp = 499;
        assert_eq!(
            hp(&player, skills::DRAGON_ROAR, 1),
            Err(AttackRejection::NotEnoughHp)
        );
        player.stats.hp = 8;
        assert_eq!(hp(&player, 1001005, 1), Err(AttackRejection::NotEnoughHp));
        player.stats.hp = 2;
        assert_eq!(hp(&player, skills::SACRIFICE, 60), Ok(1));
    }

    #[test]
    fn final_attack_is_free_and_follows_slash_blast_to_its_mobs() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        player.stats.mp = 0;
        learn(&mut player, 11101002, 30);
        learn(&mut player, 1100002, 30);
        let mut attack = parse_close_attack(&skill_attack_packet(11101002, 1, 1)).unwrap();
        attack.display = 0;
        // The Cygnus one has an `mpCon`, for its buff.
        assert!(validate(&attack, &player).is_ok());

        attack.skill = 1100002;
        attack.mob_count = 3;
        attack.hits = (1..=3)
            .map(|object_id| AttackHit {
                object_id,
                damage: vec![1],
            })
            .collect();
        let rejection = |attack: &Attack, player: &Player| validate(attack, player).err();
        assert_eq!(
            rejection(&attack, &player),
            Some(AttackRejection::TooManyTargets)
        );
        attack.display = 1;
        assert_eq!(
            rejection(&attack, &player),
            Some(AttackRejection::TooManyTargets)
        );
        learn(&mut player, 1001005, 1);
        assert_eq!(rejection(&attack, &player), None);
        attack.display = 0;
        assert_eq!(
            rejection(&attack, &player),
            Some(AttackRejection::TooManyTargets)
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
    fn an_item_morph_cannot_attack() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let swing = parse_close_attack(&attack_packet(1, 1)).unwrap();
        let mut player = beginner_with_sword();
        player
            .buffs
            .give(game::Buff::new(-2210000, BuffStat::Morph, 1, now, long));
        assert_eq!(validate(&swing, &player), Err(AttackRejection::Morphed));
        player
            .buffs
            .give(game::Buff::new(5111005, BuffStat::Morph, 1000, now, long));
        assert!(validate(&swing, &player).is_ok());
    }

    #[test]
    fn recoil_shot_is_a_plain_gun_shot() {
        if skills::data().is_err() || crate::items::data().is_err() {
            return;
        }
        // The client pushes the user back itself, in its movement path.
        let mut player = beginner_with_sword();
        player.stats.job = 520;
        player.stats.mp = 100;
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1492000, EquipStats::default()),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2330000, 10));
        learn(&mut player, 5201006, 20);
        let recoil = |damage: &[i32]| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 5201006, 1, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        assert!(validate(&recoil(&[1]), &player).is_ok());
        assert_eq!(
            validate(&recoil(&[1, 1]), &player).unwrap_err(),
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
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        assert!(max_base_damage(&buffed, &basic) > max_base_damage(&player, &basic));
    }

    #[test]
    fn a_pirate_hits_bare_handed_with_attack_of_its_own() {
        let mut player = crate::codec::stats::sample_player();
        (player.stats.str, player.stats.dex) = (12, 5);
        (player.stats.job, player.stats.level) = (500, 20);
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        // ceil((3.0 * 12 + 5) / 100 * (0.7 * 20 + 10)) = 10
        assert_eq!(max_base_damage(&player, &basic), 10);
        // ceil(41 / 100 * 31) = 13
        player.stats.level = 70;
        assert_eq!(max_base_damage(&player, &basic), 13);
        // ceil((4.2 * 12 + 5) / 100 * 1) = 1
        player.stats.job = 100;
        assert_eq!(max_base_damage(&player, &basic), 1);
    }

    #[test]
    fn echo_of_hero_raises_whole_attack_and_magic() {
        let mut player = crate::codec::stats::sample_player();
        (player.stats.str, player.stats.dex, player.stats.int) = (12, 5, 200);
        (player.stats.job, player.stats.level) = (500, 70);
        player.buffs.give(game::Buff::new(
            1005,
            BuffStat::EchoOfHero,
            4,
            std::time::Instant::now(),
            std::time::Duration::from_secs(60),
        ));
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        // Bare hands' 31 attack and 4% more is 32: ceil(41 / 100 * 32) = 14.
        assert_eq!(max_base_damage(&player, &basic), 14);
        // M = 208: ((43.264 + 208) / 30 + 1) * 55 = 515.65.
        assert_eq!(max_magic_damage(&player, 55), 516);
    }

    #[test]
    fn a_dagger_counts_luk_for_a_thief_only() {
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1332000,
                EquipStats {
                    watk: 27,
                    ..EquipStats::default()
                },
            ),
        );
        (player.stats.str, player.stats.dex, player.stats.luk) = (30, 60, 150);
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        // ceil((4.0 * 30 + 60) / 100 * 27) = 49
        assert_eq!(max_base_damage(&player, &basic), 49);
        // ceil((3.6 * 150 + 30 + 60) / 100 * 27) = 171; v83's window shows 170.
        player.stats.job = 420;
        assert_eq!(max_base_damage(&player, &basic), 171);
    }

    #[test]
    fn assassinate_multiplies_its_strikes_and_crits_on_the_second() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1332000,
                EquipStats {
                    watk: 27,
                    ..EquipStats::default()
                },
            ),
        );
        (player.stats.str, player.stats.dex, player.stats.luk) = (30, 60, 150);
        player.stats.job = 422;
        player.stats.mp = 40;
        learn(&mut player, ASSASSINATE, 30);
        let strike = |player: &Player, display, damage| {
            let mut attack =
                parse_close_attack(&skill_attack_packet(ASSASSINATE, 1, damage)).unwrap();
            attack.display = display;
            validate(&attack, player)
                .map(|uses| attack_mp_cost(player, &uses.skill.unwrap(), &attack))
        };
        // Twice 171 at 600%, five times over for 12 seconds in Dark Sight.
        assert_eq!(strike(&player, 0, 10_260), Ok(40));
        assert_eq!(strike(&player, 0, 10_261), Err(AttackRejection::Damage));
        // The second strike may crit for 250% and costs nothing.
        player.stats.mp = 0;
        assert_eq!(strike(&player, SECOND_STRIKE, 12_825), Ok(0));
        assert_eq!(
            strike(&player, SECOND_STRIKE, 12_826),
            Err(AttackRejection::Damage)
        );
        assert_eq!(strike(&player, 0, 1), Err(AttackRejection::NotEnoughMp));
        let mut twice = parse_close_attack(&skill_attack_packet(ASSASSINATE, 1, 1)).unwrap();
        (twice.display, twice.hit_count) = (SECOND_STRIKE, 2);
        assert_eq!(
            validate(&twice, &player).err(),
            Some(AttackRejection::TooManyTargets)
        );
    }

    #[test]
    fn a_shot_adds_its_ammo_and_an_expert_its_x() {
        if skills::data().is_err() || crate::items::data().is_err() {
            return;
        }
        let shot = |slot| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 0, slot, &[1]),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = rogue_with_stars(100);
        player.stats.luk = 100;
        // A claw of no attack throws Subis' 15: ceil((3.6 * 100 + 12 + 5) / 100 * 15) = 57.
        assert_eq!(max_base_damage(&player, &shot(1)), 57);
        // A whack throws nothing, and counts at least 1 attack.
        let whack = parse_close_attack(&attack_packet(1, 0)).unwrap();
        assert_eq!(max_base_damage(&player, &whack), 4);

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
        player.stats.dex = 100;
        // ceil((3.4 * 100 + 12) / 100 * 50) = 176
        assert_eq!(max_base_damage(&player, &shot(1)), 176);
        // Bow Expert's level 30 adds 10: ceil(352 / 100 * 60) = 212.
        learn(&mut player, 3120005, 30);
        assert_eq!(max_base_damage(&player, &shot(1)), 212);
    }

    #[test]
    fn barrage_ends_with_twice_and_four_times_the_cap() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        player.stats.job = 512;
        learn(&mut player, 5121007, 30);
        let basic = parse_close_attack(&attack_packet(1, 0)).unwrap();
        // Level 30 does 230%.
        let cap = 2 * max_base_damage(&player, &basic) * 230 / 100;
        let barrage = |lines: &[i32]| {
            let packet = kind_attack_packet(AttackKind::Close, 5121007, 0, lines);
            validate(&parse_attack(&packet, AttackKind::Close).unwrap(), &player).map(|_| ())
        };
        let (c, double, quadruple) = (cap, 2 * cap, 4 * cap);
        assert_eq!(barrage(&[c, c, c, c, double, quadruple]), Ok(()));
        assert_eq!(
            barrage(&[c, c, c, c, double, quadruple + 1]),
            Err(AttackRejection::Damage)
        );
        assert_eq!(
            barrage(&[c, c, c, double, c, c]),
            Err(AttackRejection::Damage)
        );
    }

    #[test]
    fn criticals_raise_the_cap_by_their_bonus() {
        if skills::data().is_err() || crate::items::data().is_err() {
            return;
        }
        let shot = |player: &Player, damage| {
            let attack = parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 0, 1, &[damage]),
                AttackKind::Ranged,
            )
            .unwrap();
            validate(&attack, player).map(|_| ())
        };
        let mut player = rogue_with_stars(100);
        player.stats.luk = 100;
        // Twice the base of 57.
        assert_eq!(shot(&player, 114), Ok(()));
        assert_eq!(shot(&player, 115), Err(AttackRejection::Damage));
        // Critical Throw's level 30 crits for 200%: twice 57 * (100 + 100) / 100.
        learn(&mut player, 4100001, 30);
        assert_eq!(shot(&player, 228), Ok(()));
        assert_eq!(shot(&player, 229), Err(AttackRejection::Damage));
        // Sharp Eyes adds its low byte as the server gives it: 57 * 2 * (100 + 240) / 100.
        let sharp_eyes = game::Buff::new(
            3121002,
            BuffStat::SharpEyes,
            (30 << 8 | 140) as i16,
            std::time::Instant::now(),
            std::time::Duration::from_secs(60),
        );
        player.buffs.give(sharp_eyes);
        assert_eq!(shot(&player, 387), Ok(()));
        assert_eq!(shot(&player, 388), Err(AttackRejection::Damage));
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
        let cap = (max_base_damage(&player, &bomb(&[0])) * 2).max(10) * 130 / 100;
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
        let cap = (max_base_damage(&player, &hit(AttackKind::Close, 4001002, 0)) * 2).max(10);
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
    fn elemental_boost_raises_flamethrowers_cap() {
        if skills::data().is_err() || crate::items::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        player.stats.job = 521;
        player.stats.mp = 100;
        player.stats.dex = 200;
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(
                1492000,
                EquipStats {
                    watk: 50,
                    ..EquipStats::default()
                },
            ),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2331000, 10));
        learn(&mut player, 5211004, 30);
        let flame = |damage| {
            parse_attack(
                &kind_attack_packet(AttackKind::Ranged, 5211004, 1, &[damage]),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let base = max_base_damage(&player, &flame(1));
        let check = |player: &Player, damage| validate(&flame(damage), player).map(|_| ());
        // Level 30 does 160%, and 360% with Elemental Boost's 200.
        let (alone, boosted) = (2 * base * 160 / 100, 2 * base * 360 / 100);
        assert_eq!(check(&player, alone), Ok(()));
        assert_eq!(check(&player, alone + 1), Err(AttackRejection::Damage));
        learn(&mut player, skills::ELEMENTAL_BOOST, 30);
        assert_eq!(check(&player, boosted), Ok(()));
        assert_eq!(check(&player, boosted + 1), Err(AttackRejection::Damage));
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

    #[test]
    fn others_see_a_snipe_as_critical() {
        let attack = parse_attack(
            &kind_attack_packet(AttackKind::Ranged, SNIPE, 1, &[150_000]),
            AttackKind::Ranged,
        )
        .unwrap();
        let packet = build_attack(5, &attack, 30, 2060000).unwrap();
        let damage = i32::from_le_bytes(
            packet.bytes[packet.bytes.len() - 8..][..4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(damage, 150_000 | i32::MIN);
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
    fn shadow_stars_throws_its_star_for_nothing() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let mut player = rogue_with_stars(1);
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2070006, 1));
        let ilbis = game::Buff::new(skills::SHADOW_STARS, BuffStat::ShadowStars, 7, now, long);
        player.buffs.give(ilbis);
        // Display 0x40, and the star after the shot's 5 bytes.
        let shot = |slot| {
            let mut p = kind_attack_packet(AttackKind::Ranged, 0, slot, &[1]);
            p.bytes[16] |= SHADOW_STARS_DISPLAY;
            p.bytes.splice(30..30, 2070006i32.to_le_bytes());
            parse_attack(&p, AttackKind::Ranged).unwrap()
        };
        let attack = shot(2);
        assert_eq!(
            (attack.hits[0].object_id, &attack.hits[0].damage[..]),
            (77, &[1][..])
        );
        let ammo = Ammo {
            slot: 2,
            item_id: 2070006,
            quantity: 0,
        };
        assert_eq!(validate(&attack, &player).unwrap().ammo, Some(ammo));
        // The slot must hold the buff's star.
        assert_eq!(
            validate(&shot(1), &player).unwrap_err(),
            AttackRejection::Ammo
        );
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
                mesos: 0,
                hp: 0,
                blast: None
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

    /// Meso Explosion's CLOSE_RANGE_ATTACK: each target's lines, counted,
    /// then where the player stands, the drops with their masks, and when
    /// the hits land.
    fn explosion_packet(lines: &[&[i32]], drops: &[(i32, u8)]) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0x2C).unwrap();
        p.write_byte(0).unwrap();
        let most = lines.iter().map(|lines| lines.len()).max().unwrap_or(0) as u8;
        p.write_byte((lines.len() as u8) << 4 | most).unwrap();
        p.write_int(MESO_EXPLOSION).unwrap();
        p.write_bytes(&[0; 8]).unwrap();
        p.write_byte(0).unwrap();
        p.write_short(0x801Fu16 as i16).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(4).unwrap();
        p.write_bytes(&[0; 4]).unwrap();
        for (target, lines) in lines.iter().enumerate() {
            p.write_int(100 + target as i32).unwrap();
            p.write_bytes(&[0; 12]).unwrap();
            p.write_byte(lines.len() as u8).unwrap();
            for &line in *lines {
                p.write_int(line).unwrap();
            }
            p.write_bytes(&[0; 4]).unwrap();
        }
        p.write_short(-40).unwrap();
        p.write_short(85).unwrap();
        p.write_byte(drops.len() as u8).unwrap();
        for &(object_id, mask) in drops {
            p.write_int(object_id).unwrap();
            p.write_byte(mask).unwrap();
        }
        p.write_short(420).unwrap();
        p
    }

    #[test]
    fn meso_explosion_names_its_drops_after_its_targets() {
        let packet = explosion_packet(&[&[10, 20], &[30]], &[(7, 0b01), (8, 0b11), (9, 0)]);
        let attack = parse_close_attack(&packet).unwrap();
        assert_eq!((attack.mob_count, attack.hit_count), (2, 2));
        assert_eq!(attack.hits[0].damage, [10, 20]);
        assert_eq!(
            (attack.hits[1].object_id, &attack.hits[1].damage[..]),
            (101, &[30][..])
        );
        assert_eq!(
            attack.explosion,
            Some(Explosion {
                position: (-40, 85),
                drops: vec![(7, 0b01), (8, 0b11), (9, 0)],
                delay: 420,
            })
        );
        // The others hear how many hits each target takes.
        let packet = build_attack(5, &attack, 30, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::CloseRangeAttack as i16)
            .int(5)
            .byte(0x22)
            .byte(0x5B)
            .byte(30)
            .int(MESO_EXPLOSION);
        expect
            .byte(0)
            .short(0x801Fu16 as i16)
            .byte(4)
            .byte(0x0A)
            .int(0);
        expect.int(100).byte(0).byte(2).int(10).int(20);
        expect.int(101).byte(0).byte(1).int(30);
        expect.end();
    }

    #[test]
    fn meso_explosion_sets_off_each_drop_once_and_hits_by_its_x() {
        if skills::data().is_err() {
            return;
        }
        let mut player = beginner_with_sword();
        learn(&mut player, MESO_EXPLOSION, 30);
        player.stats.mp = 30;
        let check_as = |player: &Player, lines: &[&[i32]], drops: &[(i32, u8)]| {
            validate(
                &parse_close_attack(&explosion_packet(lines, drops)).unwrap(),
                player,
            )
        };
        let check = |lines: &[&[i32]], drops: &[(i32, u8)]| check_as(&player, lines, drops);
        let blast = check(&[&[50_000, 1], &[2]], &[(7, 0b01), (8, 0b11)])
            .unwrap()
            .blast
            .unwrap();
        assert_eq!(blast.drops, [7, 8]);
        assert_eq!(blast.lines, [vec![(0, 50_000), (1, 1)], vec![(1, 2)]]);
        assert_eq!(
            (blast.x, blast.lt, blast.rb, blast.left),
            (1000, (-300, -120), (0, 50), true)
        );
        // With nothing to hit, it still sets off its drops.
        assert!(check(&[], &[(7, 0)]).unwrap().blast.is_some());
        // No line deals 50·x or more, each drop goes off once, as many as
        // its attackCount, and each target's lines match its drops.
        assert_eq!(check(&[&[50_001]], &[(7, 1)]), Err(AttackRejection::Damage));
        assert_eq!(
            check(&[], &[(7, 0), (7, 0)]),
            Err(AttackRejection::TooManyTargets)
        );
        let many: Vec<(i32, u8)> = (0..21).map(|object_id| (object_id, 0)).collect();
        assert_eq!(check(&[], &many), Err(AttackRejection::TooManyTargets));
        assert!(check(&[], &many[..20]).is_ok());
        assert_eq!(
            check(&[&[1, 1]], &[(7, 1)]),
            Err(AttackRejection::TooManyTargets)
        );
        let mut tired = player.clone();
        tired.stats.mp = 29;
        assert_eq!(
            check_as(&tired, &[], &[]),
            Err(AttackRejection::NotEnoughMp)
        );
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

    #[test]
    fn resistance_takes_its_share_off_poison() {
        if skills::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 212;
        player.skills.set(SkillRecord {
            skill_id: 2110000,
            level: 20,
            master_level: 0,
            expiration: None,
        });
        // An F/P Archmage's Partial Resistance 20 keeps 70% off.
        assert_eq!(poison_resisted(&player, 1000), 300);
        // An I/L Wizard has none for poison, nor anyone without the skill.
        player.stats.job = 222;
        assert_eq!(poison_resisted(&player, 1000), 1000);
        player.stats.job = 131;
        assert_eq!(poison_resisted(&player, 1000), 1000);
    }
}
