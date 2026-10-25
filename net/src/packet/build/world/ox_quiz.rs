//! The OX Quiz (field type 5, reports b103 and b171): the question on the
//! float notice, its answer, and the field's help.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// OX_QUIZ (0x91, v83 FUN_00535a57): question `number` of `set` in the
/// client's `OXQuiz.img`, or with `question` false its explanation and
/// answer, on the float notice. A question also starts the event clock,
/// making one of 30 s if there is none; number 0 takes the notice down.
pub fn build_ox_quiz(question: bool, set: u8, number: u16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::OxQuiz as i16)?;
    p.write_byte(question.into())?;
    p.write_byte(set)?;
    p.write_short(number as i16)?;
    Ok(p)
}

/// GMEVENT_INSTRUCTIONS (0x92, FUN_005360c0): the map's help line `index`
/// (String.nx `help<index>`), told by the Maple Administrator.
pub fn build_gm_event_instructions(index: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GmEventInstructions as i16)?;
    p.write_byte(index)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn a_question_its_answer_and_the_help() {
        let packet = build_ox_quiz(true, 2, 104).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x91).byte(1).byte(2).short(104);
        expect.end();
        let packet = build_ox_quiz(false, 2, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x91).byte(0).byte(2).short(0);
        expect.end();
        let packet = build_gm_event_instructions(0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x92).byte(0);
        expect.end();
    }
}
