use super::login::LoginCredentialsHandler;
use crate::handler::{HandlerResult, LoginContext, LoginHandler, LoginResult};
use crate::packet::build;
use packet::Packet;

pub struct GuestLoginHandler;

impl LoginHandler for GuestLoginHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        // Send the guest packet, then continue as a normal login.
        let mut result = HandlerResult::reply(build::login::status::build_guest_login_packet()?);
        result
            .actions
            .extend(LoginCredentialsHandler.handle(packet, ctx)?.actions);
        Ok(result)
    }
}
