//! Maple TV (report 179 section 6): the TV every player in its reach sees,
//! a sender's look and five lines, maybe with the receiver's look beside.

use crate::codec::look::encode_avatar_look;
use crate::error::NetworkError;
use crate::maple_tv::MapleTv;
use crate::packet::op::SendOpcode;
use game::AvatarLook;
use packet::{io::write::PktWrite, Packet};

/// Why a Maple TV message was refused (recv 343's code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TvRefusal {
    /// "You've entered the wrong user name." (StringPool 0xf77)
    WrongName = 2,
    /// "The waiting line is longer than an hour. Please try using it at a
    /// later time." (StringPool 0xf76)
    LineTooLong = 3,
}

/// Put `tv` on the screen (recv 341, FUN_006371c1): `wait` is the seconds
/// the line takes to clear, which the next sender's Yes/No shows.
pub fn build_send_tv(
    tv: &MapleTv,
    partner: Option<&AvatarLook>,
    wait: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SendTv as i16)?;
    // Bit 2: a partner's look follows.
    packet.write_byte(if partner.is_some() { 3 } else { 1 })?;
    packet.write_byte(tv.screen())?;
    encode_avatar_look(&mut packet, &tv.look, false)?;
    packet.write_str_with_length(&tv.sender)?;
    packet.write_str_with_length(tv.receiver.as_deref().unwrap_or(""))?;
    for line in &tv.lines {
        packet.write_str_with_length(line)?;
    }
    packet.write_int(wait)?;
    if let Some(look) = partner {
        encode_avatar_look(&mut packet, look, false)?;
    }
    Ok(packet)
}

/// Clear the screen (recv 342, FUN_006371ad).
pub fn build_remove_tv() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::RemoveTv as i16)?;
    Ok(packet)
}

/// How sending a message went (recv 343, FUN_006373a0).
pub fn build_tv_result(refused: Option<TvRefusal>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::TvResult as i16)?;
    match refused {
        None => {
            packet.write_byte(0)?;
        }
        Some(why) => {
            packet.write_byte(1)?;
            packet.write_byte(why as u8)?;
        }
    }
    Ok(packet)
}
