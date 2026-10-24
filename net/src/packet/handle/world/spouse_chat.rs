use crate::handler::{
    HandlerResult, WhisperDelivery, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::messaging::{build_spouse_chat, build_spouse_chat_result};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// A line to the player's partner (FUN_008defff). The client names the
/// partner from its own copy of the couple's record; the server goes by its
/// record instead, as the client does: engaged or married alike.
pub struct SpouseChatHandler;

impl WorldHandler for SpouseChatHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let _named = reader.read_str_with_length()?;
        let text = reader.read_str_with_length()?;
        if text.is_empty() {
            return Ok(HandlerResult::empty());
        }

        let failure = build_spouse_chat_result(None)?;
        let Some(record) = &ctx.player.marriage else {
            return Ok(HandlerResult::reply(failure));
        };
        let target_name = record.partner_name_of(ctx.player.id()).to_string();

        Ok(
            HandlerResult::empty().with(WorldAction::Whisper(WhisperDelivery {
                target_name,
                recipient_packet: build_spouse_chat(ctx.player.name(), &text)?,
                sender_success_packet: Some(build_spouse_chat_result(Some(&text))?),
                sender_failure_packet: Some(failure),
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::marriage::{MarriageRecord, ENGAGED};
    use packet::io::write::PktWrite;

    fn request(text: &str) -> Packet {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::SpouseChat as i16).unwrap();
        packet.write_str_with_length("Anyone").unwrap();
        packet.write_str_with_length(text).unwrap();
        packet
    }

    fn handle(player: &mut game::Player, text: &str) -> HandlerResult<WorldAction> {
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        SpouseChatHandler
            .handle(&mut request(text), &mut ctx)
            .unwrap()
    }

    #[test]
    fn a_line_goes_to_the_partner_on_the_servers_record() {
        let mut player = crate::codec::stats::sample_player();
        player.marriage = Some(MarriageRecord {
            id: 1,
            groom_id: 77,
            bride_id: player.id(),
            status: ENGAGED,
            item_id: 2240000,
            groom_name: "Romeo".into(),
            bride_name: player.name().to_string(),
        });
        let result = handle(&mut player, "hi");
        let [WorldAction::Whisper(delivery)] = &result.actions[..] else {
            panic!("expected a delivery: {:?}", result.actions);
        };
        assert_eq!(delivery.target_name, "Romeo");
        assert_eq!(
            delivery.recipient_packet.bytes,
            build_spouse_chat("Tester", "hi").unwrap().bytes
        );
        assert_eq!(
            delivery.sender_success_packet.as_ref().unwrap().bytes,
            build_spouse_chat_result(Some("hi")).unwrap().bytes
        );
        assert_eq!(
            delivery.sender_failure_packet.as_ref().unwrap().bytes,
            build_spouse_chat_result(None).unwrap().bytes
        );
    }

    #[test]
    fn without_a_partner_the_sender_hears_so() {
        let mut player = crate::codec::stats::sample_player();
        let result = handle(&mut player, "hi");
        let [WorldAction::Reply(packet)] = &result.actions[..] else {
            panic!("expected a reply: {:?}", result.actions);
        };
        assert_eq!(packet.bytes, build_spouse_chat_result(None).unwrap().bytes);
    }
}
