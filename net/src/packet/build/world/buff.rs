//! Buffs, skill cooldowns, and skill effects.

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::BuffStat;
use packet::{io::write::PktWrite, Packet};

/// A buff's bit in the client's two 64-bit masks: (first, second).
fn mask(stat: BuffStat) -> (u64, u64) {
    match stat {
        BuffStat::Recovery => (0, 0x4),
        BuffStat::Speed => (0, 0x80_0000_0000),
    }
}

fn write_mask(p: &mut Packet, stats: &[BuffStat]) -> Result<(), NetworkError> {
    let (first, second) = stats.iter().fold((0, 0), |(first, second), &stat| {
        let (f, s) = mask(stat);
        (first | f, second | s)
    });
    p.write_long(first as i64)?;
    p.write_long(second as i64)?;
    Ok(())
}

/// GIVE_BUFF: one buff from a skill, lasting `duration_ms`.
pub fn build_give_buff(
    skill_id: i32,
    duration_ms: i32,
    stat: BuffStat,
    value: i16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, &[stat])?;
    p.write_short(value)?;
    p.write_int(skill_id)?;
    p.write_int(duration_ms)?;
    p.write_int(0)?;
    p.write_byte(0)?;
    p.write_int(i32::from(value))?;
    Ok(p)
}

/// CANCEL_BUFF: these buffs ended.
pub fn build_cancel_buff(stats: &[BuffStat]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CancelBuff as i16)?;
    write_mask(&mut p, stats)?;
    p.write_byte(1)?;
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
        let packet = build_give_buff(1002, 4000, BuffStat::Speed, 10).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(0x80_0000_0000);
        expect.short(10).int(1002).int(4000);
        expect.int(0).byte(0).int(10);
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
