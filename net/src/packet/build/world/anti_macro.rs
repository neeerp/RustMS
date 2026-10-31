use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// What gave the test, the code after the type: the item, or a GM's
/// skill (9001009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detector {
    Item = 1,
    Skill = 2,
}

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
    /// 5: by the item, to the tested player, "`name` have used the Lie
    /// Detector Test." in chat (0xc5f); by the skill, to the GM, "`name`_The
    /// screenshot has been saved. The Lie Detector has been activated."
    /// (0xc61), with a screenshot.
    Used { name: String },
    /// 6: the picture to read, as a JPEG; `first` starts the client's 60
    /// seconds, otherwise it shows the time it had left.
    Challenge { first: bool, jpeg: Vec<u8> },
    /// 7: the test is over and failed, "...confirms that you have been
    /// botting." (0xc5b), and by the skill "You will be sanctioned for using
    /// a macro-assisted program." (0xc6c)
    Failed,
    /// 8: to the GM, "`name`_The screenshot has been saved. It appears that
    /// you may be using a macro-assisted program." (0xc63), with a
    /// screenshot.
    Suspected { name: String },
    /// 9: the test is over and passed, "...You'll be rewarded 5000 mesos
    /// for not botting." (0xc5a), by the skill "Thank you for your
    /// cooperation." (0x15bd)
    Passed,
    /// 10: to the GM, "`name`_You have passed the Lie Detector Test."
    /// (0xc62)
    Cleared { name: String },
    /// 11: to the tester, "The user has failed the Lie Detector Test.
    /// You'll be rewarded 7000 mesos from the user." (0xc6a)
    FineReceived,
}

/// ANTI_MACRO_RESULT (0x2B, v83 FUN_00a265c2) for the item.
pub fn build_anti_macro_result(result: &AntiMacroResult) -> Result<Packet, NetworkError> {
    build_anti_macro_result_by(Detector::Item, result)
}

/// ANTI_MACRO_RESULT: a type, the code of what gave the test and what the
/// type carries.
pub fn build_anti_macro_result_by(
    by: Detector,
    result: &AntiMacroResult,
) -> Result<Packet, NetworkError> {
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
        AntiMacroResult::Suspected { .. } => 8,
        AntiMacroResult::Passed => 9,
        AntiMacroResult::Cleared { .. } => 10,
        AntiMacroResult::FineReceived => 11,
    };
    packet.write_byte(kind)?;
    packet.write_byte(by as u8)?;
    match result {
        AntiMacroResult::Used { name }
        | AntiMacroResult::Suspected { name }
        | AntiMacroResult::Cleared { name } => {
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
    fn the_skills_carry_its_code_and_the_gms_lines_the_name() {
        let packet = build_anti_macro_result_by(Detector::Skill, &AntiMacroResult::Passed).unwrap();
        assert_eq!(packet.bytes, [0x2B, 0, 9, 2]);
        for (result, kind) in [
            (AntiMacroResult::Suspected { name: "Bo".into() }, 8),
            (AntiMacroResult::Cleared { name: "Bo".into() }, 10),
        ] {
            let packet = build_anti_macro_result_by(Detector::Skill, &result).unwrap();
            assert_eq!(packet.bytes, [0x2B, 0, kind, 2, 2, 0, b'B', b'o']);
        }
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
