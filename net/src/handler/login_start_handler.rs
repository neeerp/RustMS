use crate::error::NetworkError;
use packet::Packet;

pub struct LoginStartHandler {}

impl LoginStartHandler {
    pub fn new() -> Self {
        LoginStartHandler {}
    }

    pub fn handle(&self, packet: &Packet) -> Result<(), NetworkError> {
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
