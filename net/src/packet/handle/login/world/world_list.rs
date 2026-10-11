use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build::login::world;
use packet::Packet;

pub struct WorldListHandler;

impl LoginHandler for WorldListHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &LoginContext) -> LoginResult {
        let mut packets = world::build_world_list_packets()?;
        packets.push(world::build_end_of_world_list()?);
        packets.push(world::build_select_world()?);
        packets.push(world::build_send_recommended_worlds()?);
        Ok(packets
            .into_iter()
            .fold(HandlerResult::empty(), HandlerResult::with_reply))
    }
}
