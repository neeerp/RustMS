use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use packet::Packet;

pub struct LoginStartHandler;

impl LoginStartHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for LoginStartHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        if packet.len() != 0 {
            return Err(NetworkError::PacketHandlerError(
                "Start login packet has invalid length.",
            ));
        }
        println!("Login started.");
        Ok(HandlerResult::empty())
    }
}
