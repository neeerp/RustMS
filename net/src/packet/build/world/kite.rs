//! Kites and banners (5080000-5080003, report c014): a player's message
//! that floats above where they stood until the field takes it down.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// A kite as players see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KiteView {
    pub object_id: i32,
    pub item_id: i32,
    /// The player's lines, joined with '\n'.
    pub message: String,
    /// Who flew it.
    pub name: String,
    pub x: i16,
    pub y: i16,
}

/// CANNOT_SPAWN_KITE (0x10E, v83 0x65acb4): "You can't fly it there."
pub fn build_cannot_spawn_kite() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CannotSpawnKite as i16)?;
    Ok(p)
}

/// SPAWN_KITE (0x10F, v83 0x65acdf): the client draws the item's
/// `iconReward` about 100 px above (x, y) and shows the lines when it is
/// clicked. Cosmic sends the foothold id where the client reads y.
pub fn build_spawn_kite(kite: &KiteView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnKite as i16)?;
    p.write_int(kite.object_id)?;
    p.write_int(kite.item_id)?;
    p.write_str_with_length(&kite.message)?;
    p.write_str_with_length(&kite.name)?;
    p.write_short(kite.x)?;
    p.write_short(kite.y)?;
    Ok(p)
}

/// REMOVE_KITE (0x110, v83 0x65b2ca): the kite fades out over a second,
/// or with `fade` false is gone at once.
pub fn build_remove_kite(object_id: i32, fade: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveKite as i16)?;
    p.write_byte(if fade { 0 } else { 1 })?;
    p.write_int(object_id)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn a_kite_spawns_with_its_lines_name_and_place() {
        let kite = KiteView {
            object_id: 800_000_000,
            item_id: 5080000,
            message: "a\nb\n".into(),
            name: "Al".into(),
            x: -120,
            y: 35,
        };
        let packet = build_spawn_kite(&kite).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x10F).int(800_000_000).int(5080000);
        expect.string("a\nb\n").string("Al").short(-120).short(35);
        expect.end();
    }

    #[test]
    fn a_kite_fades_or_vanishes_and_can_be_refused() {
        let packet = build_remove_kite(800_000_001, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x110).byte(0).int(800_000_001);
        expect.end();
        let packet = build_remove_kite(800_000_001, false).unwrap();
        assert_eq!(packet.bytes[2], 1);
        let packet = build_cannot_spawn_kite().unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x10E);
        expect.end();
    }
}
