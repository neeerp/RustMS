//! VEGA_SCROLL (recv 358, v83 FUN_0082d8d5): one code byte for the Vega's
//! Spell window. A start code begins the roll and picks the window that
//! follows it; a result code lets that window open once the roll is over
//! (report c004c).

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Start the roll; `success` picks the success or the failure window.
pub fn build_vega_start(success: bool) -> Result<Packet, NetworkError> {
    build_vega(if success { 0x40 } else { 0x45 })
}

/// The scroll is applied: the window that the start code picked opens when
/// the roll is over, and the client's item actions unlock.
pub fn build_vega_result(success: bool) -> Result<Packet, NetworkError> {
    build_vega(if success { 0x41 } else { 0x43 })
}

fn build_vega(code: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::VegaScroll as i16)?;
    packet.write_byte(code)?;
    Ok(packet)
}
