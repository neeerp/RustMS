use crate::{error::NetworkError, helpers::to_hex_string};
use bufstream::BufStream;
use crypt::maple_crypt;
use crypt::MapleAES;
use packet::{
    io::{PktRead, PktWrite},
    Packet,
};
use std::{io::Write, net::TcpStream};

pub struct LoginCredentialsHandler {}

impl LoginCredentialsHandler {
    pub fn new() -> Self {
        LoginCredentialsHandler {}
    }

    // TODO: We probably want to return a credentials object or something...
    pub fn handle(
        &self,
        packet: &Packet,
        stream: &mut BufStream<TcpStream>,
        send_crypt: &mut MapleAES,
    ) -> Result<(), NetworkError> {
        println!("Login attempted...");

        let mut cursor = 2;

        let user = packet.read_str_with_length(cursor);
        cursor = cursor + user.len() + 2;

        let pw = packet.read_str_with_length(cursor);
        cursor = cursor + pw.len() + 2;

        // The next 6 bytes should be zero'd out
        cursor = cursor + 6;

        let hwid_nibble = packet.read_bytes(cursor, 4);

        println!("Username: {}", user);
        println!("Password: {}", pw);
        println!("HWID nibble: {}", to_hex_string(hwid_nibble.to_vec()));

        let mut return_packet = Packet::new_empty();

        // Send a login failed packet with the "Not registered" reason
        println!("Denying logon...");
        return_packet.write_short(0x00);
        return_packet.write_byte(5);
        return_packet.write_byte(0);
        return_packet.write_int(0);

        maple_crypt::encrypt(&mut return_packet);
        send_crypt.crypt(&mut return_packet);

        let header = send_crypt.gen_packet_header(return_packet.len() + 2);

        // Header
        stream.write(&header).unwrap();
        // Packet
        stream.write(&return_packet).unwrap();
        stream.flush().unwrap();
        println!("Logon denied packet sent.");

        Ok(())
    }
}

// TODO: Need to read about mocking!
//
// #[cfg(test)]
// mod tests {
//     use super::LoginCredentialsHandler;
//     use packet::{io::PktWrite, Packet};
//     use rand::{distributions::Alphanumeric, Rng};

//     #[test]
//     fn user_pw_login() {
//         let handler = LoginCredentialsHandler::new();
//         let mut rng = rand::thread_rng();
//         for _ in 0..100 {
//             let mut packet = Packet::new_empty();

//             let user_length = rng.gen_range(0, 255);
//             let pw_length = rng.gen_range(0, 255);

//             let user = rng
//                 .sample_iter(&Alphanumeric)
//                 .take(user_length)
//                 .collect::<String>();

//             let pw = rng
//                 .sample_iter(&Alphanumeric)
//                 .take(pw_length)
//                 .collect::<String>();

//             let zeros = [0u8; 6];
//             let hwid: [u8; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

//             packet.write_short(1);
//             packet.write_str_with_length(&user);
//             packet.write_str_with_length(&pw);
//             packet.write_bytes(&zeros);
//             packet.write_bytes(&hwid);

//             match handler.handle(&packet) {
//                 Ok(_) => (),
//                 Err(e) => panic!(e),
//             };
//         }
//     }
// }
