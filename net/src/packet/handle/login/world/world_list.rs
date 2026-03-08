use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::login::world;
use packet::Packet;

pub struct WorldListHandler;

impl WorldListHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for WorldListHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut packets = world::build_world_list_packets()?;
        packets.push(world::build_end_of_world_list()?);
        packets.push(world::build_select_world()?);
        packets.push(world::build_send_recommended_worlds()?);
        Ok(HandlerResult::replies(packets))
    }
}
