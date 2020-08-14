use super::PacketHandler;
use crate::{error::NetworkError, helpers::to_hex_string, io::client::MapleClient};
use packet::Packet;

pub struct DefaultHandler {}

/// A default handler that echoes the packet in the console and throws an error.
impl DefaultHandler {
    pub fn new() -> Self {
        DefaultHandler {}
    }
}

impl PacketHandler for DefaultHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let op = packet.opcode();
        println!("Opcode: {}", op);
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Err(NetworkError::UnsupportedOpcodeError(op))
    }
}
