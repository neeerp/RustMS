use crate::handler::{
    HandlerResult, RingRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `RING_ACTION` (reports b052 and b054): a proposal with an engagement ring
/// box, its withdrawal, the partner's answer, and breaking off by dropping a
/// token. Invitations and the wishlist come later.
pub struct RingActionHandler;

impl WorldHandler for RingActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0 => {
                let name = reader.read_str_with_length()?;
                let item_id = reader.read_int()?;
                RingRequest::Propose { name, item_id }
            }
            1 => RingRequest::Cancel,
            2 => {
                let accept = reader.read_byte()? != 0;
                reader.read_str_with_length()?;
                let proposer = reader.read_int()?;
                RingRequest::Answer { proposer, accept }
            }
            3 => RingRequest::Break {
                item_id: reader.read_int()?,
            },
            other => {
                debug!(mode = other, "Ignoring RING_ACTION");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Ring(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn read(bytes: &[u8]) -> Option<RingRequest> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::RingAction as i16).unwrap();
        packet.write_bytes(bytes).unwrap();
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        match RingActionHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
            .pop()
        {
            Some(WorldAction::Ring(request)) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_a_proposal_and_its_answer() {
        let propose = [&[0, 3, 0][..], b"Ann", &[0x40, 0x2e, 0x22, 0]].concat();
        assert_eq!(
            read(&propose),
            Some(RingRequest::Propose {
                name: "Ann".into(),
                item_id: 2240064
            })
        );
        assert_eq!(read(&[1]), Some(RingRequest::Cancel));
        let answer = [&[2, 1, 3, 0][..], b"Bob", &[9, 0, 0, 0]].concat();
        assert_eq!(
            read(&answer),
            Some(RingRequest::Answer {
                proposer: 9,
                accept: true
            })
        );
        assert_eq!(
            read(&[3, 0x7d, 0x83, 0x3d, 0]),
            Some(RingRequest::Break { item_id: 4031357 })
        );
        assert_eq!(read(&[9, 0, 0]), None);
    }
}
