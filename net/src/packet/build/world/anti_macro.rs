use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The item flow's code, after the type: the GM skill's is 2.
const ITEM: u8 = 1;

/// What ANTI_MACRO_RESULT tells a player of a Lie Detector test (report
/// c025), by its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AntiMacroResult {
    /// 0: "The user cannot be found." (0xc56)
    NotFound,
    /// 1: "You cannot use it on a user that isn't in the middle of attack."
    /// (0xc57)
    NotAttacking,
    /// 2: "This user has already been tested before." (0xc58)
    TestedBefore,
    /// 3: "This user is currently going through the Lie Detector Test."
    /// (0xc59)
    InTest,
    /// 5: "`name` have used the Lie Detector Test." in chat (0xc5f).
    Used { name: String },
    /// 6: the picture to read, as a JPEG; `first` starts the client's 60
    /// seconds, otherwise it shows the time it had left.
    Challenge { first: bool, jpeg: Vec<u8> },
    /// 7: the test is over and failed, "...confirms that you have been
    /// botting." (0xc5b)
    Failed,
    /// 9: the test is over and passed, "...You'll be rewarded 5000 mesos
    /// for not botting." (0xc5a)
    Passed,
    /// 11: to the tester, "The user has failed the Lie Detector Test.
    /// You'll be rewarded 7000 mesos from the user." (0xc6a)
    FineReceived,
}

/// ANTI_MACRO_RESULT (0x2B, v83 FUN_00a265c2): a type, a code and what the
/// type carries.
pub fn build_anti_macro_result(result: &AntiMacroResult) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::AntiMacroResult as i16)?;
    let kind = match result {
        AntiMacroResult::NotFound => 0,
        AntiMacroResult::NotAttacking => 1,
        AntiMacroResult::TestedBefore => 2,
        AntiMacroResult::InTest => 3,
        AntiMacroResult::Used { .. } => 5,
        AntiMacroResult::Challenge { .. } => 6,
        AntiMacroResult::Failed => 7,
        AntiMacroResult::Passed => 9,
        AntiMacroResult::FineReceived => 11,
    };
    packet.write_byte(kind)?;
    packet.write_byte(ITEM)?;
    match result {
        AntiMacroResult::Used { name } => {
            packet.write_str_with_length(name)?;
        }
        AntiMacroResult::Challenge { first, jpeg } => {
            packet.write_byte(u8::from(*first))?;
            packet.write_int(jpeg.len() as i32)?;
            packet.write_bytes(jpeg)?;
        }
        _ => {}
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_challenge_carries_its_flag_and_picture() {
        let result = AntiMacroResult::Challenge {
            first: true,
            jpeg: vec![0xFF, 0xD8, 0xFF, 0xD9],
        };
        let packet = build_anti_macro_result(&result).unwrap();
        assert_eq!(
            packet.bytes,
            [0x2B, 0, 6, 1, 1, 4, 0, 0, 0, 0xFF, 0xD8, 0xFF, 0xD9]
        );
    }

    #[test]
    fn the_user_line_names_the_tester() {
        let result = AntiMacroResult::Used { name: "Ann".into() };
        let packet = build_anti_macro_result(&result).unwrap();
        assert_eq!(packet.bytes, [0x2B, 0, 5, 1, 3, 0, b'A', b'n', b'n']);
    }

    #[test]
    fn the_ends_carry_only_their_type() {
        for (result, kind) in [
            (AntiMacroResult::NotFound, 0),
            (AntiMacroResult::NotAttacking, 1),
            (AntiMacroResult::TestedBefore, 2),
            (AntiMacroResult::InTest, 3),
            (AntiMacroResult::Failed, 7),
            (AntiMacroResult::Passed, 9),
            (AntiMacroResult::FineReceived, 11),
        ] {
            let packet = build_anti_macro_result(&result).unwrap();
            assert_eq!(packet.bytes, [0x2B, 0, kind, 1]);
        }
    }
}
