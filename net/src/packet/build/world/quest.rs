use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{QuestState, QuestStatus};
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
        Some(status @ QuestStatus::Started { .. }) => {
            p.write_byte(QuestState::Started as u8)?;
            p.write_str_with_length(&status.record_string().unwrap_or_default())?;
        }
        Some(QuestStatus::Completed { at }) => {
            p.write_byte(QuestState::Completed as u8)?;
            p.write_long(quest_time(*at)?)?;
        }
    }
    Ok(p)
}
/// A started record with its own data string, as the explorer medals' info
/// records (27010 ...) hold their counts.
pub fn build_quest_record(quest_id: i16, data: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(1)?;
    p.write_short(quest_id)?;
    p.write_byte(QuestState::Started as u8)?;
    p.write_str_with_length(data)?;
    Ok(p)
}

/// `QUEST_CLEAR`: the quest's title is earned.
pub fn build_quest_clear(quest_id: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::QuestClear as i16)?;
    p.write_short(quest_id)?;
    Ok(p)
}

/// `SCRIPT_PROGRESS_MESSAGE`: a floating message on screen.
pub fn build_script_progress(text: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ScriptProgressMessage as i16)?;
    p.write_str_with_length(text)?;
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

/// `UPDATE_QUEST_INFO` mode 6: the clock of a timed quest (`QuestInfo`
/// `timeLimit`) with `ms` left. v83 (FUN_00959a73) adds one for each quest
/// it reads without looking for one already shown, so it is sent once.
pub fn build_quest_timer(quest_id: i16, ms: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(6)?;
    p.write_short(1)?;
    p.write_short(quest_id)?;
    p.write_int(ms)?;
    Ok(p)
}

/// `UPDATE_QUEST_INFO` mode 7: the quest's clock goes, as the quest is done
/// or given up.
pub fn build_quest_timer_end(quest_id: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(7)?;
    p.write_short(1)?;
    p.write_short(quest_id)?;
    Ok(p)
}

/// `UPDATE_QUEST_INFO` mode 15: "The [name] quest expired because the time
/// limit ended" in the chat log.
pub fn build_quest_expired(quest_id: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(15)?;
    p.write_short(quest_id)?;
    Ok(p)
}

/// `UPDATE_QUEST_INFO` mode 10: the items of the quest's step don't fit.
/// The client names the inventories from the step's own items.
pub fn build_quest_items_full(quest_id: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(10)?;
    p.write_short(quest_id)?;
    Ok(p)
}

/// `UPDATE_QUEST_INFO` mode 14: "You may not possess more than one of this
/// item." (FUN_00959a73).
pub fn build_quest_only_one() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(14)?;
    Ok(p)
}

/// `UPDATE_QUEST_INFO` mode 11: "You do not have enough mesos."
pub fn build_quest_short_of_mesos() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::UpdateQuestInfo as i16)?;
    p.write_byte(11)?;
    Ok(p)
}

/// The white "You have gained experience" line on the right; `party_bonus`
/// is the extra EXP from sharing the kill with a party, shown on its own
/// line under it.
pub fn build_gain_exp(gain: i32, party_bonus: i32) -> Result<Packet, NetworkError> {
    write_gain_exp(gain, party_bonus, false)
}

/// "You have gained experience" in the chat log, as for quest EXP.
pub fn build_gain_exp_in_chat(gain: i32) -> Result<Packet, NetworkError> {
    write_gain_exp(gain, 0, true)
}

/// SHOW_STATUS_INFO 4: "You have gained fame. (+n)" (or lost it) in the
/// chat log, for a quest's fame reward.
pub fn build_fame_gain_in_chat(gain: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(4)?;
    p.write_int(i32::from(gain))?;
    Ok(p)
}

/// SHOW_STATUS_INFO 9: a line in the chat log (the v83 client's chat type
/// 0xc, salmon), as Cosmic's `showInfoText`.
pub fn build_info_text(text: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(9)?;
    p.write_str_with_length(text)?;
    Ok(p)
}

fn write_gain_exp(gain: i32, party_bonus: i32, in_chat: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowStatusInfo as i16)?;
    p.write_byte(3)?;
    p.write_byte(1)?;
    p.write_int(gain)?;
    p.write_byte(u8::from(in_chat))?;
    p.write_int(0)?;
    p.write_short(0)?;
    p.write_int(0)?;
    if in_chat {
        // Quest bonus EXP rate.
        p.write_byte(0)?;
    }
    p.write_byte(0)?;
    p.write_int(party_bonus)?;
    // Equipment, internet cafe, and rainbow week bonus EXP.
    for _ in 0..3 {
        p.write_int(0)?;
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use std::time::{Duration, UNIX_EPOCH};

    /// FILETIME of the unix epoch plus 1000 seconds.
    const FILETIME_1000S: i64 = 116_444_746_000_000_000;

    #[test]
    fn an_info_text_is_mode_9_and_its_line() {
        let packet = build_info_text("Go right").unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(9)
            .string("Go right")
            .end();
    }

    #[test]
    fn an_only_one_refusal_is_mode_14_alone() {
        let packet = build_quest_only_one().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(14);
        expect.end();
    }

    #[test]
    fn quest_time_is_a_filetime() {
        let at = UNIX_EPOCH + Duration::from_secs(1_000);
        assert_eq!(quest_time(at).unwrap(), FILETIME_1000S);
    }

    #[test]
    fn started_quest_carries_its_kill_counts() {
        let status = QuestStatus::Started {
            kills: vec![3, 12],
            info: None,
        };
        let packet = build_quest_progress(1021, Some(&status)).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(1);
        expect.short(1021).byte(1).string("003012");
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
    fn a_quest_clock_is_a_list_of_one() {
        let packet = build_quest_timer(3458, 1_800_000).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(6);
        expect.short(1).short(3458).int(1_800_000);
        expect.end();

        let packet = build_quest_timer_end(3458).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(7);
        expect.short(1).short(3458);
        expect.end();

        let packet = build_quest_expired(3458).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(15);
        expect.short(3458);
        expect.end();
    }

    #[test]
    fn quest_refusals_carry_only_what_the_client_reads() {
        let packet = build_quest_items_full(1032).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::UpdateQuestInfo as i16)
            .byte(10)
            .short(1032);
        expect.end();

        let packet = build_quest_short_of_mesos().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdateQuestInfo as i16).byte(11);
        expect.end();
    }

    #[test]
    fn fame_rewards_show_in_chat_as_mode_4() {
        let packet = build_fame_gain_in_chat(-2).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowStatusInfo as i16)
            .byte(4)
            .int(-2);
        expect.end();
    }

    #[test]
    fn gain_exp_matches_reference() {
        let packet = build_gain_exp(25, 3).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(3);
        expect.byte(1).int(25).byte(0);
        expect.int(0);
        expect.byte(0).byte(0);
        expect.int(0);
        expect.byte(0);
        expect.int(3).int(0).int(0).int(0);
        expect.end();
    }

    #[test]
    fn gain_exp_in_chat_carries_the_quest_rate() {
        let packet = build_gain_exp_in_chat(10).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowStatusInfo as i16).byte(3);
        expect.byte(1).int(10).byte(1);
        expect.int(0);
        expect.byte(0).byte(0);
        expect.int(0);
        expect.byte(0).byte(0);
        expect.int(0).int(0).int(0).int(0);
        expect.end();
    }
}
