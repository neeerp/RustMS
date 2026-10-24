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

        // A GM's lines show on the white strip.
        let gm = ctx.player.gm_level > 0;
        let chat_packet = build_local_chat(ctx.client_id, &msg, gm, show)?;
        Ok(HandlerResult::empty().with(WorldAction::FieldChat(chat_packet)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    /// The GM byte of the line the map is sent, or None if nothing is sent.
    fn gm_byte(gm_level: i16, msg: &str) -> Option<u8> {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = gm_level;
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::AllChat as i16).unwrap();
        packet.write_str_with_length(msg).unwrap();
        packet.write_byte(0).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = AllChatHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        match actions.as_slice() {
            // Opcode, then the character id, then the GM byte.
            [WorldAction::FieldChat(line)] => Some(line.bytes[6]),
            [] => None,
            _ => panic!("unexpected actions"),
        }
    }

    #[test]
    fn a_gms_lines_carry_the_gm_flag() {
        assert_eq!(gm_byte(0, "hi"), Some(0));
        assert_eq!(gm_byte(1, "hi"), Some(1));
    }

    #[test]
    fn empty_and_overlong_lines_are_dropped() {
        assert_eq!(gm_byte(0, ""), None);
        assert_eq!(gm_byte(0, &"a".repeat(MAX_CHAT_LEN + 1)), None);
    }
}
