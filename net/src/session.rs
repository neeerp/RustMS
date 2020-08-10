use crypt::MapleAES;

use packet::io::PktWrite;
use packet::Packet;

use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use crate::accept;
use crate::{
    error::NetworkError,
    handler::{LoginCredentialsHandler, LoginStartHandler},
    helpers::to_hex_string,
};

use bufstream::BufStream;
use rand::{thread_rng, Rng};

pub struct Session {
    pub stream: BufStream<TcpStream>,
    pub recv_crypt: MapleAES,
    pub send_crypt: MapleAES,
}

impl Session {
    /// Instantiate a new maplestory client session, generating encryption
    /// IVs in the process.
    pub fn new(mut stream: TcpStream) -> Session {
        // Set timeouts on stream so that IO does not block for too long
        stream
            .set_read_timeout(Some(Duration::from_secs(90)))
            .expect("Could not set read timeout");
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .expect("Could not set write timeout");

        // Initialization vectors that would be used for encryption... They're hardcoded though
        let (recv_iv, send_iv) = Session::generate_ivs();

        let handshake_packet = Session::build_handshake_packet(&recv_iv, &send_iv);
        match stream.write(&handshake_packet) {
            Ok(_) => println!("Handshake sent"),
            Err(e) => panic!("Could not send Handshake: {}", e),
        }

        let recv_crypt = MapleAES::new(recv_iv, 83);
        let send_crypt = MapleAES::new(send_iv, 83);

        let stream = BufStream::new(stream);

        Session {
            stream,
            recv_crypt,
            send_crypt,
        }
    }

    /// Generate a pair of encryption IVs.
    fn generate_ivs() -> (Vec<u8>, Vec<u8>) {
        let mut recv_iv: Vec<u8> = vec![0u8; 4];
        let mut send_iv: Vec<u8> = vec![0u8; 4];

        let mut rng = thread_rng();
        rng.fill(&mut recv_iv[..]);
        rng.fill(&mut send_iv[..]);

        (recv_iv, send_iv)
    }

    /// Build the handshake_packet which shares the encryption IVs with the
    /// client.
    fn build_handshake_packet(recv_iv: &Vec<u8>, send_iv: &Vec<u8>) -> Packet {
        let mut packet = Packet::new_empty();

        packet.write_short(0x0E); // Packet length header
        packet.write_short(83); // Version

        // Not sure what this part is meant to represent...
        // HeavenClient doesn't seem to care for these values but the
        // official clients do...
        packet.write_short(0);
        packet.write_byte(0);

        packet.write_bytes(&recv_iv);
        packet.write_bytes(&send_iv);
        packet.write_byte(8); // Locale byte

        packet
    }

    /// Listen for packets being sent from the client via the session stream.
    pub fn listen(&mut self) {
        loop {
            match self.read_from_stream() {
                Ok(_) => continue,
                Err(NetworkError::NoData) => continue,
                Err(e) => {
                    println!("{}", e);
                    break;
                }
            }
        }
    }

    /// Read packets from the session stream.
    fn read_from_stream(&mut self) -> Result<(), NetworkError> {
        match accept::read_packet(&mut self.stream, &mut self.recv_crypt) {
            Ok(packet) => self.handle_packet(packet),
            Err(e) => Err(e),
        }
    }

    /// Deal with the packet data by printing it out.
    fn handle_packet(&mut self, packet: Packet) -> Result<(), NetworkError> {
        // TODO: Implement handlers that we delegate to based off opcode

        let start_handler = LoginStartHandler::new();
        let credential_handler = LoginCredentialsHandler::new();

        match packet.opcode() {
            1 => credential_handler.handle(&packet),
            35 => start_handler.handle(&packet),
            op => {
                println!("Opcode: {}", packet.opcode());
                println!("Received packet: {}", to_hex_string(packet.bytes));
                Err(NetworkError::UnsupportedOpcodeError(op))
            }
        }
    }
}
