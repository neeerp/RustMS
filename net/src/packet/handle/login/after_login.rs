use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use build::login;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct AfterLoginHandler {}

impl AfterLoginHandler {
    pub fn new() -> Self {
        AfterLoginHandler {}
    }
}

impl PacketHandler for AfterLoginHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short().unwrap();

        let c2 = reader.read_byte().unwrap();
        let mut c3 = 5u8;
        if let Ok(b) = reader.read_byte() {
            c3 = b;
        };

        if c2 == 1 && c3 == 1 {
            client.send(&mut login::pin::build_pin_register()).unwrap();
        } else if c2 == 1 && c3 == 0 {
            let _pin = reader.read_str_with_length().unwrap();

            client.send(&mut login::pin::build_pin_accepted()).unwrap();
        } else if c2 == 0 && c3 == 0 {
            let _pin = reader.read_str_with_length().unwrap();

            client.send(&mut login::pin::build_pin_register()).unwrap();
        } else if c2 == 0 && c3 == 5 {
            // TODO: Should the DC the client here?
        }
        Ok(())
    }
}
