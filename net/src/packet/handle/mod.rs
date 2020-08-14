use super::op::RecvOpcode;
use crate::{error::NetworkError, io::client::MapleClient};
use packet::Packet;

mod default;
mod login;

pub trait PacketHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError>;
}

/// Get the packet handler corresponding to the given opcode.
pub fn get_handler(op: i16) -> Box<dyn PacketHandler> {
    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::LoginCredentials) => Box::new(login::LoginCredentialsHandler::new()),
        Some(RecvOpcode::LoginStarted) => Box::new(login::LoginStartHandler::new()),
        None => Box::new(default::DefaultHandler::new()),
    }
}
