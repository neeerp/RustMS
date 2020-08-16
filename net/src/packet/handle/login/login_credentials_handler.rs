use crate::{
    error::NetworkError,
    helpers::to_hex_string,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct LoginCredentialsHandler {}

/// A handler for login attempt packets.
impl LoginCredentialsHandler {
    pub fn new() -> Self {
        LoginCredentialsHandler {}
    }

    fn echo_details(&self, packet: &mut Packet) {
        let mut reader = BufReader::new(&**packet);

        // prune opcode
        reader.read_short().unwrap();

        let user = reader.read_str_with_length().unwrap();
        let pw = reader.read_str_with_length().unwrap();

        // The next 6 bytes should be zero'd out
        reader.read_bytes(6).unwrap();

        let hwid_nibble = reader.read_bytes(4).unwrap();

        println!("Username: {}", &user);
        println!("Password: {}", &pw);
        println!("HWID nibble: {}", to_hex_string(&hwid_nibble.to_vec()));
    }

    /// Send a login failed packet with the "Not registered" reason
    fn deny_logon(&self, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Denying logon...");

        let mut return_packet = build::login::status::build_login_status_packet(5);
        match client.send(&mut return_packet) {
            Ok(_) => {
                println!("Logon denied packet sent.");
                Ok(())
            }
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }

    fn accept_logon(&self, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Logging in!");

        let mut return_packet = build::login::status::build_successful_login_packet();
        match client.send(&mut return_packet) {
            Ok(_) => {
                println!("Logon success packet sent.");
                Ok(())
            }
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }
}

impl PacketHandler for LoginCredentialsHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Login attempted...");
        self.echo_details(packet);

        let logged_in = true;

        if logged_in {
            self.accept_logon(client)
        } else {
            self.deny_logon(client)
        }
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
