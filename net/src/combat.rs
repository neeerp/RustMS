//! Player attacks: parsing, damage limits, and the attack broadcast.

use crate::skills::{self, Learned};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::inventory::equip_slot;
use game::{InventoryType, Player};
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

/// Skills whose shot carries 4 more bytes before its targets.
const LONG_SHOTS: [i32; 4] = [
    3121004,  // Hurricane
    3221001,  // Piercing Arrow
    5221004,  // Rapid Fire
    13111002, // Hurricane (Wind Archer)
];

/// Rapid Fire's targets end without the usual 4 bytes.
const RAPID_FIRE: i32 = 5221004;

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
    pub direction: u8,
    pub stance: u8,
    pub speed: u8,
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
/// as shorts, then a byte where the others have an int), and 4 more for a
/// few skills.
pub fn parse_attack(packet: &Packet, kind: AttackKind) -> Result<Attack, NetworkError> {
    let mut r = Cursor::new(&packet.bytes);
    r.read_short()?;
    r.read_byte()?;
    let counts = r.read_byte()?;
    let skill = r.read_int()?;
    r.read_bytes(8)?;
    let display = r.read_byte()?;
    let direction = r.read_byte()?;
    let stance = r.read_byte()?;
    r.read_byte()?;
    let speed = r.read_byte()?;
    let mut ammo_slot = 0;
    if kind == AttackKind::Ranged {
        r.read_int()?;
        ammo_slot = r.read_short()?;
        r.read_short()?;
        r.read_byte()?;
        if LONG_SHOTS.contains(&skill) {
            r.read_bytes(4)?;
        }
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
        direction,
        stance,
        speed,
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

/// The most one basic hit can do (`calculateMaxBaseDamage`).
pub fn max_base_damage(player: &Player) -> i32 {
    let stats = &player.stats;
    let worn = |slot: i16| player.inventory.get(InventoryType::Equipped, slot);
    let watk: i32 = player
        .inventory
        .items(InventoryType::Equipped)
        .filter_map(|(_, item)| item.equip.as_ref())
        .map(|equip| i32::from(equip.watk))
        .sum();
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
    let magic = f64::from(int + matk);
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

/// What a valid attack uses: its skill, whose MP the caller spends, and a
/// shot's ammo, which the caller takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Uses {
    pub skill: Option<Learned>,
    pub ammo: Option<Ammo>,
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
    TooManyTargets,
    Damage,
    /// A shot without enough of an ammo its weapon fires.
    Ammo,
}

/// Basic attacks hit one mob once; each hit must stay under twice the base
/// maximum (room for the client's own rolls, never a multiple of it). A
/// skill attack takes its limits from the skill's level: its mob and hit
/// counts (a shot's bullets), and its fixed damage, its magic attack, or its
/// damage percent. A spell must be a skill, and a shot needs ammo.
pub fn validate(attack: &Attack, player: &Player) -> Result<Uses, AttackRejection> {
    if player.stats.hp <= 0 {
        return Err(AttackRejection::Dead);
    }
    let base_cap = (max_base_damage(player) * 2).max(10);
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
        } else if effect.damage != 0 {
            (base_cap * effect.damage / 100).max(10)
        } else {
            return Err(AttackRejection::Skill);
        };
        if i32::from(player.stats.mp) < effect.mp_cost {
            return Err(AttackRejection::NotEnoughMp);
        }
        let hit_count = match attack.kind {
            AttackKind::Ranged => effect.bullet_count,
            _ => effect.attack_count,
        };
        let mob_count = effect.mob_count;
        (Some(skill), mob_count, hit_count, cap)
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
    Ok(Uses { skill, ammo })
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
    p.write_byte(attack.direction)?;
    p.write_byte(attack.stance)?;
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
    Ok(p)
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
        p.write_byte(0x80).unwrap(); // direction
        p.write_byte(5).unwrap(); // stance
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
            (
                attack.display,
                attack.direction,
                attack.stance,
                attack.speed
            ),
            (3, 0x80, 5, 4)
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
                ammo: None
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
        let mut p = Packet::new_empty();
        p.write_short(0x2D).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(0x10 | damage.len() as u8).unwrap();
        p.write_int(skill).unwrap();
        p.write_bytes(&[0; 8]).unwrap();
        p.write_byte(3).unwrap(); // display
        p.write_byte(0x80).unwrap(); // direction
        p.write_byte(5).unwrap(); // stance
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

    /// A claw, with `stars` Subis in USE slot 1.
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
                })
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
            .byte(0x80)
            .byte(5)
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
        expect.byte(3).byte(0x80).byte(5).byte(4).byte(0x0A).int(0);
        expect.int(77).byte(0).int(25);
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
        expect.byte(3).byte(0x80).byte(5).byte(4).byte(0x0A).int(0);
        expect.int(77).byte(0).int(9);
        expect.end();
    }
}
