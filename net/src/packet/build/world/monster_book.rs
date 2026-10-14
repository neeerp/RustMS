use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// MONSTER_BOOK_SET_CARD: a picked-up card went into the book at `level`
/// copies, or was wasted on a full page.
pub fn build_set_card(full: bool, card: i32, level: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MonsterBookSetCard as i16)?;
    p.write_byte(u8::from(!full))?;
    p.write_int(card)?;
    p.write_int(i32::from(level))?;
    Ok(p)
}

/// MONSTER_BOOK_SET_COVER: the card now on the book's cover (0 for none).
pub fn build_set_cover(card: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MonsterBookSetCover as i16)?;
    p.write_int(card)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn set_card_says_whether_the_page_had_room() {
        let packet = build_set_card(false, 2380000, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MonsterBookSetCard as i16)
            .byte(1)
            .int(2380000)
            .int(3);
        expect.end();

        let packet = build_set_card(true, 2380000, 5).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MonsterBookSetCard as i16)
            .byte(0)
            .int(2380000)
            .int(5);
        expect.end();
    }

    #[test]
    fn set_cover_is_the_card() {
        let packet = build_set_cover(2380001).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::MonsterBookSetCover as i16)
            .int(2380001);
        expect.end();
    }
}
