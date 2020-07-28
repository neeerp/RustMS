use crypt::MapleAES;
use crypt::maple_crypt;

use packet::MaplePacket;
use std::net::TcpStream;
use std::io::{BufReader, Write, Read};

pub struct Session {
    pub stream: TcpStream,
    pub recv_crypt: MapleAES,
    pub send_crypt: MapleAES,
}

impl Session {
    pub fn new(mut stream: TcpStream) -> Session {

        // Initialization vectors that would be used for encryption... They're hardcoded though
        let recv_iv: Vec<u8> = vec![28, 62, 13, 176];
        let send_iv: Vec<u8> = vec![236, 76, 141, 116];

        let handshake_packet = Session::build_handshake_packet(&recv_iv, &send_iv);
        match stream.write(handshake_packet.get_bytes()) {
            Ok(_) => println!("Handshake sent"),
            Err(e) => panic!("Could not send Handshake: {}", e),
        }
        
        let recv_crypt = MapleAES::new(recv_iv, 83);
        let send_crypt = MapleAES::new(send_iv, 83);

        Session {
            stream,
            recv_crypt,
            send_crypt,
        }
    }

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

    pub fn handle(&mut self) {
        let mut reader = BufReader::new(&self.stream);

        loop {
            let mut header_buf: [u8; 4] = [0u8; 4];
            match reader.read(&mut header_buf) {
                Ok(hlen) => {
                    if hlen == 4 && self.recv_crypt.check_header(&header_buf[0..4]) {
                        let length: i16 = self.recv_crypt.get_packet_length(&header_buf[0..4]);
                        println!("Packet header: length {}", length);

                        if length < 0 {
                            println!("Invalid packet length!");
                            break;
                        }

                        let mut buf = vec![0u8; length as usize];
                        match reader.read(&mut buf) {
                            Ok(len) => {
                                if len != length.max(0) as usize {
                                    println!("Actual length {} does not match reported length {}", len, length);
                                }

                                self.recv_crypt.crypt(&mut buf[..len]);
                                maple_crypt::decrypt(&mut buf[..len]);
                                println!("Opcode byte values: {} {}", buf[0], buf[1]);
                                println!("Received packet: {}", to_hex_string(buf[..len].to_vec()));

                            },
                            Err(e) => {
                                println!("{}", e);
                                break;
                            }
                        };
                    } else if hlen > 0 {
                        println!("Could not interpret header: {}", to_hex_string(header_buf[..hlen].to_vec()));
                    }
                },
                Err(_) => break,
            };
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
