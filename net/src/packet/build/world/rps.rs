//! RPS_GAME (recv 312; report b038): the Rock, Paper, Scissors window. Every
//! packet starts with a mode byte; the client ignores modes 9 to 14 without
//! the window, and throws if mode 8 comes while it is open.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Refused for want of the fee (pool 0xe64); the window shows Start again.
pub const NO_MESOS: u8 = 0x06;
/// Refused for want of a free etc. slot (pool 0xe63).
pub const NO_SLOT: u8 = 0x07;
const OPEN: u8 = 0x08;
/// The first round of a game.
pub const START: u8 = 0x09;
/// The round ran out of time.
pub const TIME_OVER: u8 = 0x0a;
const ROUND: u8 = 0x0b;
/// The next round of a streak; the client takes it as it takes 9.
pub const NEXT_ROUND: u8 = 0x0c;
/// Closes the window.
pub const CLOSE: u8 = 0x0d;

/// RPS_GAME with no more than its mode.
pub fn build_rps(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::RpsGame as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// Opens the window with the notice of `npc_id` first.
pub fn build_rps_open(npc_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = build_rps(OPEN)?;
    packet.write_int(npc_id)?;
    Ok(packet)
}

/// A round's end: the NPC's hand, where the client's spin stops, and the
/// streak, negative on a loss.
pub fn build_rps_round(hand: u8, streak: i8) -> Result<Packet, NetworkError> {
    let mut packet = build_rps(ROUND)?;
    packet.write_byte(hand)?;
    packet.write_byte(streak as u8)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lost_round_sends_its_streak_signed() {
        let packet = build_rps_round(2, -1).unwrap();
        assert_eq!(packet.bytes, vec![0x38, 0x01, 0x0b, 2, 0xff]);
    }

    #[test]
    fn opening_names_the_npc() {
        let packet = build_rps_open(9000019).unwrap();
        assert_eq!(&packet.bytes[..3], &[0x38, 0x01, 0x08]);
        assert_eq!(&packet.bytes[3..], &9000019i32.to_le_bytes());
    }
}
