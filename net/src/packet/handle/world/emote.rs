use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::field::build_facial_expression;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// Emotes F1-F7 are free; higher ones need an emotion cash item.
const FREE_EMOTES: std::ops::RangeInclusive<i32> = 1..=7;
/// The face items (5160000 Queasy to 5160014 Bleh) give emotes 8 to 22.
const ITEM_EMOTES: std::ops::RangeInclusive<i32> = 8..=22;
const FACE_ITEM_BASE: i32 = 5_159_992;

/// Whether the player may show `emote`: free, from a face item they own, or
/// from the cash mist they used, whose face is the next one they send.
fn can_show(player: &mut game::Player, emote: i32) -> bool {
    let from_mist = mist_face(player, emote);
    from_mist
        || FREE_EMOTES.contains(&emote)
        || (ITEM_EMOTES.contains(&emote) && player.inventory.count(FACE_ITEM_BASE + emote) > 0)
}

/// A cash mist's user pulls one of its faces, which their client picks
/// (ITEM_EMOTION, FUN_00938b3e) and sends next.
fn mist_face(player: &mut game::Player, emote: i32) -> bool {
    player
        .mist_face
        .take()
        .and_then(|item_id| crate::items::info(item_id)?.mist)
        .is_some_and(|mist| mist.has_face(emote))
}

/// FACE_EXPRESSION: show the player's emote to the rest of the map.
pub struct FaceExpressionHandler;

impl WorldHandler for FaceExpressionHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let emote = reader.read_int()?;

        if !can_show(ctx.player, emote) {
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
        handle_as(crate::codec::stats::sample_player(), emote)
    }

    fn handle_as(mut player: game::Player, emote: i32) -> Vec<WorldAction> {
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

    #[test]
    fn item_emotes_need_their_face_item() {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            game::InventoryType::Cash,
            1,
            game::Item::stack(5_160_006, 1),
        );

        assert!(matches!(
            handle_as(player.clone(), 14).as_slice(),
            [WorldAction::Field(FieldRequest::Broadcast(_))]
        ));
        assert!(handle_as(player.clone(), 8).is_empty());
        assert!(handle_as(player, 23).is_empty());
    }

    #[test]
    fn a_cash_mist_lends_one_of_its_faces_once() {
        if crate::items::info(5_281_000).is_none() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.mist_face = Some(5_281_000);
        // Passed Gas pulls vomit (8), not love (17).
        assert!(handle_as(player.clone(), 17).is_empty());
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::FaceExpression as i16)
            .unwrap();
        packet.write_int(8).unwrap();
        let actions = FaceExpressionHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::Field(FieldRequest::Broadcast(_))]
        ));
        assert_eq!(player.mist_face, None);
        assert!(handle_as(player.clone(), 8).is_empty());
        // A free face sent first spends it too.
        player.mist_face = Some(5_281_000);
        assert!(!handle_as(player.clone(), 5).is_empty());
        let mut cloned = player.clone();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut cloned,
        };
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::FaceExpression as i16)
            .unwrap();
        packet.write_int(5).unwrap();
        FaceExpressionHandler.handle(&mut packet, &mut ctx).unwrap();
        assert_eq!(cloned.mist_face, None);
    }
}
