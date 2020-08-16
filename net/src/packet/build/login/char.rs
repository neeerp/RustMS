use crate::packet::op::SendOpcode;
use packet::{io::write::PktWrite, Packet};

pub fn build_char_list() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CharList as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(0).unwrap(); // account status?
    packet.write_byte(0).unwrap(); // number of chars

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
