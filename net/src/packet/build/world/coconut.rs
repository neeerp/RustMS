//! Coconut Season (field type 4, report b167): the nuts, the scoreboard
//! and the team colours.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// COCONUT_HIT (0x11D, v83 0x55c8e8): nut `index` (-1 every nut) goes to
/// `state` after `delay` ms: 0 shows it, 1 shakes it, 2 bursts it, 3
/// drops it.
pub fn build_coconut_hit(index: i16, delay: u16, state: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CoconutHit as i16)?;
    p.write_short(index)?;
    p.write_short(delay as i16)?;
    p.write_byte(state)?;
    Ok(p)
}

/// COCONUT_SCORE (0x11E): the scoreboard's points for Maple and Story.
pub fn build_coconut_score(maple: u16, story: u16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CoconutScore as i16)?;
    p.write_short(maple as i16)?;
    p.write_short(story as i16)?;
    Ok(p)
}

/// FIELD_SPECIFIC_DATA (0x85): the player's team on an event field, which
/// dresses them in its colours; `None` for no team.
pub fn build_field_team(team: Option<u8>) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldSpecificData as i16)?;
    p.write_byte(team.unwrap_or(0xFF))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn a_nut_changes_after_its_delay() {
        let packet = build_coconut_hit(-1, 5000, 0).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11D).short(-1).short(5000).byte(0);
        expect.end();
        let packet = build_coconut_hit(42, 1000, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11D).short(42).short(1000).byte(3);
        expect.end();
    }

    #[test]
    fn the_score_and_the_team() {
        let packet = build_coconut_score(3, 12).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11E).short(3).short(12);
        expect.end();
        let packet = build_field_team(Some(1)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x85).byte(1);
        expect.end();
        assert_eq!(build_field_team(None).unwrap().bytes[2], 0xFF);
    }
}
