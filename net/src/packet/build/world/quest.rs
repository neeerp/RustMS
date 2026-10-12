use super::stat::{build_stat_update, StatUpdate};
use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{CharacterStats, QuestState, QuestStatus};
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

pub fn quest_time(time: SystemTime) -> Result<i64, NetworkError> {
    Ok(filetime(time))
}
/// A quest's new status; `None` once it is forfeited.
pub fn build_quest_progress(
    quest_id: i16,
    status: Option<&QuestStatus>,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(1)?;
    p.write_short(quest_id)?;
    match status {
        None => {
            p.write_byte(QuestState::NotStarted as u8)?;
        }
        Some(QuestStatus::Started) => {
            p.write_byte(QuestState::Started as u8)?;
            p.write_str_with_length("")?;
        }
        Some(QuestStatus::Completed { at }) => {
            p.write_byte(QuestState::Completed as u8)?;
            p.write_long(quest_time(*at)?)?;
        }
    }
    Ok(p)
}
/// `UPDATE_QUEST_INFO` success (mode 8): the quest action at `npc_id`
/// succeeded. A nonzero `next_quest` makes the client continue straight into
/// that quest's conversation with the same NPC (`Act/nextQuest`).
pub fn build_quest_action_result(
    quest_id: i16,
    npc_id: i32,
    next_quest: i16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(8)?;
    p.write_short(quest_id)?;
    p.write_int(npc_id)?;
    p.write_short(next_quest)?;
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
    // Party, equipment, internet cafe, and rainbow week bonus EXP.
    for _ in 0..4 {
        p.write_int(0)?;
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;
    use std::time::{Duration, UNIX_EPOCH};

    /// FILETIME of the unix epoch plus 1000 seconds.
    const FILETIME_1000S: i64 = 116_444_746_000_000_000;

    #[test]
    fn quest_time_is_a_filetime() {
        let at = UNIX_EPOCH + Duration::from_secs(1_000);
        assert_eq!(quest_time(at).unwrap(), FILETIME_1000S);
    }

    #[test]
    fn started_quest_carries_empty_progress_string() {
        let packet = build_quest_progress(1021, Some(&QuestStatus::Started)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(1);
        expect.short(1021).byte(1).string("");
        expect.end();
    }

    #[test]
    fn completed_quest_carries_completion_filetime() {
        // HeavenClient (and the v83 client) read a FILETIME for status 2.
        let at = UNIX_EPOCH + Duration::from_secs(1_000);
        let packet = build_quest_progress(1021, Some(&QuestStatus::Completed { at })).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(1);
        expect.short(1021).byte(2).long(FILETIME_1000S);
        expect.end();
    }

    #[test]
    fn forfeited_quest_has_no_payload() {
        let packet = build_quest_progress(1021, None).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(1);
        expect.short(1021).byte(0);
        expect.end();
    }

    #[test]
    fn quest_action_result_names_the_npc_and_next_quest() {
        let packet = build_quest_action_result(1032, 2001, 1033).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(8);
        expect.short(1032).int(2001).short(1033);
        expect.end();
    }

    #[test]
    fn reward_stats_update_level_ap_and_exp() {
        let packet = build_reward_stats(&sample_stats()).unwrap();

        // Reference updatePlayerStats: mask, then values in mask order with
        // level a byte, AP a short and EXP an int.
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(1);
        expect.int(0x10 | 0x4000 | 0x10000);
        expect.byte(10).short(3).int(1234);
        expect.end();
    }

    #[test]
    fn gain_exp_matches_reference() {
        let packet = build_gain_exp(25).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(3);
        expect.byte(1).int(25).byte(0);
        expect.int(0);
        expect.byte(0).byte(0);
        expect.int(0);
        expect.byte(0);
        expect.int(0).int(0).int(0).int(0);
        expect.end();
    }
}
