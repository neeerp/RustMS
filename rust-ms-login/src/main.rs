use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::exit;
use std::thread;

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
    // Handshake with a v28 game client - Following Hucaru's start here
    let handshake_packet: [u8; 15] = [13, 0, 28, 0, 0, 0, 28, 62, 13, 176, 236, 76, 141, 116, 8];
    match stream.write(&handshake_packet) {
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
