use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// What became of a report, as CLAIM_RESULT tells the reporter in an OK box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimResult {
    /// "Your report has been successfully registered. You have %d reports
    /// left this week." (0xd35), or "...no report left this week." (0xd36).
    Kept { left: i32 },
    /// "Please re-check the character name, then try again." (0xd3b)
    NotFound,
    /// "You do not have enough mesos to report." (0xd3c)
    NoMesos,
    /// "You have exceeded the number of reports available." (0xd3e)
    NoneLeft,
    /// "Please try again later." (0xd3a)
    Later,
}

/// CLAIM_RESULT (0x2D, v83 FUN_00a27891): a mode; mode 2 adds whether the
/// reporter may report again today (0 says they may not, 0xd37) and how many
/// reports they have left this week.
pub fn build_claim_result(result: ClaimResult) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ClaimResult as i16)?;
    let mode = match result {
        ClaimResult::Kept { .. } => 2,
        ClaimResult::NotFound => 0x42,
        ClaimResult::NoMesos => 0x43,
        ClaimResult::NoneLeft => 0x45,
        ClaimResult::Later => 0x41,
    };
    packet.write_byte(mode)?;
    if let ClaimResult::Kept { left } = result {
        packet.write_byte(1)?;
        packet.write_int(left)?;
    }
    Ok(packet)
}

/// CLAIM_AVAILABLE_TIME (0x2E, v83 FUN_00a27b38): the hours of the day,
/// from `start` until `end`, in which the client lets a player report;
/// 0 and 0 is all day.
pub fn build_claim_available_time(start: u8, end: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ClaimAvailableTime as i16)?;
    packet.write_byte(start)?;
    packet.write_byte(end)?;
    Ok(packet)
}

/// CLAIM_STATUS_CHANGED (0x2F, v83 FUN_00a27b61): whether reports are taken.
/// Until it says so, the client answers every report with "The reporting
/// system is currently undergoing maintenence check." (0xd39).
pub fn build_claim_status(open: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ClaimStatusChanged as i16)?;
    packet.write_byte(u8::from(open))?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_report_says_how_many_are_left() {
        let packet = build_claim_result(ClaimResult::Kept { left: 9 }).unwrap();
        assert_eq!(packet.bytes, [0x2D, 0, 2, 1, 9, 0, 0, 0]);
        let packet = build_claim_result(ClaimResult::NoMesos).unwrap();
        assert_eq!(packet.bytes, [0x2D, 0, 0x43]);
    }

    #[test]
    fn reports_open_all_day() {
        let packet = build_claim_available_time(0, 0).unwrap();
        assert_eq!(packet.bytes, [0x2E, 0, 0, 0]);
        let packet = build_claim_status(true).unwrap();
        assert_eq!(packet.bytes, [0x2F, 0, 1]);
    }
}
