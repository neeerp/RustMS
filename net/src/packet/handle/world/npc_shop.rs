use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::shop::ShopRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `NPC_SHOP_ACTION`: mode 0 buys (list row, item, quantity), 1 sells (bag
/// slot, item, quantity), 2 recharges (USE slot), and 3 leaves. The runtime
/// holds the open shop and answers.
pub struct NpcShopActionHandler;

impl WorldHandler for NpcShopActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0 => ShopRequest::Buy {
                index: reader.read_short()?,
                item_id: reader.read_int()?,
                quantity: reader.read_short()?,
            },
            1 => ShopRequest::Sell {
                slot: reader.read_short()?,
                item_id: reader.read_int()?,
                quantity: reader.read_short()?,
            },
            2 => ShopRequest::Recharge {
                slot: reader.read_short()?,
            },
            3 => ShopRequest::Leave,
            other => {
                debug!(mode = other, "Ignoring shop action");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Shop(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse(build: impl FnOnce(&mut Packet)) -> Option<ShopRequest> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::NpcShopAction as i16)
            .unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = NpcShopActionHandler.handle(&mut packet, &mut ctx).unwrap();
        match result.actions.into_iter().next() {
            Some(WorldAction::Shop(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_every_mode() {
        assert_eq!(
            parse(|p| {
                p.write_byte(0).unwrap();
                p.write_short(1).unwrap();
                p.write_int(2000000).unwrap();
                p.write_short(5).unwrap();
            }),
            Some(ShopRequest::Buy {
                index: 1,
                item_id: 2000000,
                quantity: 5
            })
        );
        assert_eq!(
            parse(|p| {
                p.write_byte(1).unwrap();
                p.write_short(3).unwrap();
                p.write_int(1302000).unwrap();
                p.write_short(1).unwrap();
            }),
            Some(ShopRequest::Sell {
                slot: 3,
                item_id: 1302000,
                quantity: 1
            })
        );
        assert_eq!(
            parse(|p| {
                p.write_byte(2).unwrap();
                p.write_short(7).unwrap();
            }),
            Some(ShopRequest::Recharge { slot: 7 })
        );
        assert_eq!(
            parse(|p| {
                p.write_byte(3).unwrap();
            }),
            Some(ShopRequest::Leave)
        );
        assert_eq!(
            parse(|p| {
                p.write_byte(9).unwrap();
            }),
            None
        );
    }
}
