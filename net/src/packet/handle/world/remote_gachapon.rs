//! USE_REMOTE (0x74, 0xa14074, report b196): the Remote Gachapon Ticket's
//! window picked a town. The ticket's item id, then the town's button, 0
//! to 9 (`gachapon::remote_machine`). The client closes the window and
//! waits for nothing; the machine's script answers as its NPC.

use crate::dialogue::NpcInteraction;
use crate::gachapon;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct RemoteGachaponHandler;

impl WorldHandler for RemoteGachaponHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let ticket = r.read_int()?;
        let index = r.read_int()?;
        // Cosmic disconnects for another item or button; nothing happens
        // here.
        let machine = (ticket == gachapon::REMOTE_TICKET)
            .then(|| gachapon::remote_machine(index))
            .flatten();
        Ok(match machine {
            Some(machine) => HandlerResult::empty().with(WorldAction::NpcInteraction(
                NpcInteraction::RemoteGachapon { machine },
            )),
            None => HandlerResult::empty(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(ticket: i32, index: i32) -> Option<i32> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::RemoteGachapon as i16)
            .unwrap();
        packet.write_int(ticket).unwrap();
        packet.write_int(index).unwrap();
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = (RemoteGachaponHandler.handle(&mut packet, &mut ctx))
            .unwrap()
            .actions;
        match actions.as_slice() {
            [] => None,
            [WorldAction::NpcInteraction(NpcInteraction::RemoteGachapon { machine })] => {
                Some(*machine)
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_town_button_plays_its_machine() {
        assert_eq!(handle(gachapon::REMOTE_TICKET, 0), Some(9100100));
        assert_eq!(handle(gachapon::REMOTE_TICKET, 8), Some(9100109));
        assert_eq!(handle(gachapon::REMOTE_TICKET, 9), Some(9100117));
    }

    #[test]
    fn another_item_or_button_does_nothing() {
        assert_eq!(handle(gachapon::TICKET, 0), None);
        assert_eq!(handle(gachapon::REMOTE_TICKET, 10), None);
        assert_eq!(handle(gachapon::REMOTE_TICKET, -1), None);
    }
}
