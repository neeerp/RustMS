use crate::{client::MapleClient, error::NetworkError};
use packet::Packet;

mod default;
mod login;

pub trait PacketHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError>;
}

/// Get the packet handler corresponding to the given opcode.
pub fn get_handler(op: i16) -> Box<dyn PacketHandler> {
    match op {
        1 => Box::new(login::LoginCredentialsHandler::new()),
        35 => Box::new(login::LoginStartHandler::new()),
        _ => Box::new(default::DefaultHandler::new()),
    }
}
