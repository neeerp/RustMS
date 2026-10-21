use crate::duey::DueyRequest;
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `DUEY_ACTION` (report b028): op 0 opens the window, 2 sends (bag, slot,
/// quantity, mesos, receiver, then for a quick send its message and the
/// coupon's slot, which the server finds itself), 4 claims a parcel, 5
/// deletes one, and 7 closes the window. The runtime answers.
pub struct DueyActionHandler;

impl WorldHandler for DueyActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0 => DueyRequest::Open,
            2 => {
                let kind = reader.read_byte()?;
                let slot = reader.read_short()?;
                let quantity = reader.read_short()?;
                let mesos = reader.read_int()?;
                let to = reader.read_str_with_length()?;
                let quick = match reader.read_byte()? {
                    0 => None,
                    _ => Some(reader.read_str_with_length()?),
                };
                DueyRequest::Send {
                    kind,
                    slot,
                    quantity,
                    mesos,
                    to,
                    quick,
                }
            }
            4 => DueyRequest::Claim {
                id: reader.read_int()?,
            },
            5 => DueyRequest::Delete {
                id: reader.read_int()?,
            },
            7 => DueyRequest::Close,
            other => {
                debug!(op = other, "Ignoring Duey request");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Duey(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<DueyRequest> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::DueyAction as i16).unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = DueyActionHandler.handle(&mut packet, &mut ctx).unwrap();
        match result.actions.into_iter().next() {
            Some(WorldAction::Duey(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    fn send(p: &mut Packet, quick: u8) {
        p.write_byte(2).unwrap();
        p.write_byte(2).unwrap();
        p.write_short(3).unwrap();
        p.write_short(10).unwrap();
        p.write_int(5000).unwrap();
        p.write_str_with_length("Ann").unwrap();
        p.write_byte(quick).unwrap();
    }

    #[test]
    fn reads_every_op() {
        let open = parse(|p| {
            p.write_byte(0).unwrap();
            p.write_int(-1).unwrap();
            p.write_int(2).unwrap();
        });
        assert_eq!(open, Some(DueyRequest::Open));
        assert_eq!(
            parse(|p| send(p, 0)),
            Some(DueyRequest::Send {
                kind: 2,
                slot: 3,
                quantity: 10,
                mesos: 5000,
                to: "Ann".into(),
                quick: None,
            })
        );
        let quick = parse(|p| {
            send(p, 1);
            p.write_str_with_length("thanks").unwrap();
            p.write_int(4).unwrap();
        });
        assert_eq!(
            quick,
            Some(DueyRequest::Send {
                kind: 2,
                slot: 3,
                quantity: 10,
                mesos: 5000,
                to: "Ann".into(),
                quick: Some("thanks".into()),
            })
        );
        let claim = parse(|p| {
            p.write_byte(4).unwrap();
            p.write_int(12).unwrap();
        });
        assert_eq!(claim, Some(DueyRequest::Claim { id: 12 }));
        let delete = parse(|p| {
            p.write_byte(5).unwrap();
            p.write_int(13).unwrap();
        });
        assert_eq!(delete, Some(DueyRequest::Delete { id: 13 }));
        assert_eq!(
            parse(|p| p.write_byte(7).unwrap()),
            Some(DueyRequest::Close)
        );
        assert_eq!(parse(|p| p.write_byte(9).unwrap()), None);
    }
}
