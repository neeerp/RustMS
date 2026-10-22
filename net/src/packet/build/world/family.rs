//! The family's packets (recv 94-104 in the client; report b039).

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::family::{FamilyChart, FamilyInfo, Refusal, MAX_JUNIORS};
use packet::{io::write::PktWrite, Packet};

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
    // The entitlements used today, none yet.
    packet.write_int(0)?;
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
    // The entitlements used today, none yet.
    packet.write_int(0)?;
    Ok(packet)
}

/// FAMILY_PRIVILEGE_LIST: the entitlements, none yet.
pub fn build_privilege_list() -> Result<Packet, NetworkError> {
    let mut packet = op(SendOpcode::FamilyPrivilegeList)?;
    packet.write_int(0)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
        };
        let packet = build_family_info(&info).unwrap();
        let mut expected = vec![0x5F, 0];
        expected.extend([5, 0, 0, 0, 9, 0, 0, 0, 2, 0, 0, 0]);
        expected.extend([1, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
        expected.extend([5, 0]);
        expected.extend(b"Alice");
        expected.extend([2, 0]);
        expected.extend(b"Hi");
        expected.extend([0, 0, 0, 0]);
        assert_eq!(packet.bytes, expected);
    }

    #[test]
    fn a_refusal_is_its_code_and_nothing() {
        let packet = build_family_refusal(Refusal::LevelGap).unwrap();
        assert_eq!(packet.bytes, [0x60, 0, 0x48, 0, 0, 0, 0, 0, 0, 0]);
    }
}
