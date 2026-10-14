use crate::cash_shop::{self, CashShopRequest};
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build;
use crate::packet::build::world::cash_shop::{CashShopFailure, WISH_LIST_SLOTS};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// 0x28, `ENTER_CASHSHOP`: the SHOP button or the Cash Shop key. The client
/// leaves again with an empty `CHANGE_MAP`.
pub struct EnterCashShopHandler;

impl WorldHandler for EnterCashShopHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.stats.hp <= 0 {
            let enable_actions = build::world::stat::build_stat_update(true, &[])?;
            return Ok(HandlerResult::reply(enable_actions));
        }
        Ok(HandlerResult::empty().with(WorldAction::EnterCashShop))
    }
}

/// 0xE5, `CASHSHOP_OPERATION`: an action byte, then its fields. Buying (0x03)
/// sends a byte, the balance to pay with, and the commodity's serial number;
/// the wish list (0x05) sends its ten serial numbers, 0 for an empty slot.
pub struct CashShopOperationHandler;

const BUY: u8 = 0x03;
const SET_WISH_LIST: u8 = 0x05;

impl WorldHandler for CashShopOperationHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            BUY => {
                reader.read_byte()?;
                let cash_type = cash_shop::cash_type(reader.read_int()?);
                let sn = reader.read_int()?;
                let commodity = cash_shop::check_purchase(
                    cash_shop::commodity(sn),
                    ctx.player.stats.gender.into(),
                );
                match (cash_type, commodity) {
                    (Some(cash_type), Ok(commodity)) => CashShopRequest::Buy {
                        cash_type,
                        commodity: commodity.clone(),
                    },
                    (None, _) => return fail(CashShopFailure::Unknown),
                    (_, Err(failure)) => return fail(failure),
                }
            }
            SET_WISH_LIST => {
                let mut sns = [0; WISH_LIST_SLOTS];
                for sn in &mut sns {
                    *sn = reader.read_int()?;
                }
                CashShopRequest::SetWishList(cash_shop::wish_list(&sns, |sn| {
                    cash_shop::commodity(sn).is_some_and(|c| c.on_sale)
                }))
            }
            other => {
                // Gifts, packages, coupons, and moving items out of the Cash
                // Inventory aren't supported yet.
                debug!(action = other, "Ignoring Cash Shop operation");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::CashShop(request)))
    }
}

fn fail(failure: CashShopFailure) -> WorldResult {
    Ok(HandlerResult::reply(
        build::world::cash_shop::build_cash_shop_failure(failure)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(build: impl FnOnce(&mut Packet)) -> HandlerResult<WorldAction> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::CashShopOperation as i16)
            .unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        CashShopOperationHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
    }

    #[test]
    fn an_unknown_balance_fails_the_purchase() {
        let result = handle(|p| {
            p.write_byte(BUY).unwrap();
            p.write_byte(0).unwrap();
            p.write_int(3).unwrap();
            p.write_int(50200005).unwrap();
        });
        let [WorldAction::Reply(reply)] = result.actions.as_slice() else {
            panic!("expected a failure reply: {:?}", result.actions);
        };
        assert_eq!(reply.bytes[2..], [0x5C, CashShopFailure::Unknown as u8]);
    }

    #[test]
    fn an_empty_wish_list_is_saved() {
        let result = handle(|p| {
            p.write_byte(SET_WISH_LIST).unwrap();
            for _ in 0..WISH_LIST_SLOTS {
                p.write_int(0).unwrap();
            }
        });
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::CashShop(CashShopRequest::SetWishList(sns))] if sns.is_empty()
        ));
    }
}
