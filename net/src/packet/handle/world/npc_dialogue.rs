use crate::dialogue::NpcInteraction;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct NpcTalkHandler;
pub struct NpcTalkMoreHandler;

impl WorldHandler for NpcTalkHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let object_id = reader.read_int()?;
        Ok(
            HandlerResult::empty().with(WorldAction::NpcInteraction(NpcInteraction::Start {
                object_id,
                request_id: 0,
            })),
        )
    }
}

impl WorldHandler for NpcTalkMoreHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let message_type = reader.read_byte()?;
        let response = reader.read_byte()? as i8;
        // A menu answers with the choice, a style picker with its index.
        let selection = match (message_type, response) {
            (4, 1) => Some(reader.read_int()?),
            (7, 1) => Some(i32::from(reader.read_byte()?)),
            _ => None,
        };
        // A text prompt answers with what was typed.
        let text = match (message_type, response) {
            (2, 1) => Some(reader.read_str_with_length()?),
            _ => None,
        };
        Ok(
            HandlerResult::empty().with(WorldAction::NpcInteraction(NpcInteraction::Respond {
                message_type,
                response,
                selection,
                text,
            })),
        )
    }
}
