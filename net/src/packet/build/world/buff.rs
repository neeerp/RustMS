//! Buffs, skill cooldowns, and skill effects.

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::BuffStat;
use packet::{io::write::PktWrite, Packet};

/// A temporary stat's bit in the client's 128-bit flags, as the two longs
/// the packet carries (first, second). The flags go out as they lie in
/// memory, with bits 0..31 in the last dword and 96..127 in the first.
fn mask(bit: u8) -> (u64, u64) {
    match bit {
        0..=31 => (0, 1 << (32 + bit)),
        32..=63 => (0, 1 << (bit - 32)),
        64..=95 => (1 << (bit - 32), 0),
        _ => (1 << (bit - 96), 0),
    }
}

/// The two-state stats (energy charge, dash, riding, ...), bits 82..88.
/// A player always carries them; their data has a fixed size.
pub(crate) const TWO_STATE: [u8; 7] = [82, 83, 84, 85, 86, 87, 88];

fn write_mask(p: &mut Packet, bits: impl IntoIterator<Item = u8>) -> Result<(), NetworkError> {
    let (first, second) = bits.into_iter().fold((0, 0), |(first, second), bit| {
        let (f, s) = mask(bit);
        (first | f, second | s)
    });
    p.write_long(first as i64)?;
    p.write_long(second as i64)?;
    Ok(())
}

/// Stats the client reads one more byte for at the end of GIVE_BUFF
/// (FUN_0077dc78): the ones that change how the player moves.
fn moves(stat: BuffStat) -> bool {
    matches!(
        stat,
        BuffStat::Speed | BuffStat::Jump | BuffStat::MapleWarrior
    )
}

/// GIVE_BUFF: the stats a skill gives, lasting `duration_ms`
/// (FUN_00a202be). Each stat's value, skill, and duration come in the
/// order of their bits.
pub fn build_give_buff(
    skill_id: i32,
    duration_ms: i32,
    stats: &[(BuffStat, i16)],
) -> Result<Packet, NetworkError> {
    let mut stats = stats.to_vec();
    stats.sort_by_key(|&(stat, _)| stat.bit());
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, stats.iter().map(|&(stat, _)| stat.bit()))?;
    for &(_, value) in &stats {
        p.write_short(value)?;
        p.write_int(skill_id)?;
        p.write_int(duration_ms)?;
    }
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    p.write_short(0)?; // delay
    if stats.iter().any(|&(stat, _)| moves(stat)) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// CANCEL_BUFF: these buffs ended.
pub fn build_cancel_buff(stats: &[BuffStat]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CancelBuff as i16)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    p.write_byte(1)?;
    Ok(p)
}

/// The buffs another player shows (FUN_00788156), as SPAWN_PLAYER and
/// GIVE_FOREIGN_BUFF carry them: the flags, then each one's data in the
/// client's fixed order, then the defense attack and state. SPAWN_PLAYER
/// also sets the two-state stats, whose data the caller writes next.
pub(crate) fn write_shown(
    p: &mut Packet,
    shown: &[(BuffStat, i32)],
    two_state: bool,
) -> Result<(), NetworkError> {
    let two_state = two_state.then_some(TWO_STATE).into_iter().flatten();
    write_mask(p, shown.iter().map(|(stat, _)| stat.bit()).chain(two_state))?;
    for stat in BuffStat::SHOWN {
        let Some(&(_, value)) = shown.iter().find(|(s, _)| *s == stat) else {
            continue;
        };
        match stat {
            BuffStat::Speed | BuffStat::Combo => {
                p.write_byte(value as u8)?;
            }
            BuffStat::WeaponCharge => {
                p.write_int(value)?;
            }
            _ => {}
        }
    }
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    Ok(())
}

/// GIVE_FOREIGN_BUFF: another player started showing these buffs
/// (0x98385d); the client leaves the others as they were.
pub fn build_give_foreign_buff(
    character_id: i32,
    shown: &[(BuffStat, i32)],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_shown(&mut p, shown, false)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// CANCEL_FOREIGN_BUFF: another player's shown buffs ended (0x983921).
pub fn build_cancel_foreign_buff(
    character_id: i32,
    stats: &[BuffStat],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CancelForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    Ok(p)
}

/// COOLDOWN: the skill can't be used for `seconds`.
pub fn build_cooldown(skill_id: i32, seconds: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::Cooldown as i16)?;
    p.write_int(skill_id)?;
    p.write_short(seconds)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT 1: another player used a skill.
pub fn build_skill_effect(
    character_id: i32,
    skill_id: i32,
    character_level: u8,
    skill_level: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(1)?;
    p.write_int(skill_id)?;
    p.write_byte(character_level)?;
    p.write_byte(skill_level)?;
    Ok(p)
}

/// The skill effect of a Monster Magnet, which also says which way the
/// player faced (0x937b42).
pub fn build_magnet_effect(
    character_id: i32,
    skill_id: i32,
    character_level: u8,
    skill_level: u8,
    toleft: bool,
) -> Result<Packet, NetworkError> {
    let mut p = build_skill_effect(character_id, skill_id, character_level, skill_level)?;
    p.write_byte(u8::from(toleft))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn give_buff_matches_the_client_masks() {
        let packet = build_give_buff(1002, 4000, &[(BuffStat::Speed, 10)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(0x80_0000_0000);
        expect.short(10).int(1002).int(4000);
        // Defense, delay, and the byte for a stat that changes movement.
        expect.byte(0).byte(0).short(0).byte(0);
        expect.end();
    }

    #[test]
    fn give_buff_writes_stats_in_bit_order() {
        // Hyper Body, given out of order; neither changes movement.
        let stats = [(BuffStat::MaxMp, 60), (BuffStat::MaxHp, 50)];
        let packet = build_give_buff(1301007, 300_000, &stats).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(0x6000_0000_0000);
        expect.short(50).int(1301007).int(300_000);
        expect.short(60).int(1301007).int(300_000);
        expect.byte(0).byte(0).short(0);
        expect.end();
    }

    #[test]
    fn masks_cover_all_four_dwords() {
        assert_eq!(mask(0), (0, 1 << 32));
        assert_eq!(mask(34), (0, 4));
        assert_eq!(mask(82), (1 << 50, 0));
        assert_eq!(mask(100), (1 << 4, 0));
    }

    #[test]
    fn foreign_buffs_carry_shown_data_in_the_client_order() {
        let shown = [
            (BuffStat::SoulArrow, 1),
            (BuffStat::WeaponCharge, 1211004),
            (BuffStat::Speed, 40),
        ];
        let packet = build_give_foreign_buff(7, &shown).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(0).long((1 << 39) | (1 << 54) | (1 << 48));
        expect.byte(40).int(1211004);
        expect.byte(0).byte(0).short(0);
        expect.end();

        let packet = build_cancel_foreign_buff(7, &[BuffStat::DarkSight]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelForeignBuff as i16).int(7);
        expect.long(0).long(1 << 42);
        expect.end();
    }

    #[test]
    fn cancel_buff_combines_masks() {
        let packet = build_cancel_buff(&[BuffStat::Recovery, BuffStat::Speed]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelBuff as i16);
        expect.long(0).long(0x80_0000_0004).byte(1);
        expect.end();
    }

    #[test]
    fn skill_effect_carries_both_levels() {
        let packet = build_skill_effect(7, 1001, 12, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowForeignEffect as i16).int(7);
        expect.byte(1).int(1001).byte(12).byte(3);
        expect.end();
    }
}
