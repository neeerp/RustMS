use crate::{error::NetworkError, packet::op::SendOpcode};
use db::character::Character;
use packet::{io::write::PktWrite, Packet};

pub fn build_char_list(chars: Vec<Character>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CharList as i16;

    packet.write_short(op)?;
    packet.write_byte(0)?; // account status?

    packet.write_byte(chars.len() as u8)?; // number of chars
    for character in chars {
        write_char(&mut packet, &character)?;
    }

    packet.write_byte(2)?; // use pic?
    packet.write_int(3)?; // Number of character slots

    Ok(packet)
}

pub fn build_char_name_response(name: &str, valid: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CharNameResponse as i16;

    packet.write_short(op)?;
    packet.write_str_with_length(name)?;
    packet.write_byte(!valid as u8)?;

    Ok(packet)
}

pub fn build_char_delete(character_id: i32, status: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::DeleteCharacter as i16;

    packet.write_short(op)?;

    packet.write_int(character_id)?;
    packet.write_byte(status)?;

    Ok(packet)
}

pub fn build_char_packet(character: Character) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::NewCharacter as i16;

    packet.write_short(op)?;
    packet.write_byte(0)?;

    write_char(&mut packet, &character)?;

    Ok(packet)
}

fn write_char(packet: &mut Packet, character: &Character) -> Result<(), NetworkError> {
    write_char_meta(packet, &character)?;
    write_char_look(packet, &character)?;

    packet.write_byte(0)?;

    // Disable rank.
    packet.write_byte(0)?;

    Ok(())
}

fn write_char_meta(packet: &mut Packet, character: &Character) -> Result<(), NetworkError> {
    packet.write_int(character.id)?;
    packet.write_str(&character.name)?;
    packet.write_bytes(&vec![0u8; 13 - character.name.len()])?;
    packet.write_byte(character.gender as u8)?;
    packet.write_byte(character.skin as u8)?;
    packet.write_int(character.face)?;
    packet.write_int(character.hair)?;

    // Pets... Not implemented yet
    packet.write_long(0)?;
    packet.write_long(0)?;
    packet.write_long(0)?;

    packet.write_byte(character.level as u8)?;
    packet.write_short(character.job)?;

    packet.write_short(character.stre)?;
    packet.write_short(character.dex)?;
    packet.write_short(character.int)?;
    packet.write_short(character.luk)?;
    packet.write_short(character.hp)?;
    packet.write_short(character.maxhp)?;
    packet.write_short(character.mp)?;
    packet.write_short(character.maxmp)?;
    packet.write_short(character.ap)?;

    // SP
    packet.write_short(0)?;

    packet.write_int(character.exp)?;
    packet.write_short(character.fame)?;

    // Gach xp?
    packet.write_int(0)?;

    // Map.. Not implemented yet
    packet.write_int(0)?;
    packet.write_byte(1)?;

    packet.write_int(0)?;

    Ok(())
}

fn write_char_look(packet: &mut Packet, character: &Character) -> Result<(), NetworkError> {
    packet.write_byte(character.gender as u8)?;
    packet.write_byte(character.skin as u8)?;
    packet.write_int(character.face)?;
    packet.write_byte(0)?;
    packet.write_int(character.hair)?;

    write_char_equips(packet, character)?;

    Ok(())
}

fn write_char_equips(packet: &mut Packet, _character: &Character) -> Result<(), NetworkError> {
    // Regular equips

    // Overall (Top slot)
    packet.write_byte(5)?;
    packet.write_int(1052122)?;

    // Shoes
    packet.write_byte(7)?;
    packet.write_int(1072318)?;

    packet.write_byte(0xFF)?;

    // Cash shop equips

    packet.write_byte(0xFF)?;

    // Weapon
    packet.write_int(1302000)?;

    // Pet stuff...
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;

    Ok(())
}
