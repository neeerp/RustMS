//! Name Change (5400000, report c060): the Cash Shop's answers while one is
//! bought, and the answers to cancelling one from the bag.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Whether a character may ask for a new name; v83 shows the rules, then
/// the name dialog, only for `Allowed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NameChangeRules {
    Allowed = 0,
    /// "The name change is already submitted due to the item purchase."
    Pending = 1,
    /// The recent-ban notice (unused: bans are not kept).
    Banned = 2,
    /// The once-a-month notice.
    Cooldown = 3,
    /// "An unknown error has occured.": too low a level, or a bad request.
    Refused = 4,
}

/// NAME_CHANGE_RULES_RESULT (0x149): unlocks the shop.
pub fn build_name_change_rules(rules: NameChangeRules) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NameChangeRulesResult as i16)?;
    packet.write_int(0)?;
    packet.write_byte(rules as u8)?;
    packet.write_int(0)?;
    Ok(packet)
}

/// CHECK_NEW_NAME_RESULT (0x148): the name checked and whether it is free.
/// v83 has one notice for a name taken or not allowed.
pub fn build_new_name_check(name: &str, free: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CheckNewNameResult as i16)?;
    packet.write_str_with_length(name)?;
    packet.write_byte(u8::from(!free))?;
    Ok(packet)
}

/// CANCEL_NAME_CHANGE_RESULT (0x71): 0 is the cancelled image, 0xFF the
/// failed one.
pub fn build_name_change_cancelled(cancelled: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CancelNameChangeResult as i16)?;
    packet.write_byte(if cancelled { 0 } else { 0xFF })?;
    Ok(packet)
}

/// NO_NAME_CHANGE (0x78): "The character you logged on with has not
/// requested a name change."; unlocks the client.
pub fn build_no_name_change() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NoNameChange as i16)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_answer_is_a_code_between_two_ints() {
        let packet = build_name_change_rules(NameChangeRules::Cooldown).unwrap();
        assert_eq!(packet.bytes, [0x49, 0x01, 0, 0, 0, 0, 3, 0, 0, 0, 0]);
    }

    #[test]
    fn name_check_echoes_the_name() {
        let packet = build_new_name_check("Maple", false).unwrap();
        assert_eq!(
            packet.bytes,
            [0x48, 0x01, 5, 0, b'M', b'a', b'p', b'l', b'e', 1]
        );
        let packet = build_new_name_check("Maple", true).unwrap();
        assert_eq!(packet.bytes.last(), Some(&0));
    }

    #[test]
    fn cancel_answers() {
        assert_eq!(
            build_name_change_cancelled(true).unwrap().bytes,
            [0x71, 0, 0]
        );
        assert_eq!(
            build_name_change_cancelled(false).unwrap().bytes,
            [0x71, 0, 0xFF]
        );
        assert_eq!(build_no_name_change().unwrap().bytes, [0x78, 0]);
    }
}
