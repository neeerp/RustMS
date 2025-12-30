use crate::{error::NetworkError, io::client::MapleClient, packet::handle::PacketHandler};
use packet::Packet;
use std::io::BufReader;

pub struct PlayerMoveHandler {}

impl PlayerMoveHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for PlayerMoveHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut _reader = BufReader::new(&**packet);
        // This gets annoying fast...
        // println!("Received packet: {}", to_hex_string(&packet.bytes));

        Ok(())
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncPlayerMoveHandler;

impl AsyncPlayerMoveHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncPlayerMoveHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, crate::error::NetworkError> {
        // TODO: Update player position and broadcast to map
        Ok(HandlerResult::empty())
    }
}
