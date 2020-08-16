use crate::packet::op::SendOpcode;
use packet::{io::write::PktWrite, Packet};

pub fn build_end_of_world_list() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(0xFF).unwrap();

    packet
}

pub fn build_world_details() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerList as i16;

    let world_name = "Scania";
    let server_id = 0;
    let flag = 0;
    let event_msg = "Test!";

    packet.write_short(op).unwrap();
    packet.write_byte(server_id).unwrap();
    packet.write_str_with_length(world_name).unwrap();
    packet.write_byte(flag).unwrap();
    packet.write_str_with_length(event_msg).unwrap();
    packet.write_byte(100).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_byte(100).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_byte(3).unwrap();

    // Channel 1 info
    packet.write_str_with_length("Scania-1").unwrap();
    packet.write_int(700).unwrap();
    packet.write_byte(1).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_byte(0).unwrap();

    packet.write_str_with_length("Scania-2").unwrap();
    packet.write_int(700).unwrap();
    packet.write_byte(1).unwrap();
    packet.write_byte(1).unwrap();
    packet.write_byte(0).unwrap();

    packet.write_str_with_length("Scania-3").unwrap();
    packet.write_int(700).unwrap();
    packet.write_byte(1).unwrap();
    packet.write_byte(2).unwrap();
    packet.write_byte(0).unwrap();

    packet.write_short(0).unwrap();

    packet
}

pub fn build_select_world() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::LastConnectedWorld as i16;

    packet.write_short(op).unwrap();
    packet.write_int(0).unwrap();

    packet
}

pub fn build_send_recommended_worlds() -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::RecommendedWorlds as i16;

    packet.write_short(op).unwrap();
    packet.write_byte(1).unwrap(); // No worlds recommended
    packet.write_int(0).unwrap();
    packet.write_int(0).unwrap();

    packet
}

pub fn build_server_status(status: i16) -> Packet {
    let mut packet = Packet::new_empty();
    let op = SendOpcode::ServerStatus as i16;

    packet.write_short(op).unwrap();
    packet.write_short(status).unwrap(); // Highly populated status!

    packet
}
