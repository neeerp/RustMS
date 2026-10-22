//! The family's packets (recv 94-104 in the client; report b039).

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::family::{FamilyBuff, FamilyChart, FamilyInfo, Refusal, ENTITLEMENTS, MAX_JUNIORS};
use game_data::server_content::keys;
use packet::{io::write::PktWrite, Packet};
use std::time::{Duration, SystemTime};

fn op(op: SendOpcode) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(op as i16)?;
    Ok(packet)
}

/// FAMILY_CHART_RESULT: the Family Tree around `chart.viewed`. Out of a
/// family it is empty, and the client draws the player alone.
pub fn build_family_chart(chart: &FamilyChart) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyChart)?;
    packet.write_int(chart.viewed)?;
    packet.write_int(chart.members.len() as i32)?;
    for member in &chart.members {
        packet.write_int(member.id)?;
        packet.write_int(member.senior_id)?;
        packet.write_short(member.job)?;
        packet.write_byte(member.level)?;
        packet.write_byte(u8::from(member.online))?;
        packet.write_int(member.reputation)?;
        packet.write_int(member.total_reputation)?;
        packet.write_int(member.senior_reputation)?;
        packet.write_int(member.today_reputation)?;
        packet.write_int(member.map_id)?;
        // Minutes since they logged in, not kept yet.
        packet.write_int(0)?;
        packet.write_str_with_length(&member.name)?;
    }
    packet.write_int(chart.counts.len() as i32)?;
    for &(key, count) in &chart.counts {
        packet.write_int(key)?;
        packet.write_int(count)?;
    }
    packet.write_int(chart.used.len() as i32)?;
    for &(index, times) in &chart.used {
        packet.write_int(index)?;
        packet.write_int(times)?;
    }
    packet.write_short(MAX_JUNIORS as i16)?;
    Ok(packet)
}

/// FAMILY_INFO_RESULT: the Family window's numbers, name and message. Until
/// the first, the client sends nothing but `OPEN_FAMILY` and
/// `OPEN_FAMILY_PEDIGREE`.
pub fn build_family_info(info: &FamilyInfo) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyInfo)?;
    packet.write_int(info.reputation)?;
    packet.write_int(info.total_reputation)?;
    packet.write_int(info.today_reputation)?;
    packet.write_short(info.juniors as i16)?;
    packet.write_short(MAX_JUNIORS as i16)?;
    // Unknown; the client keeps it unread (b039 1.2).
    packet.write_short(0)?;
    packet.write_int(info.leader_id)?;
    packet.write_str_with_length(&info.name)?;
    packet.write_str_with_length(&info.precepts)?;
    // The entitlements used today.
    packet.write_int(info.used.len() as i32)?;
    for &(index, times) in &info.used {
        packet.write_int(index)?;
        packet.write_int(times)?;
    }
    Ok(packet)
}

/// FAMILY_PRIVILEGE_LIST: the entitlements, in USE_FAMILY's order, with
/// their names and descriptions from the content pack.
pub fn build_privilege_list() -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyPrivilegeList)?;
    packet.write_int(ENTITLEMENTS.len() as i32)?;
    let text = crate::content::text();
    for (index, entitlement) in ENTITLEMENTS.iter().enumerate() {
        packet.write_byte(entitlement.kind)?;
        packet.write_int(entitlement.cost)?;
        packet.write_int(entitlement.limit)?;
        packet.write_str_with_length(&text.get(&keys::family_entitlement_name(index), &[]))?;
        packet
            .write_str_with_length(&text.get(&keys::family_entitlement_description(index), &[]))?;
    }
    Ok(packet)
}

/// SET_PRIVILEGE: the icon of `buff`, with the entitlement's tooltip and
/// its time left at `now`; none clears it (report b050). The client's icon
/// stays at 0 until cleared.
pub fn build_set_privilege(
    buff: Option<&FamilyBuff>,
    now: SystemTime,
) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilySetPrivilege)?;
    let Some(buff) = buff else {
        packet.write_byte(0)?;
        return Ok(packet);
    };
    packet.write_byte(buff.benefit().icon())?;
    packet.write_int(buff.index as i32)?;
    // Two numbers the client keeps but never reads.
    packet.write_int(0)?;
    packet.write_int(0)?;
    // Ends in this many ms from now; 1 would make it ago.
    packet.write_byte(0)?;
    let left = buff.until.duration_since(now).unwrap_or(Duration::ZERO);
    packet.write_int(i32::try_from(left.as_millis()).unwrap_or(i32::MAX))?;
    Ok(packet)
}

/// SUMMON_REQUEST: `name`, on `map`, asks to bring the player there. The
/// client asks Yes or No (pool 0x1210) and answers with
/// FAMILY_SUMMON_RESPONSE (report b050).
pub fn build_summon_request(name: &str, map: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilySummonRequest)?;
    packet.write_str_with_length(name)?;
    packet.write_str_with_length(map)?;
    Ok(packet)
}

/// FAMILY_RESULT 1: the asker parted, and the client says from whom, by
/// the name it kept (report b048).
pub fn build_separated() -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyResult)?;
    packet.write_int(1)?;
    packet.write_int(0)?;
    Ok(packet)
}

/// FAMILY_RESULT 0x51 or 0x50: parting from a junior, or from the senior,
/// costs `fee`, more than the asker has.
pub fn build_separation_fee(from_junior: bool, fee: i32) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyResult)?;
    packet.write_int(if from_junior { 0x51 } else { 0x50 })?;
    packet.write_int(fee)?;
    Ok(packet)
}

