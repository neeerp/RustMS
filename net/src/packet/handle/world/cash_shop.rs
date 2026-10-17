use crate::cash_shop::{self, CashShopRequest, Expansion};
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build;
use crate::packet::build::world::cash_shop::{CashShopFailure, GIFT_MESSAGE_LEN, WISH_LIST_SLOTS};
use crate::packet::build::world::map::BlockedServer;
use game::InventoryType;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// 0x28, `ENTER_CASHSHOP`: the SHOP button or the Cash Shop key. The client
/// leaves again with an empty `CHANGE_MAP`.
pub struct EnterCashShopHandler;

impl WorldHandler for EnterCashShopHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        if ctx.player.stats.hp <= 0 {
            return Ok(HandlerResult::reply(
                build::world::map::build_blocked_server(BlockedServer::CashShop)?,
            ));
        }
        Ok(HandlerResult::empty().with(WorldAction::EnterCashShop))
    }
}

/// 0xE5, `CASHSHOP_OPERATION`: an action byte, then its fields. Buying (0x03,
/// or 0x1E for a package) sends a byte, the balance to pay with, and the
/// commodity's serial number;
/// a gift (0x04) sends the birthday code, the serial number, and the
/// receiver's name and message; the wish list (0x05) sends its ten serial
/// numbers, 0 for an empty slot.
/// Taking an item out of the Cash Inventory (0x0D) sends its cash serial, its
/// bag and the slot the client chose; putting one back (0x0E) sends its cash
/// serial and bag. Expanding a bag (0x06) or the storage (0x07) sends a byte,
/// the balance, and either 0 and (for a bag) the bag, from the slot buttons,
/// or 1 and an expansion commodity's serial number.
pub struct CashShopOperationHandler;

const BUY: u8 = 0x03;
const GIFT: u8 = 0x04;
const SET_WISH_LIST: u8 = 0x05;
const TAKE_OUT: u8 = 0x0D;
const PUT_BACK: u8 = 0x0E;
const EXPAND_BAG: u8 = 0x06;
const EXPAND_STORAGE: u8 = 0x07;
const BUY_PACKAGE: u8 = 0x1E;

