use crate::handler::{
    HandlerResult, RingRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `RING_ACTION` (reports b052 and b054): a proposal with an engagement ring
/// box, its withdrawal, the partner's answer, and breaking off by dropping a
/// token; and (report b066) sending and opening wedding invitations. The
/// wishlist comes later.
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
            5 => {
                let guest = reader.read_str_with_length()?;
                let marriage_id = reader.read_int()?;
                let Ok(slot) = i16::try_from(reader.read_int()?) else {
                    return Ok(HandlerResult::empty());
                };
                RingRequest::Invite {
                    guest,
                    marriage_id,
                    slot,
                }
            }
            6 => {
                let Ok(slot) = i16::try_from(reader.read_int()?) else {
                    return Ok(HandlerResult::empty());
                };
                let item_id = reader.read_int()?;
                RingRequest::OpenInvitation { slot, item_id }
            }
            other => {
                debug!(mode = other, "Ignoring RING_ACTION");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Ring(request)))
    }
}

/// `WEDDING_TALK` (report b075): the groom or bride closed the priest's
/// line, naming the last step the client was shown.
pub struct WeddingTalkHandler;

impl WorldHandler for WeddingTalkHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let step = reader.read_byte()?;
        Ok(HandlerResult::empty().with(WorldAction::WeddingTalk { step }))
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

    #[test]
    fn reads_an_invitation_sent_and_opened() {
        let invite = [&[5, 3, 0][..], b"Cid", &[7, 0, 0, 0], &[4, 0, 0, 0]].concat();
        assert_eq!(
            read(&invite),
            Some(RingRequest::Invite {
                guest: "Cid".into(),
                marriage_id: 7,
                slot: 4
            })
        );
        let open = [6, 2, 0, 0, 0, 0xaf, 0x83, 0x3d, 0];
        assert_eq!(
            read(&open),
            Some(RingRequest::OpenInvitation {
                slot: 2,
                item_id: 4031407
            })
        );
        // A slot no inventory has.
        assert_eq!(read(&[6, 0, 0, 1, 0, 0xaf, 0x83, 0x3d, 0]), None);
    }
}
