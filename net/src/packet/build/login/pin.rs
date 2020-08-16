use crate::packet::op::SendOpcode;
use packet::{io::write::PktWrite, Packet};

pub fn build_pin_updated() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::UpdatePin as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(0).unwrap();

    packet
}

pub fn build_pin_accepted() -> Packet {
    build_pin_packet(0)
}

pub fn build_pin_register() -> Packet {
    build_pin_packet(1)
}

pub fn _build_pin_failed() -> Packet {
    build_pin_packet(2)
}

pub fn _build_pin_request() -> Packet {
    build_pin_packet(4)
}

fn build_pin_packet(mode: u8) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::CheckPin as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(mode).unwrap();

    packet
}
