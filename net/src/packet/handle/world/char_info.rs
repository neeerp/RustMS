use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `CHAR_INFO_REQUEST`: tick, character id. The player double-clicked a
/// character (maybe themselves); their field finds it and has it answer.
pub struct CharInfoRequestHandler;

impl WorldHandler for CharInfoRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let character_id = reader.read_int()?;
        Ok(
            HandlerResult::empty()
                .with(WorldAction::Field(FieldRequest::CharInfo { character_id })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn asks_the_field_for_the_character() {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::CharInfoRequest as i16).unwrap();
        p.write_int(1234).unwrap();
        p.write_int(62).unwrap();
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = CharInfoRequestHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions;
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::Field(FieldRequest::CharInfo {
                character_id: 62
            })]
        ));
    }
}
