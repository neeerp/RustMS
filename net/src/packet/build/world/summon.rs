//! Summons (v83 CSummoned, report 158). Every packet starts with the
//! owner's character id: the client keeps a user's summons with the user.

use crate::summons::{Assist, Moves};
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// A summon as players see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SummonView {
    pub owner_id: i32,
    pub object_id: i32,
    pub skill_id: i32,
    pub owner_level: u8,
    pub level: u8,
    pub x: i16,
    pub y: i16,
    /// The move action: its stance times two, plus one facing left.
    pub stance: u8,
    pub moves: Moves,
    pub assist: Assist,
}

/// Why a summon went away (REMOVE_SUMMON's last byte, FUN_007a64eb).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Gone {
    /// Its time ran out: it fades, and its owner reads "...'s time has
    /// run out, and will disappear."
    Expired = 0,
    /// Without a trace, e.g. its owner left.
    Silently = 1,
    /// A Puppet was broken: it plays `die`.
    Broken = 2,
    /// Cancelled or replaced: it plays `die`.
    Dismissed = 4,
}

/// SPAWN_SUMMON (0xAF): with `appear`, it plays `summoned` first.
pub fn build_spawn_summon(summon: &SummonView, appear: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnSummon as i16)?;
    p.write_int(summon.owner_id)?;
    p.write_int(summon.object_id)?;
    p.write_int(summon.skill_id)?;
    p.write_byte(summon.owner_level)?;
    p.write_byte(summon.level)?;
    p.write_short(summon.x)?;
    p.write_short(summon.y)?;
    p.write_byte(summon.stance)?;
    p.write_short(0)?; // foothold: the client finds it
    p.write_byte(summon.moves as u8)?;
    p.write_byte(summon.assist as u8)?;
    p.write_byte(u8::from(!appear))?;
    Ok(p)
}

/// REMOVE_SUMMON (0xB0).
pub fn build_remove_summon(
    owner_id: i32,
    object_id: i32,
    why: Gone,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveSummon as i16)?;
    p.write_int(owner_id)?;
    p.write_int(object_id)?;
    p.write_byte(why as u8)?;
    Ok(p)
}

/// MOVE_SUMMON (0xB1): the path its owner's client sent, from `start`.
pub fn build_move_summon(
    owner_id: i32,
    object_id: i32,
    start: (i16, i16),
    movement: &[u8],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveSummon as i16)?;
    p.write_int(owner_id)?;
    p.write_int(object_id)?;
    p.write_short(start.0)?;
    p.write_short(start.1)?;
    p.write_bytes(movement)?;
    Ok(p)
}

/// SUMMON_ATTACK (0xB2, FUN_007a6882): the animation (with 0x80 facing
/// left), and each mob it hit with its hit action and the damage.
pub fn build_summon_attack(
    summon: &SummonView,
    action: u8,
    hits: &[crate::summons::SummonHit],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SummonAttack as i16)?;
    p.write_int(summon.owner_id)?;
    p.write_int(summon.object_id)?;
    p.write_byte(summon.owner_level)?;
    p.write_byte(action)?;
    p.write_byte(hits.len() as u8)?;
    for hit in hits {
        p.write_int(hit.object_id)?;
        p.write_byte(hit.action)?;
        p.write_int(hit.damage)?;
    }
    Ok(p)
}

/// DAMAGE_SUMMON (0xB4, FUN_007a6ebe): as its owner's client sent it.
pub fn build_damage_summon(
    owner_id: i32,
    damage: &crate::summons::SummonDamage,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DamageSummon as i16)?;
    p.write_int(owner_id)?;
    p.write_int(damage.object_id)?;
    p.write_byte(damage.attack as u8)?;
    p.write_int(damage.damage)?;
    if let Some((mob_id, left)) = damage.mob {
        p.write_int(mob_id)?;
        p.write_byte(u8::from(left))?;
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summons::SummonHit;

    fn hawk() -> SummonView {
        SummonView {
            owner_id: 7,
            object_id: 900_000_000,
            skill_id: 3111005,
            owner_level: 70,
            level: 30,
            x: -100,
            y: 200,
            stance: 1,
            moves: Moves::CircleFollow,
            assist: Assist::Attack,
        }
    }

    fn ints(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn a_summon_spawns_with_how_it_moves_and_acts() {
        let p = build_spawn_summon(&hawk(), true).unwrap();
        let mut want = vec![0xAF, 0x00];
        want.extend(ints(&[7, 900_000_000, 3111005]));
        want.extend([70, 30]);
        want.extend((-100i16).to_le_bytes());
        want.extend(200i16.to_le_bytes());
        want.extend([1, 0, 0, 3, 1, 0]);
        assert_eq!(p.bytes, want);
        assert_eq!(
            *build_spawn_summon(&hawk(), false)
                .unwrap()
                .bytes
                .last()
                .unwrap(),
            1
        );
    }

    #[test]
    fn removals_moves_attacks_and_damage_name_the_owner_and_summon() {
        let mut want = vec![0xB0, 0x00];
        want.extend(ints(&[7, 900_000_000]));
        want.push(2);
        assert_eq!(
            build_remove_summon(7, 900_000_000, Gone::Broken)
                .unwrap()
                .bytes,
            want
        );

        let p = build_move_summon(7, 900_000_000, (1, 2), &[1, 9]).unwrap();
        assert_eq!(&p.bytes[..2], [0xB1, 0x00]);
        assert_eq!(&p.bytes[10..], [1, 0, 2, 0, 1, 9]);

        let hit = SummonHit {
            object_id: 2_000_000_001,
            action: 6,
            damage: 500,
        };
        let p = build_summon_attack(&hawk(), 0x84, &[hit]).unwrap();
        let mut want = vec![0xB2, 0x00];
        want.extend(ints(&[7, 900_000_000]));
        want.extend([70, 0x84, 1]);
        want.extend(ints(&[2_000_000_001]));
        want.push(6);
        want.extend(ints(&[500]));
        assert_eq!(p.bytes, want);

        let touched = crate::summons::SummonDamage {
            object_id: 900_000_000,
            attack: -1,
            damage: 30,
            mob: Some((100100, false)),
        };
        let p = build_damage_summon(7, &touched).unwrap();
        let mut want = vec![0xB4, 0x00];
        want.extend(ints(&[7, 900_000_000]));
        want.push(0xFF);
        want.extend(ints(&[30, 100100]));
        want.push(0);
        assert_eq!(p.bytes, want);
    }
}
