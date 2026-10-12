use crate::codec::{encode_avatar_look, encode_character_stats};
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{AvatarLook, CharacterStats};
use packet::{io::write::PktWrite, Packet};

/// PIC mode byte in the character list: 2 means PIC is disabled.
const PIC_DISABLED: u8 = 2;

pub fn build_char_list(
    entries: &[(CharacterStats, AvatarLook)],
    character_slots: i32,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharList as i16)?;
    packet.write_byte(0)?; // status
    packet.write_byte(entries.len() as u8)?;
    for (stats, look) in entries {
        write_char_entry(&mut packet, stats, look)?;
    }
    packet.write_byte(PIC_DISABLED)?;
    packet.write_int(character_slots)?;
    Ok(packet)
}

pub fn build_char_name_response(name: &str, valid: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CharNameResponse as i16)?;
    packet.write_str_with_length(name)?;
    packet.write_byte(!valid as u8)?;
    Ok(packet)
}

pub fn build_char_delete(character_id: i32, status: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::DeleteCharacter as i16)?;
    packet.write_int(character_id)?;
    packet.write_byte(status)?;
    Ok(packet)
}

/// ADD_NEW_CHAR_ENTRY after a successful creation.
pub fn build_char_packet(stats: &CharacterStats, look: &AvatarLook) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::NewCharacter as i16)?;
    packet.write_byte(0)?;
    write_char_entry(&mut packet, stats, look)?;
    Ok(packet)
}

/// One character in the list (`addCharEntry` in HeavenMS), with ranking off.
fn write_char_entry(
    packet: &mut Packet,
    stats: &CharacterStats,
    look: &AvatarLook,
) -> Result<(), NetworkError> {
    encode_character_stats(packet, stats)?;
    encode_avatar_look(packet, look, false)?;
    packet.write_byte(0)?; // not the "view all" list
    packet.write_byte(0)?; // world ranking disabled
    Ok(())
}
