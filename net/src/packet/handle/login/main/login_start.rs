use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use packet::Packet;

pub struct LoginStartHandler;

impl LoginHandler for LoginStartHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &LoginContext) -> LoginResult {
        if packet.len() != 0 {
            return Err(NetworkError::PacketHandlerError(
                "Start login packet has invalid length.",
            ));
        }
        Ok(HandlerResult::empty())
    }
}
