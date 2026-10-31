//! CLICK_GUIDE (0xA2, report b371): the guide following the player was
//! double-clicked; no payload. A Noblesse's guide is Mimo, whose menu
//! opens. Everyone else's would be Lilin's Aran tutorial, which isn't
//! played, so nothing happens for them.

use crate::dialogue::NpcInteraction;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::Packet;

pub struct ClickGuideHandler;

const NOBLESSE: i16 = 1000;

impl WorldHandler for ClickGuideHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        Ok(if ctx.player.stats.job == NOBLESSE {
            HandlerResult::empty().with(WorldAction::NpcInteraction(NpcInteraction::Mimo))
        } else {
            HandlerResult::empty()
        })
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
    fn only_a_noblesse_hears_from_mimo() {
        assert!(matches!(
            handle(NOBLESSE).as_slice(),
            [WorldAction::NpcInteraction(NpcInteraction::Mimo)]
        ));
        assert!(handle(2000).is_empty());
        assert!(handle(0).is_empty());
    }
}
