use crate::handler::{
    HandlerResult, WhisperDelivery, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::messaging::{
    build_whisper_blocked, build_whisper_receive, build_whisper_result, FIND_REQUEST_MODE,
    WHISPER_BLOCKED_MODE, WHISPER_REQUEST_MODE,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct WhisperHandler;

impl WorldHandler for WhisperHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        match reader.read_byte()? {
            WHISPER_REQUEST_MODE => whisper(&mut reader, ctx),
            FIND_REQUEST_MODE => {
                let target_name = reader.read_str_with_length()?;
                if target_name.is_empty() {
                    return Ok(HandlerResult::empty());
                }
                Ok(HandlerResult::empty().with(WorldAction::Find { target_name }))
            }
            WHISPER_BLOCKED_MODE => blocked(&mut reader, ctx),
            _ => Ok(HandlerResult::empty()),
        }
    }
}

fn whisper(reader: &mut Cursor<&Vec<u8>>, ctx: &mut WorldContext) -> WorldResult {
    let target_name = reader.read_str_with_length()?;
    let message = reader.read_str_with_length()?;
    if target_name.is_empty() || message.is_empty() {
        return Ok(HandlerResult::empty());
    }

    let sender_failure_packet = build_whisper_result(&target_name, false)?;
    if ctx.player.name() == target_name {
        return Ok(HandlerResult::reply(sender_failure_packet));
    }

    let recipient_packet =
        build_whisper_receive(ctx.player.name(), ctx.channel_id, false, &message)?;
    let sender_success_packet = build_whisper_result(&target_name, true)?;

    Ok(
        HandlerResult::empty().with(WorldAction::Whisper(WhisperDelivery {
            target_name,
            recipient_packet,
            sender_success_packet: Some(sender_success_packet),
            sender_failure_packet: Some(sender_failure_packet),
        })),
    )
}

/// The player's Game Option turned away a whisper from `sender`, who hears
/// so (FUN_0053228e); the player hears nothing back.
fn blocked(reader: &mut Cursor<&Vec<u8>>, ctx: &mut WorldContext) -> WorldResult {
    let target_name = reader.read_str_with_length()?;
    if target_name.is_empty() || ctx.player.name() == target_name {
        return Ok(HandlerResult::empty());
    }
    Ok(
        HandlerResult::empty().with(WorldAction::Whisper(WhisperDelivery {
            target_name,
            recipient_packet: build_whisper_blocked(ctx.player.name())?,
            sender_success_packet: None,
            sender_failure_packet: None,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn a_blocked_whisper_tells_only_its_sender() {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::Whisper as i16).unwrap();
        packet.write_byte(WHISPER_BLOCKED_MODE).unwrap();
        packet.write_str_with_length("Bobby").unwrap();
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = WhisperHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        let [WorldAction::Whisper(delivery)] = &actions[..] else {
            panic!("expected a delivery: {actions:?}");
        };
        assert_eq!(delivery.target_name, "Bobby");
        assert_eq!(
            delivery.recipient_packet.bytes,
            build_whisper_blocked("Tester").unwrap().bytes
        );
        assert!(delivery.sender_success_packet.is_none());
        assert!(delivery.sender_failure_packet.is_none());
    }
}
