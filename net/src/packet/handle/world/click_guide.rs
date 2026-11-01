//! CLICK_GUIDE (0xA2, report b371): the guide following the player was
//! double-clicked; no payload. A Noblesse's guide is Mimo, whose menu
//! opens. Anyone else's is Lilin, an Aran's on the way to Rien, whose menu
//! opens (Cosmic `ClickGuideHandler`).

use crate::dialogue::NpcInteraction;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::Packet;

pub struct ClickGuideHandler;

const NOBLESSE: i16 = 1000;

impl WorldHandler for ClickGuideHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let guide = if ctx.player.stats.job == NOBLESSE {
            NpcInteraction::Mimo
        } else {
            NpcInteraction::Lilin
        };
        Ok(HandlerResult::empty().with(WorldAction::NpcInteraction(guide)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(job: i16) -> Vec<WorldAction> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::ClickGuide as i16).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = job;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        ClickGuideHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn a_noblesse_hears_from_mimo_and_anyone_else_from_lilin() {
        assert!(matches!(
            handle(NOBLESSE).as_slice(),
            [WorldAction::NpcInteraction(NpcInteraction::Mimo)]
        ));
        assert!(matches!(
            handle(2000).as_slice(),
            [WorldAction::NpcInteraction(NpcInteraction::Lilin)]
        ));
    }
}