/// FAMILY_RESULT with a refusal's line.
pub fn build_family_refusal(refusal: Refusal) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyResult)?;
    packet.write_int(refusal.code())?;
    packet.write_int(0)?;
    Ok(packet)
}

/// FAMILY_JOIN_REQUEST: `name` (`inviter`) asks the player to be their
/// junior. The client shows a card, or says no by itself.
pub fn build_join_request(inviter: i32, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyJoinRequest)?;
    packet.write_int(inviter)?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// FAMILY_JOIN_REQUEST_RESULT: `junior` said yes (a chat line) or no (a
/// notice).
pub fn build_join_result(accepted: bool, junior: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyJoinRequestResult)?;
    packet.write_byte(u8::from(accepted))?;
    packet.write_str_with_length(junior)?;
    Ok(packet)
}

/// FAMILY_JOIN_ACCEPTED: `senior` is now the player's senior (a chat line).
pub fn build_join_accepted(senior: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyJoinAccepted)?;
    packet.write_str_with_length(senior)?;
    Ok(packet)
}

/// FAMILY_NOTIFY_LOGIN_OR_LOGOUT: the player's senior or junior `name` came
/// online (a card) or left.
pub fn build_relative_online(online: bool, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyNotifyLoginOrLogout)?;
    packet.write_byte(u8::from(online))?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// FAMILY_REP_GAIN: "Your Rep has increased (`from`, +`gain`)" in the chat
/// log, or for a negative `gain`, the Rep used (0x120b, 0x120c).
pub fn build_rep_gain(gain: i32, from: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyRepGain)?;
    packet.write_int(gain)?;
    packet.write_str_with_length(from)?;
    Ok(packet)
}

/// NOTIFY_LEVELUP for a senior: "<Family> `name` has reached Lv. `level`."
/// When `halved`, a senior below that level also reads that Rep from `name`
/// is halved (0x1246; report 178-guilds).
pub fn build_family_level_up(halved: bool, level: u8, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::NotifyLevelUp)?;
    packet.write_byte(u8::from(!halved))?;
    packet.write_int(i32::from(level))?;
    packet.write_str_with_length(name)?;
    Ok(packet)
}

/// NOTIFY_JOB_CHANGE for a relative: "<Family> `name` has advanced to a(an)
/// `job`." The client's text has no '>' after the group, so it goes with
/// the name.
pub fn build_family_job_change(job: i16, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::NotifyJobChange)?;
    packet.write_byte(1)?;
    packet.write_int(i32::from(job))?;
    packet.write_str_with_length(&format!("> {name}"))?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_privilege_gives_the_icon_the_tooltip_and_the_time_left() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let buff = FamilyBuff::new(3, now);
        let packet = build_set_privilege(Some(&buff), now + Duration::from_secs(60)).unwrap();
        let mut expected = vec![0x67, 0, 2];
        expected.extend(3i32.to_le_bytes());
        expected.extend([0; 8]);
        expected.push(0);
        expected.extend(840_000i32.to_le_bytes());
        assert_eq!(packet.bytes, expected);
        assert_eq!(build_set_privilege(None, now).unwrap().bytes, [0x67, 0, 0]);
    }

    #[test]
    fn info_carries_the_numbers_then_the_names() {
        let info = FamilyInfo {
            reputation: 5,
            total_reputation: 9,
            today_reputation: 2,
            juniors: 1,
            leader_id: 3,
            name: "Alice".into(),
            precepts: "Hi".into(),
            used: vec![(3, 1)],
        };
        let packet = build_family_info(&info).unwrap();
        let mut expected = vec![0x5F, 0];
        expected.extend([5, 0, 0, 0, 9, 0, 0, 0, 2, 0, 0, 0]);
        expected.extend([1, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
        expected.extend([5, 0]);
        expected.extend(b"Alice");
        expected.extend([2, 0]);
        expected.extend(b"Hi");
        expected.extend([1, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(packet.bytes, expected);
    }

    #[test]
    fn a_chart_ends_with_the_viewers_uses_then_the_junior_limit() {
        let chart = FamilyChart {
            viewed: 7,
            used: vec![(3, 1)],
            ..FamilyChart::default()
        };
        let packet = build_family_chart(&chart).unwrap();
        let mut expected = vec![0x5E, 0, 7, 0, 0, 0];
        expected.extend([0; 8]);
        expected.extend([1, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0]);
        expected.extend([2, 0]);
        assert_eq!(packet.bytes, expected);
    }

    #[test]
    fn a_halved_level_up_is_type_0() {
        let packet = build_family_level_up(true, 31, "Bo").unwrap();
        assert_eq!(packet.bytes, [0x69, 0, 0, 31, 0, 0, 0, 2, 0, b'B', b'o']);
        let packet = build_family_level_up(false, 31, "Bo").unwrap();
        assert_eq!(packet.bytes[2], 1);
    }

    #[test]
    fn a_refusal_is_its_code_and_nothing() {
        let packet = build_family_refusal(Refusal::LevelGap).unwrap();
        assert_eq!(packet.bytes, [0x60, 0, 0x48, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_summons_names_the_summoner_then_the_map() {
        let packet = build_summon_request("Al", "Ant").unwrap();
        assert_eq!(
            packet.bytes,
            [0x68, 0, 2, 0, b'A', b'l', 3, 0, b'A', b'n', b't']
        );
    }
}
