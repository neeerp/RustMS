//! Player attacks: parsing, damage limits, and the attack broadcast.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attack {
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
    r.read_bytes(4)?;
    let (mob_count, hit_count) = (counts >> 4, counts & 0xF);
    let mut hits = Vec::new();
    for _ in 0..mob_count {
        let object_id = r.read_int()?;
        r.read_bytes(14)?;
        let damage = (0..hit_count)
            .map(|_| r.read_int())
            .collect::<Result<_, _>>()?;
        r.read_bytes(4)?;
        hits.push(AttackHit { object_id, damage });
    }
    Ok(Attack {
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

/// Why an attack is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackRejection {
    Dead,
    /// Skills need a skill system first.
    Skill,
    TooManyTargets,
    Damage,
}

/// Basic attacks hit one mob once; each hit must stay under twice the base
/// maximum (room for the client's own rolls, never a multiple of it).
pub fn validate(attack: &Attack, player: &Player) -> Result<(), AttackRejection> {
    if player.stats.hp <= 0 {
        return Err(AttackRejection::Dead);
    }
    if attack.skill != 0 {
        return Err(AttackRejection::Skill);
    }
    if attack.mob_count > 1 || attack.hit_count > 1 || attack.hits.len() > 1 {
        return Err(AttackRejection::TooManyTargets);
    }
    let cap = (max_base_damage(player) * 2).max(10);
    if attack
        .hits
        .iter()
        .flat_map(|hit| &hit.damage)
        .any(|&damage| !(0..=cap).contains(&damage))
    {
        return Err(AttackRejection::Damage);
    }
    Ok(())
}

/// CLOSE_RANGE_ATTACK as other players see it (`addAttackBody`).
pub fn build_close_attack(character_id: i32, attack: &Attack) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CloseRangeAttack as i16)?;
    p.write_int(character_id)?;
    p.write_byte((attack.mob_count << 4) | attack.hit_count)?;
    p.write_byte(0x5B)?;
    p.write_byte(0)?; // skill level
    p.write_byte(attack.display)?;
    p.write_byte(attack.direction)?;
    p.write_byte(attack.stance)?;
    p.write_byte(attack.speed)?;
    p.write_byte(0x0A)?;
    p.write_int(0)?; // projectile
    for hit in &attack.hits {
        p.write_int(hit.object_id)?;
        p.write_byte(0)?;
        for damage in &hit.damage {
            p.write_int(*damage)?;
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::{EquipStats, Item};

    fn attack_packet(object_id: i32, damage: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(0x2C).unwrap();
        p.write_byte(0).unwrap();
        p.write_byte(0x11).unwrap();
        p.write_int(0).unwrap();
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
            Ok(())
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
    fn the_broadcast_matches_add_attack_body() {
        let attack = parse_close_attack(&attack_packet(77, 9)).unwrap();
        let packet = build_close_attack(5, &attack).unwrap();
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
