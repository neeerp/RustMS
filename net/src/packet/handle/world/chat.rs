use crate::handler::{
    FieldRequest, GmEntry, GmEvent, HandlerResult, WorldAction, WorldContext, WorldHandler,
    WorldResult,
};
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
        // The client sends a '/' line only for a GM, as a command it does
        // not know itself (report b169): never as chat.
        if msg.starts_with('/') {
            let event = GmEvent::parse(&msg)
                .map(|event| WorldAction::Field(FieldRequest::GmEvent(event)))
                .or_else(|| GmEntry::parse(&msg).map(WorldAction::GmEntry))
                .filter(|_| gm);
            return Ok(event.map_or_else(HandlerResult::empty, |action| {
                HandlerResult::empty().with(action)
            }));
        }
        let chat_packet = build_local_chat(ctx.client_id, &msg, gm, show)?;
        Ok(HandlerResult::empty().with(WorldAction::FieldChat(chat_packet)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn chat(gm_level: i16, msg: &str) -> Vec<WorldAction> {
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
        AllChatHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    /// The GM byte of the line the map is sent, or None if nothing is sent.
    fn gm_byte(gm_level: i16, msg: &str) -> Option<u8> {
        match chat(gm_level, msg).as_slice() {
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

    #[test]
    fn a_gms_event_commands_go_to_the_field() {
        let event = |gm_level, msg| match chat(gm_level, msg).as_slice() {
            [WorldAction::Field(FieldRequest::GmEvent(event))] => Some(*event),
            [] => None,
            _ => panic!("unexpected actions"),
        };
        assert_eq!(event(1, "/start"), Some(GmEvent::Start));
        assert_eq!(event(1, "/reset now"), Some(GmEvent::Reset));
        assert_eq!(event(1, "/START"), Some(GmEvent::Start));
        // Other commands, and anyone else's '/' lines, are never chat.
        assert_eq!(event(1, "/divideteam"), None);
        assert_eq!(event(0, "/start"), None);
        assert_eq!(gm_byte(0, "/hello"), None);
    }

    #[test]
    fn a_gms_entry_commands_go_to_the_channel() {
        let entry = |gm_level, msg| match chat(gm_level, msg).as_slice() {
            [WorldAction::GmEntry(entry)] => Some(*entry),
            [] => None,
            _ => panic!("unexpected actions"),
        };
        let open = |map_id, limit| Some(GmEntry::Open { map_id, limit });
        assert_eq!(entry(1, "/snowball"), open(109_060_000, 50));
        assert_eq!(entry(1, "/snowball3 20"), open(109_060_004, 20));
        assert_eq!(entry(1, "/Coconut2"), open(109_080_001, 50));
        assert_eq!(entry(1, "/ox 30"), open(109_020_001, 30));
        assert_eq!(entry(1, "/fitness1"), open(109_040_000, 50));
        assert_eq!(entry(1, "/fitness4 20"), open(109_040_000, 20));
        assert_eq!(entry(1, "/fitness"), None);
        assert_eq!(entry(1, "/endevent"), Some(GmEntry::Close));
        assert_eq!(entry(1, "/snowball 0"), None);
        assert_eq!(entry(1, "/snowball4"), None);
        assert_eq!(entry(0, "/snowball"), None);
    }
}
