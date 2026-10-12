use crate::error::NetworkError;
use packet::{io::write::PktWrite, Packet};

/// Build the handshake_packet which shares the encryption IVs with the client.
pub fn build_handshake_packet(
    recv_iv: &Vec<u8>,
    send_iv: &Vec<u8>,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();

    packet.write_short(0x0E)?; // Packet length header
    packet.write_short(83)?; // Version

    // Patch version as a length-prefixed string: "1".
    packet.write_str_with_length("1")?;

    packet.write_bytes(&recv_iv)?;
    packet.write_bytes(&send_iv)?;
    packet.write_byte(8)?; // Locale byte

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    const RECV_IV: [u8; 4] = [1, 2, 3, 4];
    const SEND_IV: [u8; 4] = [5, 6, 7, 8];

    #[test]
    fn handshake_matches_reference() {
        let packet = build_handshake_packet(&RECV_IV.to_vec(), &SEND_IV.to_vec()).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x0E).short(83);
        expect.short(1).byte(49);
        expect.bytes(&RECV_IV).bytes(&SEND_IV).byte(8);
        expect.end();
    }
}
