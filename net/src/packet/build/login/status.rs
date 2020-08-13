use packet::{io::write::PktWrite, Packet};

/// Build a login status packet that gets sent upon login failure, relaying the
/// reason.
pub fn build_login_status_packet(status: u8) -> Packet {
    // TODO: Need to create an enum for the status...
    let mut packet = Packet::new_empty();

    packet.write_short(0x00).unwrap();
    packet.write_byte(status).unwrap();
    packet.write_byte(0).unwrap();
    packet.write_int(0).unwrap();

    packet
}
