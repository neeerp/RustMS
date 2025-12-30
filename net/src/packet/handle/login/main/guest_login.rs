use crate::{
    error::NetworkError,
    io::client::MapleClient,
    listener::ServerType,
    packet::{
        build,
        handle::{get_handler, PacketHandler},
        op::RecvOpcode,
    },
};
use packet::Packet;

pub struct GuestLoginHandler {}

/// A handler for guest logins...?
impl GuestLoginHandler {
    pub fn new() -> Self {
        GuestLoginHandler {}
    }
}

impl PacketHandler for GuestLoginHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        client.send(&mut build::login::status::build_guest_login_packet()?)?;

        get_handler(RecvOpcode::LoginCredentials as i16, &ServerType::Login).handle(packet, client)
    }
}

// === ASYNC HANDLER ===
use crate::handler::{AsyncPacketHandler, HandlerContext, HandlerResult};

pub struct AsyncGuestLoginHandler;

impl AsyncGuestLoginHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AsyncPacketHandler for AsyncGuestLoginHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        // Guest login sends the guest packet, then delegates to login handler
        let guest_packet = build::login::status::build_guest_login_packet()?;
        let mut result = HandlerResult::reply(guest_packet);

        // Delegate to login handler
        use super::login::AsyncLoginCredentialsHandler;
        let login_result = AsyncLoginCredentialsHandler::new().handle(packet, ctx)?;
        result.actions.extend(login_result.actions);
        Ok(result)
    }
}
