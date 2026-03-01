use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build;
use packet::Packet;

pub struct GuestLoginHandler;

impl GuestLoginHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for GuestLoginHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        // Guest login sends the guest packet, then delegates to login handler
        let guest_packet = build::login::status::build_guest_login_packet()?;
        let mut result = HandlerResult::reply(guest_packet);

        // Delegate to login handler
        use super::login::LoginCredentialsHandler;
        let login_result = LoginCredentialsHandler::new().handle(packet, ctx)?;
        result.actions.extend(login_result.actions);
        Ok(result)
    }
}
