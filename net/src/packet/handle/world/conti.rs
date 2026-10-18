use crate::conti;
use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::conti::build_conti_state;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// 0xDC (v83 FUN_0054db7a): the client has entered a map that shows a ship
/// and asks where the trip is. Field id, then the ship's kind.
pub struct ContiStateRequestHandler;

impl WorldHandler for ContiStateRequestHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let field = reader.read_int()?;
        if field != ctx.player.stats.map_id {
            return Ok(HandlerResult::empty());
        }
        match conti::ship_state(field, conti::now()) {
            Some(state) => Ok(HandlerResult::reply(build_conti_state(state)?)),
            None => Ok(HandlerResult::empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn ask(on: i32, field: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ContiStateRequest as i16).unwrap();
        p.write_int(field).unwrap();
        p.write_byte(0).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = on;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        ContiStateRequestHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn a_client_on_a_platform_is_told_the_trips_state() {
        let actions = ask(101000300, 101000300);
        let [WorldAction::Reply(packet)] = actions.as_slice() else {
            panic!("expected a reply: {actions:?}");
        };
        assert_eq!(packet.bytes[..2], [0x95, 0]);
    }

    #[test]
    fn only_the_players_own_map_with_a_ship_is_answered() {
        assert!(ask(101000300, 200000111).is_empty());
        assert!(ask(100000000, 100000000).is_empty());
    }
}
