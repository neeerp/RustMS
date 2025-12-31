use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CharacterSelectHandler;

impl CharacterSelectHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for CharacterSelectHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);

        let _op = reader.read_short()?;
        let cid = reader.read_int()?;
        let _mac = reader.read_str_with_length();
        let _hwid = reader.read_str_with_length();

        println!("Redirecting to port 8485!");
        let redirect_packet = build::login::world::build_server_redirect(cid)?;

        // Attach character to session and send redirect
        Ok(HandlerResult::empty()
            .with_attach_character(cid)
            .with_reply(redirect_packet))
    }
}
