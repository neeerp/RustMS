//! VICIOUS_HAMMER (recv 354, v83 FUN_0082b2c3): the Vicious' Hammer window's
//! answers. Every one clears the client's item-use lock (report c004b).

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The hammer took: the window fills its gauge, plays the effect and acks
/// with USE_HAMMER.
const APPLIED: u8 = 0x39;
/// The ack is answered: the window closes with "Increased available upgrade
/// by 1. %d upgrades are left."
const DONE: u8 = 0x3d;
/// The hammer is refused: the window closes with the reason's text.
const REFUSED: u8 = 0x3e;

/// Why a hammer is refused, as the client words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HammerRefusal {
    /// "The item is not upgradable."
    NotUpgradable = 1,
    /// "2 upgrade increases have been used already."
    UsedUp = 2,
    /// "You can't use Vicious' Hammer on Horntail Necklace."
    Horntail = 3,
}

/// The hammer took; `applied` hammers are now on the equip, and the client
/// says 2 - `applied` are left.
pub fn build_hammer_applied(applied: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ViciousHammer as i16)?;
    packet.write_byte(APPLIED)?;
    packet.write_int(0)?;
    packet.write_int(applied)?;
    Ok(packet)
}

/// The answer to USE_HAMMER: the window closes with its success text.
pub fn build_hammer_done() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ViciousHammer as i16)?;
    packet.write_byte(DONE)?;
    packet.write_int(0)?;
    Ok(packet)
}

/// The hammer is refused.
pub fn build_hammer_refusal(why: HammerRefusal) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ViciousHammer as i16)?;
    packet.write_byte(REFUSED)?;
    packet.write_int(why as i32)?;
    Ok(packet)
}
