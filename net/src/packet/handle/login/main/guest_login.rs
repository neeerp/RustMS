use crate::{
    error::NetworkError,
    io::client::MapleClient,
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
        client
            .send(&mut build::login::status::build_guest_login_packet())
            .unwrap();

        get_handler(RecvOpcode::LoginCredentials as i16).handle(packet, client)
    }
}
