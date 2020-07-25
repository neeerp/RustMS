use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::exit;
use std::thread;

use packet::MaplePacket;
use crypt::MapleAES;
use crypt::maple_crypt;

fn main() {
    println!("Starting up...");

    // Shut down the server somewhat gracefuly; not a fan of seeing an error on ctrl+c
    ctrlc::set_handler(move || {
        println!("Shutting down...");
        exit(0);
    }).expect("Error setting ctrl+c handler!");

    let listener = TcpListener::bind("0.0.0.0:8484").unwrap();

    for stream in listener.incoming() {
        println!("Incoming connection...");
        let stream = stream.unwrap();

        thread::spawn(move || {
            handle_connection(stream);
        });
    }

}

fn handle_connection(mut stream: TcpStream) {
    let handshake_packet = build_handshake_packet();

    match stream.write(handshake_packet.get_bytes()) {
        Ok(_) => println!("Handshake sent"),
        Err(e) => println!("Could not send Handshake: {}", e),
    }

    // Spit out the bytes... Right now we can only really see the login request which we aren't yet handling.
    let mut reader = BufReader::new(stream);
    let recv_iv: [u8; 4] = [28, 62, 13, 176];
    let mut cipher = MapleAES::new(recv_iv.to_vec(), 83);

    loop {
        let mut header_buf: [u8; 4] = [0u8; 4];
        match reader.read(&mut header_buf) {
            Ok(hlen) => {
                if hlen == 4 && cipher.check_header(&header_buf[0..4]) {
                    let length: i16 = cipher.get_packet_length(&header_buf[0..4]);
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

                            cipher.crypt(&mut buf[..len]);
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
    println!("Connection terminated");
}

fn build_handshake_packet() -> MaplePacket {
    let mut packet = MaplePacket::new();

    packet.write_short(0x0E); // Packet length header
    packet.write_short(83); // Version

    // Not sure what this part is meant to represent...
    // HeavenClient doesn't seem to care for these values but the
    // official clients do...
    packet.write_short(0);
    packet.write_byte(0);

    // Initialization vectors that would be used for encryption... They're hardcoded though
    let recv_iv: [u8; 4] = [28, 62, 13, 176];
    let send_iv: [u8; 4] = [236, 76, 141, 116];

    packet.write_bytes(&recv_iv);
    packet.write_bytes(&send_iv);
    packet.write_byte(8); // Locale byte

    packet
}

// Helper method to print out received packets
fn to_hex_string(bytes: Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join(" ")
}
