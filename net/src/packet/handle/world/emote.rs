use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::field::build_facial_expression;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// Emotes F1-F7 are free; higher ones need an emotion cash item.
const FREE_EMOTES: std::ops::RangeInclusive<i32> = 1..=7;

/// FACE_EXPRESSION: show the player's emote to the rest of the map.
pub struct FaceExpressionHandler;

impl WorldHandler for FaceExpressionHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let emote = reader.read_int()?;

        if !FREE_EMOTES.contains(&emote) {
            return Ok(HandlerResult::empty());
        }

        let shown = build_facial_expression(ctx.player.id(), emote)?;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(emote: i32) -> Vec<WorldAction> {
        let mut player = crate::codec::stats::sample_player();
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::FaceExpression as i16)
            .unwrap();
        packet.write_int(emote).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        FaceExpressionHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn free_emotes_are_shown_to_the_map() {
        for emote in 1..=7 {
            assert!(matches!(
                handle(emote).as_slice(),
                [WorldAction::Field(FieldRequest::Broadcast(_))]
            ));
        }
        assert!(handle(0).is_empty());
        assert!(handle(8).is_empty());
    }
}
