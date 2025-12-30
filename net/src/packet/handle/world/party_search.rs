use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;
use std::io::BufReader;

pub struct PartySearchHandler {}

impl PartySearchHandler {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Serious implementation later...
impl PacketHandler for PartySearchHandler {
    fn handle(&self, packet: &mut Packet, _client: &mut MapleClient) -> Result<(), NetworkError> {
        let mut _reader = BufReader::new(&**packet);
        println!("Received packet: {}", to_hex_string(&packet.bytes));

        Ok(())
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncPartySearchHandler;

impl AsyncPartySearchHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncPartySearchHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Ok(HandlerResult::empty())
    }
}
