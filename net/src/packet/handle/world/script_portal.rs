use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `CHANGE_MAP_SPECIAL`: the client touched a script portal. A byte, the
/// portal's name, then a short.
pub struct ChangeMapSpecialHandler;

impl WorldHandler for ChangeMapSpecialHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_byte()?;
        let portal = reader.read_str_with_length()?;
        if super::change_map::held_at(ctx.player, &portal) {
            return Ok(HandlerResult::reply(
                crate::packet::build::world::stat::build_stat_update(true, &[])?,
            ));
        }
        Ok(HandlerResult::empty().with(WorldAction::ScriptPortal { portal }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn parses_the_portal_name() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::ChangeMapSpecial as i16)
            .unwrap();
        packet.write_byte(0).unwrap();
        packet.write_str_with_length("tuto01").unwrap();
        packet.write_short(0).unwrap();
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = ChangeMapSpecialHandler
            .handle(&mut packet, &mut ctx)
            .unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::ScriptPortal { portal }] if portal == "tuto01"
        ));
    }
}
