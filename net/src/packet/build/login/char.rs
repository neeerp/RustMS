use crate::packet::op::SendOpcode;
use db::character::Character;
use packet::{io::write::PktWrite, Packet};

pub fn build_char_list(chars: Vec<Character>) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CharList as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(0).unwrap(); // account status?

    packet.write_byte(chars.len() as u8).unwrap(); // number of chars
    for character in chars {
        write_char(&mut packet, &character);
    }

    packet.write_byte(2).unwrap(); // use pic?
    packet.write_int(3).unwrap(); // Number of character slots

    packet
}

pub fn build_char_name_response(name: &str, valid: bool) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CharNameResponse as i16;

    packet.write_short(op).unwrap();
    packet.write_str_with_length(name).unwrap();
    packet.write_byte(!valid as u8).unwrap();

    packet
}

pub fn build_char_delete(character_id: i32, status: u8) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::DeleteCharacter as i16;

    packet.write_short(op).unwrap();

    packet.write_int(character_id).unwrap();
    packet.write_byte(status).unwrap();

    packet
}

pub fn build_char_packet(character: Character) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::NewCharacter as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(0).unwrap();

    write_char(&mut packet, &character);

    packet
}

fn write_char(packet: &mut Packet, character: &Character) {
    write_char_meta(packet, &character);
    write_char_look(packet, &character);

    packet.write_byte(0).unwrap();

    // Disable rank.
    packet.write_byte(0).unwrap();
}

fn write_char_meta(packet: &mut Packet, character: &Character) {
    packet.write_int(character.id).unwrap();
    packet.write_str(&character.name).unwrap();
    packet
        .write_bytes(&vec![0u8; 13 - character.name.len()])
        .unwrap();
    packet.write_byte(character.gender as u8).unwrap();
    packet.write_byte(character.skin as u8).unwrap();
    packet.write_int(character.face).unwrap();
    packet.write_int(character.hair).unwrap();

    // Pets... Not implemented yet
    packet.write_long(0).unwrap();
    packet.write_long(0).unwrap();
    packet.write_long(0).unwrap();

    packet.write_byte(character.level as u8).unwrap();
    packet.write_short(character.job).unwrap();

    packet.write_short(character.stre).unwrap();
    packet.write_short(character.dex).unwrap();
    packet.write_short(character.int).unwrap();
    packet.write_short(character.luk).unwrap();
    packet.write_short(character.hp).unwrap();
    packet.write_short(character.maxhp).unwrap();
    packet.write_short(character.mp).unwrap();
    packet.write_short(character.maxmp).unwrap();
    packet.write_short(character.ap).unwrap();

    // SP
    packet.write_short(0).unwrap();

    packet.write_int(character.exp).unwrap();
    packet.write_short(character.fame).unwrap();

    // Gach xp?
    packet.write_int(0).unwrap();

    // Map.. Not implemented yet
    packet.write_int(0).unwrap();
    packet.write_byte(1).unwrap();

    packet.write_int(0).unwrap();
}

fn write_char_look(packet: &mut Packet, character: &Character) {
    packet.write_byte(character.gender as u8).unwrap();
    packet.write_byte(character.skin as u8).unwrap();
    packet.write_int(character.face).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_int(character.hair).unwrap();

    write_char_equips(packet, character);
}

fn write_char_equips(packet: &mut Packet, _character: &Character) {
    // Regular equips

    packet.write_byte(0xFF).unwrap();

    // Cash shop equips

    packet.write_byte(0xFF).unwrap();

    // Weapon
    packet.write_int(0).unwrap();

    // Pet stuff...
    packet.write_int(0).unwrap();
    packet.write_int(0).unwrap();
    packet.write_int(0).unwrap();
}
