use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::exit;
use std::thread;

use packet::MaplePacket;

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
    loop {
        let mut buf: [u8; 1] = [0];
        match reader.read_exact(&mut buf) {
            Ok(_) => println!("Read bytes {:?}", buf),
            Err(_) => break,
        };
    }

    println!("Connection terminated");
}

// Handshake with a v28 game client
fn build_handshake_packet() -> MaplePacket {
    let mut packet = MaplePacket::new();

    packet.write_short(0x0E); // Packet length header
    packet.write_short(28); // Version

    // Not sure what this is meant to represent
    // Interestingly enough, both 00 00 and 01 00 XX
    // will work as long as length header changes accordingly
    // - Valhalla does the former, HeavenMS the latter... 
    //
    // Changing the first 2 bytes breaks things, making the client
    // spit out a normal client error with some bad unicode. When the packet
    // is 14 bytes however, setting the first byte to 0 causes the client
    // to seg fault.
    packet.write_short(1);
    packet.write_byte(49);

    // Initialization vectors that would be used for encryption... They're hardcoded though
    let recv_iv: [u8; 4] = [28, 62, 13, 176];
    let send_iv: [u8; 4] = [236, 76, 141, 116];

    packet.write_bytes(&recv_iv);
    packet.write_bytes(&send_iv);
    packet.write_byte(8); // Locale byte

    packet
}
