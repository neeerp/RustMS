//! The chalkboard (537xxxx): held up through USE_CASH_ITEM, put away with
//! CLOSE_CHALKBOARD (report 179, 1.7 and 3). It isn't used up.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::field::build_chalkboard;
use crate::packet::build::world::stat::build_stat_update;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The longest note the v83 client lets a chalkboard hold.
pub const MAX_NOTE: usize = 40;

/// Hold up a chalkboard with the note `r` carries. Not where shops open
/// (the Free Market rooms), which the client checks too.
pub fn chalkboard(ctx: &mut WorldContext, r: &mut Cursor<&Vec<u8>>) -> WorldResult {
    let note = r.read_str_with_length()?;
    let unlock = build_stat_update(true, &[])?;
    let fits = !note.is_empty() && note.len() <= MAX_NOTE;
    if !fits || crate::personal_shop::allowed_on(ctx.player.stats.map_id) {
        return Ok(HandlerResult::reply(unlock));
    }
    let packet = build_chalkboard(ctx.player.stats.id, Some(&note))?;
    ctx.player.chalkboard = Some(note);
    Ok(HandlerResult::reply(packet.clone())
        .with_reply(unlock)
        .with(WorldAction::ChalkboardChanged(packet)))
}

/// CLOSE_CHALKBOARD (0x32): the player clicked their chalkboard to put it
/// away.
pub struct CloseChalkboardHandler;

impl WorldHandler for CloseChalkboardHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.chalkboard.take().is_none() {
            return Ok(HandlerResult::empty());
        }
        let packet = build_chalkboard(ctx.player.stats.id, None)?;
        Ok(HandlerResult::reply(packet.clone()).with(WorldAction::ChalkboardChanged(packet)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::{InventoryType, Item, Player};
    use packet::io::write::PktWrite;

    const CHALKBOARD: i32 = 5370000;

    fn holder() -> Player {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Cash, 3, Item::stack(CHALKBOARD, 1));
        player
    }

    fn handle(handler: &dyn WorldHandler, player: &mut Player, p: &mut Packet) -> Vec<WorldAction> {
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        handler.handle(p, &mut ctx).unwrap().actions
    }

    fn write_note(player: &mut Player, note: &str) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(3).unwrap();
        p.write_int(CHALKBOARD).unwrap();
        p.write_str_with_length(note).unwrap();
        p.write_int(1234).unwrap();
        handle(&UseCashItemHandler, player, &mut p)
    }

    fn close(player: &mut Player) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::CloseChalkboard as i16).unwrap();
        handle(&CloseChalkboardHandler, player, &mut p)
    }

    fn opcode(packet: &Packet) -> i16 {
        i16::from_le_bytes([packet.bytes[0], packet.bytes[1]])
    }

    #[test]
    fn a_chalkboard_shows_its_note_to_everyone() {
        let mut player = holder();
        let actions = write_note(&mut player, "selling");
        let [WorldAction::Reply(own), WorldAction::Reply(unlock), WorldAction::ChalkboardChanged(shown)] =
            &actions[..]
        else {
            panic!("expected the board, an unlock and the board for the map: {actions:?}");
        };
        assert_eq!(own.bytes, shown.bytes);
        assert_eq!(opcode(own), SendOpcode::Chalkboard as i16);
        assert_eq!(opcode(unlock), SendOpcode::StatChange as i16);
        assert_eq!(player.chalkboard.as_deref(), Some("selling"));
        assert_eq!(player.inventory.count(CHALKBOARD), 1, "not used up");
    }

    #[test]
    fn a_note_the_client_would_not_send_is_refused() {
        let mut player = holder();
        for note in ["", &"a".repeat(MAX_NOTE + 1)] {
            let actions = write_note(&mut player, note);
            let [WorldAction::Reply(unlock)] = &actions[..] else {
                panic!("expected a lone unlock: {actions:?}");
            };
            assert_eq!(opcode(unlock), SendOpcode::StatChange as i16);
            assert_eq!(player.chalkboard, None);
        }
    }

    #[test]
    fn closing_the_chalkboard_puts_it_away() {
        let mut player = holder();
        assert!(close(&mut player).is_empty(), "no board to put away");
        write_note(&mut player, "selling");
        let actions = close(&mut player);
        let [WorldAction::Reply(own), WorldAction::ChalkboardChanged(shown)] = &actions[..] else {
            panic!("expected the board put away for all: {actions:?}");
        };
        assert_eq!(own.bytes, shown.bytes);
        assert_eq!(own.bytes[2..], [0x04, 0x03, 0x02, 0x01, 0][..]);
        assert_eq!(player.chalkboard, None);
    }
}
