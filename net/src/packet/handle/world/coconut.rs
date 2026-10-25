use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `COCONUT` (0xD5, report b167): the player's attack hit nut `index`.
/// The delay that follows is the client's and is ignored.
pub struct CoconutHandler;

impl WorldHandler for CoconutHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let index = reader.read_short()? as u16;
        Ok(HandlerResult::empty().with(WorldAction::Field(FieldRequest::CoconutHit { index })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_player;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    #[test]
    fn a_hit_names_its_nut() {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::Coconut as i16).unwrap();
        packet.write_short(321).unwrap();
        packet.write_short(1200).unwrap();
        let mut player = sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = CoconutHandler.handle(&mut packet, &mut ctx).unwrap();
        let [WorldAction::Field(FieldRequest::CoconutHit { index })] = result.actions.as_slice()
        else {
            panic!("{:?}", result.actions);
        };
        assert_eq!(*index, 321);
    }
}
