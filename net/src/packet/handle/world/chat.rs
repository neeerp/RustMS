use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::field::build_local_chat;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// Longest chat line the v83 client lets players type.
const MAX_CHAT_LEN: usize = 70;

pub struct AllChatHandler;

impl WorldHandler for AllChatHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;

        let msg = reader.read_str_with_length()?;
        let show = reader.read_byte()?;

        if msg.is_empty() || msg.len() > MAX_CHAT_LEN {
            return Ok(HandlerResult::empty());
        }

        let chat_packet = build_local_chat(ctx.client_id, &msg, false, show)?;
        Ok(HandlerResult::empty().with(WorldAction::FieldChat(chat_packet)))
    }
}