impl WorldHandler for CashShopOperationHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            BUY | BUY_PACKAGE => {
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
            GIFT => {
                let birthday = reader.read_int()?;
                let sn = reader.read_int()?;
                let to = reader.read_str_with_length()?;
                let message = reader.read_str_with_length()?;
                let commodity = match cash_shop::check_gift(cash_shop::commodity(sn)) {
                    Ok(commodity) => commodity,
                    Err(failure) => return fail(failure),
                };
                if message.is_empty() || message.len() > GIFT_MESSAGE_LEN {
                    return fail(CashShopFailure::Unknown);
                }
                if !cash_shop::birthday_code(birthday) {
                    return fail(CashShopFailure::BirthdayCode);
                }
                CashShopRequest::Gift {
                    commodity: commodity.clone(),
                    to,
                    message,
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
            TAKE_OUT => {
                let cash_id = reader.read_long()?;
                reader.read_byte()?; // the item's bag
                let slot = reader.read_short()?;
                CashShopRequest::TakeOut { cash_id, slot }
            }
            PUT_BACK => {
                let cash_id = reader.read_long()?;
                let kind = InventoryType::from_i16(reader.read_byte()?.into())
                    .filter(|kind| InventoryType::BAGS.contains(kind));
                match kind {
                    Some(kind) => CashShopRequest::PutBack { cash_id, kind },
                    None => return fail(CashShopFailure::Unknown),
                }
            }
            action @ (EXPAND_BAG | EXPAND_STORAGE) => {
                reader.read_byte()?;
                let cash_type = cash_shop::cash_type(reader.read_int()?);
                let expansion = match reader.read_byte()? {
                    0 if action == EXPAND_STORAGE => Some((
                        Expansion::Storage,
                        cash_shop::EXPANSION_SLOTS,
                        cash_shop::EXPANSION_PRICE,
                    )),
                    0 => {
                        let kind = InventoryType::from_i16(reader.read_byte()?.into())
                            .filter(|&kind| kind != InventoryType::Cash);
                        kind.map(|kind| {
                            (
                                Expansion::Bag(kind),
                                cash_shop::EXPANSION_SLOTS,
                                cash_shop::EXPANSION_PRICE,
                            )
                        })
                    }
                    _ => {
                        let commodity = cash_shop::commodity(reader.read_int()?)
                            .filter(|c| c.on_sale && c.price > 0);
                        commodity.and_then(|c| {
                            let expansion = cash_shop::expansion(c)?;
                            let storage = expansion == Expansion::Storage;
                            (storage == (action == EXPAND_STORAGE)).then_some((
                                expansion,
                                cash_shop::COMMODITY_EXPANSION_SLOTS,
                                c.price,
                            ))
                        })
                    }
                };
                match (cash_type, expansion) {
                    (Some(cash_type), Some((expansion, slots, price))) => CashShopRequest::Expand {
                        cash_type,
                        expansion,
                        slots,
                        price,
                    },
                    _ => return fail(CashShopFailure::Unknown),
                }
            }
            other => {
                // Coupons and the rest aren't supported yet.
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
    use db::cash_shop::CashType;
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
    fn the_dead_are_kept_out_of_the_cash_shop() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.hp = 0;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::EnterCashShop as i16)
            .unwrap();
        let result = EnterCashShopHandler.handle(&mut packet, &mut ctx).unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::Reply(p)] if p.bytes == [0x84, 0, 2]
        ));
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
    fn items_move_between_the_cash_inventory_and_the_bags() {
        let result = handle(|p| {
            p.write_byte(TAKE_OUT).unwrap();
            p.write_long(42).unwrap();
            p.write_byte(5).unwrap();
            p.write_short(3).unwrap();
        });
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::CashShop(CashShopRequest::TakeOut {
                cash_id: 42,
                slot: 3
            })]
        ));

        let result = handle(|p| {
            p.write_byte(PUT_BACK).unwrap();
            p.write_long(42).unwrap();
            p.write_byte(1).unwrap();
        });
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::CashShop(CashShopRequest::PutBack {
                cash_id: 42,
                kind: InventoryType::Equip
            })]
        ));

        // Equipped items (-1) can't go back.
        let result = handle(|p| {
            p.write_byte(PUT_BACK).unwrap();
            p.write_long(42).unwrap();
            p.write_byte(0xFF).unwrap();
        });
        assert!(matches!(result.actions.as_slice(), [WorldAction::Reply(_)]));
    }

    #[test]
    fn gifts_of_unknown_commodities_fail() {
        let gift = |birthday: i32, message: &str| {
            handle(|p| {
                p.write_byte(GIFT).unwrap();
                p.write_int(birthday).unwrap();
                p.write_int(0).unwrap();
                p.write_str_with_length("Bob").unwrap();
                p.write_str_with_length(message).unwrap();
            })
        };
        // Serial number 0 is no commodity, which is checked first.
        for result in [gift(19830113, "hi"), gift(1, "")] {
            let [WorldAction::Reply(reply)] = result.actions.as_slice() else {
                panic!("expected a failure reply: {:?}", result.actions);
            };
            assert_eq!(reply.bytes[2..], [0x5C, CashShopFailure::Unknown as u8]);
        }
    }

    #[test]
    fn slot_buttons_add_four_slots_for_4000() {
        let expand = |action: u8, mode: u8, rest: &[u8]| {
            handle(|p| {
                p.write_byte(action).unwrap();
                p.write_byte(0).unwrap();
                p.write_int(1).unwrap();
                p.write_byte(mode).unwrap();
                p.write_bytes(rest).unwrap();
            })
        };
        assert!(matches!(
            expand(EXPAND_BAG, 0, &[3]).actions.as_slice(),
            [WorldAction::CashShop(CashShopRequest::Expand {
                cash_type: CashType::Credit,
                expansion: Expansion::Bag(InventoryType::Setup),
                slots: 4,
                price: 4000,
            })]
        ));
        assert!(matches!(
            expand(EXPAND_STORAGE, 0, &[]).actions.as_slice(),
            [WorldAction::CashShop(CashShopRequest::Expand {
                expansion: Expansion::Storage,
                slots: 4,
                price: 4000,
                ..
            })]
        ));
        // The cash bag always has 96 slots, and an unknown commodity fails.
        for result in [
            expand(EXPAND_BAG, 0, &[5]),
            expand(EXPAND_BAG, 1, &0i32.to_le_bytes()),
        ] {
            assert!(matches!(result.actions.as_slice(), [WorldAction::Reply(_)]));
        }
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
