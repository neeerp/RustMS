//! QUICK_SLOT (0xB7, v83 FUN_0072b87e): the client's eight quick slot keys.
//! It sends them back after every QUICKSLOT_INIT, when Key Config opens, on
//! its OK, and when its changes are saved on closing. There is no answer.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use game::QuickSlots;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct QuickSlotHandler;

impl WorldHandler for QuickSlotHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let mut keys = [0; 8];
        for key in &mut keys {
            *key = r.read_int()?;
        }
        // The editor makes no layout that fails this.
        let Some(slots) = QuickSlots::from_client(keys) else {
            return Ok(HandlerResult::empty());
        };
        // The echoes repeat the layout; only a change is kept.
        if ctx.player.quick_slots.unwrap_or_default() != slots {
            ctx.player.quick_slots = Some(slots);
            ctx.player.mark_dirty();
        }
        Ok(HandlerResult::empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::quick_slots::DEFAULT;
    use game::Player;
    use packet::io::write::PktWrite;

    fn send(player: &mut Player, keys: &[i32]) -> WorldResult {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::QuickSlot as i16).unwrap();
        for key in keys {
            p.write_int(*key).unwrap();
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        QuickSlotHandler.handle(&mut p, &mut ctx)
    }

    fn player() -> Player {
        let player = crate::codec::stats::sample_player();
        assert!(!player.is_dirty());
        player
    }

    #[test]
    fn a_new_layout_is_kept_without_an_answer() {
        let mut player = player();
        let result = send(&mut player, &[2, 3, 4, 5, 16, 17, 18, 19]).unwrap();
        assert!(result.actions.is_empty());
        assert_eq!(
            player.quick_slots,
            Some(QuickSlots([2, 3, 4, 5, 16, 17, 18, 19]))
        );
        assert!(player.is_dirty());
    }

    #[test]
    fn the_echo_of_the_layout_changes_nothing() {
        let mut player = player();
        send(&mut player, &DEFAULT.map(i32::from)).unwrap();
        assert_eq!(player.quick_slots, None);
        assert!(!player.is_dirty());

        player.quick_slots = Some(QuickSlots([2, 3, 4, 5, 16, 17, 18, 19]));
        send(&mut player, &[2, 3, 4, 5, 16, 17, 18, 19]).unwrap();
        assert!(!player.is_dirty());
    }

    #[test]
    fn a_layout_the_editor_cannot_make_is_ignored() {
        let mut player = player();
        for keys in [
            [42, 82, 71, 73, 29, 83, 79, 42],
            [1, 82, 71, 73, 29, 83, 79, 81],
            [84, 82, 71, 73, 29, 83, 79, 81],
            [89, 82, 71, 73, 29, 83, 79, 81],
            [-1, 82, 71, 73, 29, 83, 79, 81],
        ] {
            send(&mut player, &keys).unwrap();
            assert_eq!(player.quick_slots, None, "{keys:?}");
        }
        assert!(!player.is_dirty());
    }

    #[test]
    fn a_short_packet_is_an_error() {
        let mut player = player();
        assert!(send(&mut player, &[2, 3, 4, 5, 16, 17, 18]).is_err());
        assert_eq!(player.quick_slots, None);
    }
}
