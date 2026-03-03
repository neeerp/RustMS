use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::world::messaging::{
    build_whisper_receive, build_whisper_result, WHISPER_REQUEST_MODE,
};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct WhisperHandler;

impl WhisperHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for WhisperHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        let _op = reader.read_short()?;
        let request_mode = reader.read_byte()?;
        if request_mode != WHISPER_REQUEST_MODE {
            return Ok(HandlerResult::empty());
        }

        let target_name = reader.read_str_with_length()?;
        let message = reader.read_str_with_length()?;
        if target_name.is_empty() || message.is_empty() {
            return Ok(HandlerResult::empty());
        }

        let sender_name = {
            let character = ctx.session.get_character().map_err(|_| {
                NetworkError::PacketHandlerError("Whisper requires a loaded character")
            })?;
            let character = character
                .lock()
                .map_err(|_| NetworkError::PacketHandlerError("Failed to lock whisper sender"))?;
            character.character.name.clone()
        };

        let sender_failure_packet = build_whisper_result(&target_name, false)?;
        if sender_name == target_name {
            return Ok(HandlerResult::reply(sender_failure_packet));
        }

        let recipient_packet = build_whisper_receive(&sender_name, 1, false, &message)?;
        let sender_success_packet = build_whisper_result(&target_name, true)?;

        Ok(HandlerResult::empty().with_whisper(
            target_name,
            recipient_packet,
            sender_success_packet,
            sender_failure_packet,
        ))
    }
}
