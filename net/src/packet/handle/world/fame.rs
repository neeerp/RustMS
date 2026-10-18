use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::fame::{build_fame_response, FameResponse};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// The lowest level that may give fame.
pub const MIN_FAME_LEVEL: u8 = 15;

/// `GIVE_FAME` (0x5F, v83 FUN_00a23eb5): character id, 1 to raise their fame
/// or 0 to drop it. The client sends it from a character's info window, for
/// someone on its map, at most once every five minutes.
pub struct GiveFameHandler;

impl WorldHandler for GiveFameHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let target = reader.read_int()?;
        let up = reader.read_byte()? != 0;
        if target == ctx.client_id {
            // The window has no fame buttons for oneself.
            return Ok(HandlerResult::empty());
        }
        if ctx.player.stats.level < MIN_FAME_LEVEL {
            return Ok(HandlerResult::reply(build_fame_response(
                &FameResponse::UnderLevel,
            )?));
        }
        Ok(HandlerResult::empty().with(WorldAction::GiveFame { target, up }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn give(level: u8, target: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::GiveFame as i16).unwrap();
        p.write_int(target).unwrap();
        p.write_byte(1).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        GiveFameHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    #[test]
    fn a_player_of_level_15_may_raise_someones_fame() {
        assert!(matches!(
            give(15, 62).as_slice(),
            [WorldAction::GiveFame {
                target: 62,
                up: true
            }]
        ));
    }

    #[test]
    fn a_player_under_level_15_is_told_they_cant() {
        let actions = give(14, 62);
        let [WorldAction::Reply(packet)] = actions.as_slice() else {
            panic!("expected a reply: {actions:?}");
        };
        assert_eq!(packet.bytes, [0x26, 0, 2]);
    }

    #[test]
    fn nobody_fames_themselves() {
        assert!(give(30, 1).is_empty());
    }
}
