use crate::error::NetworkError;
use crate::helpers::to_hex_string;
use crate::io::client::MapleClient;
use crate::packet::build::login::world;
use crate::packet::handle::PacketHandler;
use packet::Packet;

pub struct ServerStatusHandler {}

impl ServerStatusHandler {
    pub fn new() -> ServerStatusHandler {
        ServerStatusHandler {}
    }
}

impl PacketHandler for ServerStatusHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        to_hex_string(&packet.bytes);

        client.send(&mut world::build_server_status(1)?)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncServerStatusHandler;

impl AsyncServerStatusHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncServerStatusHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let status_packet = world::build_server_status(1)?;
        Ok(HandlerResult::reply(status_packet))
    }
}
