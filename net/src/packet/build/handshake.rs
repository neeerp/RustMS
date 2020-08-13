use packet::{io::write::PktWrite, Packet};

/// Build the handshake_packet which shares the encryption IVs with the client.
pub fn build_handshake_packet(recv_iv: &Vec<u8>, send_iv: &Vec<u8>) -> Packet {
    let mut packet = Packet::new_empty();

    packet.write_short(0x0E).unwrap(); // Packet length header
    packet.write_short(83).unwrap(); // Version

    // Not sure what this part is meant to represent...
    // HeavenClient doesn't seem to care for these values but the
    // official clients do...
    packet.write_short(0).unwrap();
    packet.write_byte(0).unwrap();

    packet.write_bytes(&recv_iv).unwrap();
    packet.write_bytes(&send_iv).unwrap();
    packet.write_byte(8).unwrap(); // Locale byte

    packet
}
