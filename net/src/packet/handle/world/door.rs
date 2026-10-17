use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::map::{build_blocked_map, BlockedMap};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `USE_DOOR`: the player pressed up in a Mystic Door: `owner`'s, then
/// whether they stand in its town (0x5375ed) or its field (0x7bd594). The
/// directory, which knows the doors, decides where it leads.
pub struct UseDoorHandler;

impl WorldHandler for UseDoorHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let owner = r.read_int()?;
        let from_town = r.read_byte()? != 0;
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::reply(build_blocked_map(
                BlockedMap::PortalClosed,
            )?));
        }
        Ok(HandlerResult::empty().with(WorldAction::UseDoor { owner, from_town }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn use_door(hp: i16) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::UseDoor as i16).unwrap();
        packet.write_int(7).unwrap();
        packet.write_byte(1).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = hp;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        UseDoorHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn a_door_is_asked_where_it_leads() {
        assert!(matches!(
            use_door(50).as_slice(),
            [WorldAction::UseDoor {
                owner: 7,
                from_town: true
            }]
        ));
        assert!(matches!(use_door(0).as_slice(), [WorldAction::Reply(_)]));
    }
}
