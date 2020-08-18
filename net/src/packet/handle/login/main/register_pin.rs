use crate::{
    error::NetworkError,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use build::login;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct RegisterPinHandler {}

impl RegisterPinHandler {
    pub fn new() -> Self {
        RegisterPinHandler {}
    }
}

impl PacketHandler for RegisterPinHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short().unwrap();

        let c2 = reader.read_byte().unwrap();
        if c2 == 0 {
            // TODO: Should the DC the client here?
        } else {
            let _pin = reader.read_str_with_length().unwrap();

            client.send(&mut login::pin::build_pin_updated()).unwrap();
            // TODO: Should the DC the client here?
        }
        Ok(())
    }
}
