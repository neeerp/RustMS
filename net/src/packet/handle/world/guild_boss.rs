use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::Packet;

/// `GUILD_BOSS_PULLEY` (0xD7, report b380): the player's swing hit the
/// pulley in Ergoth's room. The body is empty.
pub struct GuildBossPulleyHandler;

impl WorldHandler for GuildBossPulleyHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::PullPulley)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_player;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn a_hit_pulls_the_pulley() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::GuildBossPulley as i16)
            .unwrap();
        let mut player = sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = GuildBossPulleyHandler
            .handle(&mut packet, &mut ctx)
            .unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::Field(FieldRequest::PullPulley)]
        ));
    }
}
