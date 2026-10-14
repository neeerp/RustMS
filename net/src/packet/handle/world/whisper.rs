use crate::handler::{
    HandlerResult, WhisperDelivery, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::messaging::{
    build_whisper_receive, build_whisper_result, FIND_REQUEST_MODE, WHISPER_REQUEST_MODE,
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
            sender_success_packet,
            sender_failure_packet,
        })),
    )
}
