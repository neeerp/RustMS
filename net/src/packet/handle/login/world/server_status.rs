use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::login_world::load_login_worlds;
use crate::packet::build::login::world;
use packet::Packet;

pub struct ServerStatusHandler;

impl ServerStatusHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for ServerStatusHandler {
    fn handle(
        &self,
        _packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let worlds = load_login_worlds()?;
        let status = if worlds.iter().any(|world| !world.channels.is_empty()) {
            0
        } else {
            2
        };
        let status_packet = world::build_server_status(status)?;
        Ok(HandlerResult::reply(status_packet))
    }
}
