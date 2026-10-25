use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `SNOWBALL` (0xD3, report b168): the player's attack hit `target` (ball
/// 0, ball 1, snowman 0, snowman 1), showing after `delay` ms. The damage
/// is the client's roll; the field rolls its own.
pub struct SnowballHandler;

impl WorldHandler for SnowballHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let target = reader.read_byte()?;
        reader.read_short()?;
        let delay = reader.read_short()? as u16;
        Ok(
            HandlerResult::empty().with(WorldAction::Field(FieldRequest::SnowballHit {
                target,
                delay,
            })),
        )
    }
}

/// `LEFT_KNOCKBACK` (0xD4): the player is inside a ball.
pub struct LeftKnockbackHandler;

impl WorldHandler for LeftKnockbackHandler {
    fn handle(&self, _packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::IntoSnowball)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_player;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn a_hit_names_its_target_and_delay() {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::Snowball as i16).unwrap();
        packet.write_byte(3).unwrap();
        packet.write_short(45).unwrap();
        packet.write_short(720).unwrap();
        let mut player = sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = SnowballHandler.handle(&mut packet, &mut ctx).unwrap();
        let [WorldAction::Field(FieldRequest::SnowballHit { target, delay })] =
            result.actions.as_slice()
        else {
            panic!("{:?}", result.actions);
        };
        assert_eq!((*target, *delay), (3, 720));
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::LeftKnockback as i16)
            .unwrap();
        let result = LeftKnockbackHandler.handle(&mut packet, &mut ctx).unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::Field(FieldRequest::IntoSnowball)]
        ));
    }
}
