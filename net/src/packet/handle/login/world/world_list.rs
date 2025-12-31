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
        let packets = vec![
            world::build_world_details()?,
            world::build_end_of_world_list()?,
            world::build_select_world()?,
            world::build_send_recommended_worlds()?,
        ];
        Ok(HandlerResult::replies(packets))
    }
}
