use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::storage::StorageRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `TRUNK_REQUEST`: mode 4 takes out (tab, index in the tab), 5 stores (bag
/// slot, item, quantity), 6 sorts, 7 moves mesos (out if positive), and 8
/// closes. The runtime holds the open storage and answers.
pub struct TrunkRequestHandler;

impl WorldHandler for TrunkRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            4 => StorageRequest::TakeOut {
                tab: reader.read_byte()?,
                index: reader.read_byte()?,
            },
            5 => StorageRequest::Store {
                slot: reader.read_short()?,
                item_id: reader.read_int()?,
                quantity: reader.read_short()?,
            },
            6 => StorageRequest::Sort,
            7 => StorageRequest::Meso(reader.read_int()?),
            8 => StorageRequest::Close,
            other => {
                debug!(mode = other, "Ignoring storage request");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Storage(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<StorageRequest> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::TrunkRequest as i16).unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = TrunkRequestHandler.handle(&mut packet, &mut ctx).unwrap();
        match result.actions.into_iter().next() {
            Some(WorldAction::Storage(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_every_mode() {
        let take = parse(|p| {
            p.write_byte(4).unwrap();
            p.write_byte(2).unwrap();
            p.write_byte(1).unwrap();
        });
        assert_eq!(take, Some(StorageRequest::TakeOut { tab: 2, index: 1 }));
        let store = parse(|p| {
            p.write_byte(5).unwrap();
            p.write_short(3).unwrap();
            p.write_int(2000000).unwrap();
            p.write_short(10).unwrap();
        });
        assert_eq!(
            store,
            Some(StorageRequest::Store {
                slot: 3,
                item_id: 2000000,
                quantity: 10
            })
        );
        assert_eq!(
            parse(|p| p.write_byte(6).unwrap()),
            Some(StorageRequest::Sort)
        );
        let meso = parse(|p| {
            p.write_byte(7).unwrap();
            p.write_int(-500).unwrap();
        });
        assert_eq!(meso, Some(StorageRequest::Meso(-500)));
        assert_eq!(
            parse(|p| p.write_byte(8).unwrap()),
            Some(StorageRequest::Close)
        );
        assert_eq!(parse(|p| p.write_byte(9).unwrap()), None);
    }
}
