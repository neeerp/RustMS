use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::helpers::to_hex_string;
use packet::Packet;

pub struct PlayerMapTransferHandler;

impl PlayerMapTransferHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for PlayerMapTransferHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Ok(HandlerResult::empty())
    }
}
