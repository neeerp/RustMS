use crate::{error::NetworkError, packet::op::SendOpcode};
use db::{character::Character, quest::QuestProgress};
use packet::{io::write::PktWrite, Packet};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn quest_time(time: SystemTime) -> Result<i64, NetworkError> {
    Ok(time.duration_since(UNIX_EPOCH)?.as_millis() as i64 * 10_000 + 116_444_736_000_000_000)
}
pub fn build_quest_progress(progress: &QuestProgress) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(1)?;
    p.write_short(progress.quest_id)?;
    p.write_byte(progress.status as u8)?;
    if progress.status == 1 {
        p.write_str_with_length("")?;
    } else if let Some(time) = progress.completed_at {
        p.write_long(quest_time(time)?)?;
    }
    Ok(p)
}
pub fn build_reward_stats(character: &Character) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::StatChange as i16)?;
    p.write_byte(1)?;
    p.write_int(0x10 | 0x2000 | 0x10000)?;
    p.write_byte(character.level as u8)?;
    p.write_short(character.ap)?;
    p.write_int(character.exp)?;
    Ok(p)
}
pub fn build_gain_exp(gain: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(3)?;
    p.write_byte(1)?;
    p.write_int(gain)?;
    p.write_byte(0)?;
    p.write_int(0)?;
    p.write_short(0)?;
    p.write_int(0)?;
    p.write_byte(0)?;
    for _ in 0..3 {
        p.write_int(0)?;
    }
    Ok(p)
}
