use crate::dialogue::NpcInteraction;
use crate::error::NetworkError;
use crate::handler::{HandlerAction, HandlerContext, HandlerResult, PacketHandler};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct NpcTalkHandler;
pub struct NpcTalkMoreHandler;

impl PacketHandler for NpcTalkHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let object_id = reader.read_int()?;
        Ok(HandlerResult {
            actions: vec![HandlerAction::NpcInteraction(NpcInteraction::Start {
                object_id,
                request_id: 0,
            })],
        })
    }
}

impl PacketHandler for NpcTalkMoreHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let message_type = reader.read_byte()?;
        let response = reader.read_byte()? as i8;
        let selection = if message_type == 4 && response == 1 {
            Some(reader.read_int()?)
        } else {
            None
        };
        Ok(HandlerResult {
            actions: vec![HandlerAction::NpcInteraction(NpcInteraction::Respond {
                message_type,
                response,
                selection,
            })],
        })
    }
}
