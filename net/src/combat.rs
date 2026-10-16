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
    pub hits: Vec<AttackHit>,
}

/// CLOSE_RANGE_ATTACK body (after the opcode), as the v83 client sends it.
pub fn parse_close_attack(packet: &Packet) -> Result<Attack, NetworkError> {
    parse_attack(packet, AttackKind::Close)
}

/// An attack packet's body (after the opcode). RANGED_ATTACK and
/// MAGIC_ATTACK share CLOSE_RANGE_ATTACK's layout; a shot adds 5 bytes
/// before its targets, its direction among them, and 4 more for a few
/// skills.
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
    if kind == AttackKind::Ranged {
        r.read_bytes(9)?;
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

/// The first stack in USE that the player's weapon shoots: throwing stars
/// for a claw, arrows for a bow or crossbow, bullets for a gun.
pub fn ammo(player: &Player) -> Option<i32> {
    let weapon = player
        .inventory
        .get(InventoryType::Equipped, equip_slot::WEAPON)?
        .item_id;
    let fits = |item_id: i32| match weapon / 10_000 {
        145 => item_id / 1000 == 2060,
        146 => item_id / 1000 == 2061,
        147 => item_id / 10_000 == 207,
        149 => item_id / 10_000 == 233,
        _ => false,
    };
    player
        .inventory
        .items(InventoryType::Use)
        .find(|(_, item)| fits(item.item_id) && item.quantity > 0)
        .map(|(_, item)| item.item_id)
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
}

/// Basic attacks hit one mob once; each hit must stay under twice the base
/// maximum (room for the client's own rolls, never a multiple of it). A
/// skill attack takes its limits from the skill's level: its mob and hit
/// counts (a shot's bullets), and its fixed damage, its magic attack, or its
/// damage percent. A spell must be a skill. Returns the skill used, whose MP
/// the caller spends.
pub fn validate(attack: &Attack, player: &Player) -> Result<Option<Learned>, AttackRejection> {
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
            (max_magic_damage(player, effect.magic) * 2).max(10)
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
    Ok(skill)
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
            Ok(None)
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
        let used = validate(&snails(25), &player).unwrap().unwrap();
        assert_eq!(
            (used.skill_id, used.level, used.effect.mp_cost),
            (1000, 2, 5)
        );
        assert_eq!(
            validate(&snails(26), &player).unwrap_err(),
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
    /// shot's 5 bytes before its targets.
    fn kind_attack_packet(kind: AttackKind, skill: i32, damage: &[i32]) -> Packet {
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
            p.write_bytes(&[0, 0x80, 0, 0, 0, 0, 0, 0, 0]).unwrap();
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
            &kind_attack_packet(AttackKind::Ranged, 0, &[12]),
            AttackKind::Ranged,
        )
        .unwrap();
        assert_eq!(attack.kind, AttackKind::Ranged);
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
                &kind_attack_packet(AttackKind::Magic, 2001004, &[damage]),
                AttackKind::Magic,
            )
            .unwrap()
        };
        let mut player = beginner_with_sword();
        player.stats.int = 200;
        player.stats.mp = 100;
        learn(&mut player, 2001004, 20);
        // M = 200: ((40 + 200) / 30 + 1) * 55 = 495, twice that at most.
        assert_eq!(max_magic_damage(&player, 55), 495);
        assert!(validate(&bolt(990), &player).is_ok());
        assert_eq!(
            validate(&bolt(991), &player).unwrap_err(),
            AttackRejection::Damage
        );
        let unskilled = parse_attack(
            &kind_attack_packet(AttackKind::Magic, 0, &[1]),
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
                &kind_attack_packet(AttackKind::Ranged, 4001344, damage),
                AttackKind::Ranged,
            )
            .unwrap()
        };
        let mut player = beginner_with_sword();
        player.stats.mp = 100;
        learn(&mut player, 4001344, 20);
        assert!(validate(&seven(&[1, 1]), &player).is_ok());
        assert_eq!(
            validate(&seven(&[1, 1, 1]), &player).unwrap_err(),
            AttackRejection::TooManyTargets
        );
    }

    #[test]
    fn a_shot_broadcast_names_its_ammo() {
        let mut player = beginner_with_sword();
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1472000, EquipStats::default()),
        );
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2060000, 100));
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2070000, 100));
        assert_eq!(ammo(&player), Some(2070000));

        let attack = parse_attack(
            &kind_attack_packet(AttackKind::Ranged, 0, &[12]),
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
