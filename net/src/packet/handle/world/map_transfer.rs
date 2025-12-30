use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;
use std::io::BufReader;

pub struct PlayerMapTransferHandler {}

impl PlayerMapTransferHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for PlayerMapTransferHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut _reader = BufReader::new(&**packet);
        println!("Received packet: {}", to_hex_string(&packet.bytes));

        Ok(())
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncPlayerMapTransferHandler;

impl AsyncPlayerMapTransferHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncPlayerMapTransferHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Ok(HandlerResult::empty())
    }
}
