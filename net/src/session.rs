use crypt::MapleAES;
use crypt::maple_crypt;

use packet::{MaplePacket, MAX_PACKET_LENGTH};
use std::net::TcpStream;
use std::io::{Write, Read};
use std::time::Duration;

use crate::error::NetworkError;

use bufstream::BufStream;
use rand::{Rng, thread_rng};

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
        stream.set_read_timeout(Some(Duration::from_secs(90))).expect("Could not set read timeout");
        stream.set_write_timeout(Some(Duration::from_secs(10))).expect("Could not set write timeout");

        // Initialization vectors that would be used for encryption... They're hardcoded though
        let (recv_iv, send_iv) = Session::generate_ivs();

        let handshake_packet = Session::build_handshake_packet(&recv_iv, &send_iv);
        match stream.write(handshake_packet.get_bytes()) {
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
    fn build_handshake_packet(recv_iv: &Vec<u8>, send_iv: &Vec<u8>) -> MaplePacket {
        let mut packet = MaplePacket::new();

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

    /// Read a new packet header from the session stream and determine its
    /// length.
    fn read_packet_header(&mut self) -> Result<i16, NetworkError> {
        let mut header_buf: [u8; 4] = [0u8; 4];

        match self.stream.read_exact(&mut header_buf) {
            Ok(_) => self.parse_header(&header_buf), 
            Err(e) => Err(NetworkError::CouldNotReadHeader(e)),
        }
    }

    /// Parse the packet header and return the length of the incoming packet.
    fn parse_header(&mut self, header_buf: &[u8; 4]) -> Result<i16, NetworkError> {
        if self.recv_crypt.check_header(&header_buf[..]) {
            let length = self.recv_crypt.get_packet_length(&header_buf[..]);

            Session::validate_packet_length(length)
        } else {
            Err(NetworkError::InvalidHeader)
        }
    }

    /// Check that the given length value neither exceeds the maximum packet
    /// length nor is too short to contain an opcode.
    fn validate_packet_length(length: i16) -> Result<i16, NetworkError> {
        if length < 2 || length > MAX_PACKET_LENGTH {
            Err(NetworkError::InvalidPacketLength(length))
        } else {
            Ok(length)
        }
    }


    /// Read a packet from the session stream given a `parsed_len` from a
    /// packet header that was just read.
    fn read_packet(&mut self, parsed_len: i16) -> Result<(), NetworkError> {
        let mut buf = vec![0u8; parsed_len as usize];
        match self.stream.read_exact(&mut buf) {
            Ok(_) => {
                // Decrypt incoming packet
                self.recv_crypt.crypt(&mut buf[..]);
                maple_crypt::decrypt(&mut buf[..]);

                self.handle_packet(buf)
            },
            Err(_) => Err(NetworkError::InvalidPacket)
        }
    }


    /// Deal with the packet data by printing it out.
    fn handle_packet(&mut self, buf: Vec<u8>) -> Result<(), NetworkError> {
        // TODO: Implement handlers that we delegate to based off opcode
        println!("Opcode byte values: {} {}", buf[0], buf[1]);
        println!("Received packet: {}", to_hex_string(buf));

        Ok(())
    }


    /// Read packets from the session stream.
    fn read_from_stream(&mut self) -> Result<(), NetworkError> {
        match self.read_packet_header() {
            Ok(len) => self.read_packet(len),
            Err(e) => Err(e)
        }
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
}

// Helper method to print out received packets
fn to_hex_string(bytes: Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join(" ")
}
