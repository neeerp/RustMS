use crate::{client::MapleClient, error::NetworkError, packet::handle::PacketHandler};
use packet::Packet;

pub struct LoginStartHandler {}

/// A handler for the empty 'login started' packet.
impl LoginStartHandler {
    pub fn new() -> Self {
        LoginStartHandler {}
    }

    fn check_length(&self, packet: &Packet) -> Result<(), NetworkError> {
        if packet.len() != 0 {
            Err(NetworkError::PacketHandlerError(
                "Start login packet has invalid length.",
            ))
        } else {
            println!("Login started.");
            Ok(())
        }
    }
}

impl PacketHandler for LoginStartHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        self.check_length(packet)
    }
}
