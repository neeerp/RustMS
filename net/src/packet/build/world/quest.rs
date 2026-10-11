use crate::{error::NetworkError, packet::op::SendOpcode};
use super::stat::{build_stat_update, StatUpdate};
use crate::codec::time::filetime;
use db::quest::QuestProgress;
use game::CharacterStats;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

pub fn quest_time(time: SystemTime) -> Result<i64, NetworkError> {
    Ok(filetime(time))
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
pub fn build_reward_stats(stats: &CharacterStats) -> Result<Packet, NetworkError> {
    build_stat_update(
        true,
        &[
            StatUpdate::Level(stats.level),
            StatUpdate::Ap(stats.ap),
            StatUpdate::Exp(stats.exp),
        ],
    )
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
