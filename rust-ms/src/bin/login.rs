use std::net::{TcpListener, TcpStream};
use std::process::exit;
use std::thread;

use net::listener::ClientConnectionListener;

fn main() {
    println!("Starting up...");

    // Shut down the server somewhat gracefuly; not a fan of seeing an error on ctrl+c
    ctrlc::set_handler(move || {
        println!("Shutting down...");
        exit(0);
    })
    .expect("Error setting ctrl+c handler!");

    let listener = TcpListener::bind("0.0.0.0:8484").unwrap();

    for stream in listener.incoming() {
        println!("Incoming connection...");
        let stream = stream.unwrap();

        thread::spawn(move || {
            handle_connection(stream);
        });
    }
}

fn handle_connection(stream: TcpStream) {
    println!(
        "Connection Terminated: {}",
        ClientConnectionListener::login_server(stream)
            .and_then(|mut session| session.listen())
            .expect_err("Thread disconnects should result in error...")
    )
}
