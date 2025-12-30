use crate::packet::build::login::world;
use crate::{
    error::NetworkError, helpers::to_hex_string, io::client::MapleClient,
    packet::handle::PacketHandler,
};
use packet::Packet;

pub struct WorldListHandler {}

impl WorldListHandler {
    pub fn new() -> Self {
        WorldListHandler {}
    }
}

impl PacketHandler for WorldListHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        to_hex_string(&packet.bytes);

        client.send(&mut world::build_world_details()?)?;
        client.send(&mut world::build_end_of_world_list()?)?;
        client.send(&mut world::build_select_world()?)?; // HeavenClient ignores this...
        client.send(&mut world::build_send_recommended_worlds()?)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncWorldListHandler;

impl AsyncWorldListHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncWorldListHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let packets = vec![
            world::build_world_details()?,
            world::build_end_of_world_list()?,
            world::build_select_world()?,
            world::build_send_recommended_worlds()?,
        ];
        Ok(HandlerResult::replies(packets))
    }
}
