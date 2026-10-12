use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::login_world::load_login_worlds;
use crate::packet::build::login::world;
use packet::Packet;

pub struct ServerStatusHandler;

impl LoginHandler for ServerStatusHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &LoginContext) -> LoginResult {
        let worlds = load_login_worlds()?;
        let status = if worlds.iter().any(|world| !world.channels.is_empty()) {
            0
        } else {
            2
        };
        Ok(HandlerResult::reply(world::build_server_status(status)?))
    }
}
