use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::world::field::build_local_chat;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct AllChatHandler;

impl AllChatHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for AllChatHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;

        let msg = reader.read_str_with_length()?;
        let show = reader.read_byte()?;

        if ctx.client_id == 0 || msg.is_empty() {
            return Ok(HandlerResult::empty());
        }

        let chat_packet = build_local_chat(ctx.client_id, &msg, false, show)?;
        Ok(HandlerResult::empty().with_field_chat(chat_packet))
    }
}
