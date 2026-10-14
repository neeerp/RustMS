//! The Cash Shop's commodities (`Etc.nx`), loaded once, and the requests a
//! player makes in the shop.

use crate::load::{self, asset_path};
use crate::packet::build::world::cash_shop::{CashShopFailure, WISH_LIST_SLOTS};
use db::cash_shop::{CashItem, CashType};
use game::{InventoryType, Item};
use game_data::commodities::{Commodity, CommodityData};
use std::sync::OnceLock;

pub fn commodities() -> Result<&'static CommodityData, &'static str> {
    static COMMODITIES: OnceLock<Result<CommodityData, String>> = OnceLock::new();
    load::once(&COMMODITIES, || {
        CommodityData::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
}

/// The commodity with serial number `sn`, if the data has it.
pub fn commodity(sn: i32) -> Option<&'static Commodity> {
    commodities().ok()?.get(sn)
}

/// A Cash Shop request the runtime carries out against the account.
#[derive(Debug, PartialEq, Eq)]
pub enum CashShopRequest {
    /// Pay for `commodity` and put it in the Cash Inventory.
    Buy {
        cash_type: CashType,
        commodity: Commodity,
    },
    /// Pay for `commodity` and send it to the character named `to`, whose
    /// gender the runtime still has to check.
    Gift {
        commodity: Commodity,
        to: String,
        message: String,
    },
    /// Replace the wish list with these serial numbers.
    SetWishList(Vec<i32>),
    /// Move the Cash Inventory item with this cash serial to the character,
    /// at `slot` of its bag if that slot is free.
    TakeOut { cash_id: i64, slot: i16 },
    /// Move the character's item with this cash serial, from `kind`, back to
    /// the Cash Inventory.
    PutBack { cash_id: i64, kind: InventoryType },
}

/// The balance a purchase names: 1 NX Credit, 2 Maple Points, 4 NX Prepaid.
pub fn cash_type(value: i32) -> Option<CashType> {
    match value {
        1 => Some(CashType::Credit),
        2 => Some(CashType::MaplePoints),
        4 => Some(CashType::Prepaid),
        _ => None,
    }
}

/// Whether a character of `gender` can buy `commodity`.
pub fn check_purchase(
    commodity: Option<&Commodity>,
    gender: i16,
) -> Result<&Commodity, CashShopFailure> {
    let commodity = check_gift(commodity)?;
    if !commodity.for_gender(gender) {
        return Err(CashShopFailure::Gender);
    }
    Ok(commodity)
}

/// Whether `commodity` is for sale, to anyone; a gift's receiver is checked
/// once it is found.
pub fn check_gift(commodity: Option<&Commodity>) -> Result<&Commodity, CashShopFailure> {
    let commodity = commodity.ok_or(CashShopFailure::Unknown)?;
    if !commodity.on_sale || commodity.price <= 0 {
        return Err(CashShopFailure::SoldOut);
    }
    // Packages (CashPackage.img) hold several items; they aren't sold yet.
    if commodity.item_id / 1_000_000 == 9 {
        return Err(CashShopFailure::Unknown);
    }
    Ok(commodity)
}

/// Whether a birthday code, YYYYMMDD, is a date. Accounts don't record a
/// birthday, so any date confirms a gift.
pub fn birthday_code(code: i32) -> bool {
    let (year, month, day) = (code / 10_000, code / 100 % 100, code % 100);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    (1900..=9999).contains(&year) && (1..=days).contains(&day)
}

/// The inventory item a Cash Inventory item becomes: equipment gets its base
/// stats, and the item keeps its cash serial and expiry. `None` if the item
/// data has no such item, or for pets, whose item encoding isn't written yet.
pub fn inventory_item(cash_item: &CashItem) -> Option<Item> {
    if cash_item.item_id / 10_000 == 500 {
        return None;
    }
    let item = crate::items::create(cash_item.item_id, cash_item.quantity)?;
    Some(Item {
        cash_serial: Some(cash_item.id),
        expiration: cash_item.expires_at,
        ..item
    })
}

/// The Cash Inventory item a character's cash item becomes again. The
/// inventory doesn't keep the commodity, so it is looked up by item.
pub fn cash_item(account_id: i32, cash_id: i64, item: &Item) -> CashItem {
    CashItem {
        id: cash_id,
        account_id,
        item_id: item.item_id,
        sn: commodity_sn(item.item_id, item.quantity),
        quantity: item.quantity,
        gift_from: String::new(),
        expires_at: item.expiration,
        gift_message: None,
    }
}

/// The serial number of a commodity selling `count` of `item_id`, preferring
/// one on sale; 0 if none does.
fn commodity_sn(item_id: i32, count: i16) -> i32 {
    commodities().ok().map_or(0, |data| {
        data.iter()
            .filter(|c| c.item_id == item_id)
            .max_by_key(|c| (c.count == count, c.on_sale, -c.sn))
            .map_or(0, |c| c.sn)
    })
}

/// The wish list the client sent, keeping the commodities on sale, each once.
pub fn wish_list(sns: &[i32], on_sale: impl Fn(i32) -> bool) -> Vec<i32> {
    let mut list = Vec::new();
    for &sn in sns {
        if sn != 0 && !list.contains(&sn) && on_sale(sn) && list.len() < WISH_LIST_SLOTS {
            list.push(sn);
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket() -> Commodity {
        Commodity {
            sn: 50200005,
            item_id: 5220000,
            count: 1,
            price: 1000,
            period: 90,
            gender: 2,
            on_sale: true,
        }
    }

    #[test]
    fn pets_stay_in_the_cash_inventory() {
        let pet = CashItem {
            id: 7,
            account_id: 3,
            item_id: 5000000,
            sn: 60000000,
            quantity: 1,
            gift_from: String::new(),
            expires_at: None,
            gift_message: None,
        };
        assert_eq!(inventory_item(&pet), None);
    }

    #[test]
    fn purchases_need_an_item_on_sale_for_the_character() {
        let ticket = ticket();
        assert_eq!(check_purchase(Some(&ticket), 1), Ok(&ticket));
        assert_eq!(check_purchase(None, 0), Err(CashShopFailure::Unknown));

        let retired = Commodity {
            on_sale: false,
            ..ticket.clone()
        };
        assert_eq!(
            check_purchase(Some(&retired), 0),
            Err(CashShopFailure::SoldOut)
        );

        let male = Commodity {
            gender: 0,
            ..ticket.clone()
        };
        assert_eq!(check_purchase(Some(&male), 0), Ok(&male));
        assert_eq!(check_purchase(Some(&male), 1), Err(CashShopFailure::Gender));

        let package = Commodity {
            item_id: 9101000,
            ..ticket
        };
        assert_eq!(
            check_purchase(Some(&package), 0),
            Err(CashShopFailure::Unknown)
        );
    }

    #[test]
    fn gifts_ignore_the_senders_gender() {
        let male = Commodity {
            gender: 0,
            ..ticket()
        };
        assert_eq!(check_gift(Some(&male)), Ok(&male));
        let retired = Commodity {
            on_sale: false,
            ..ticket()
        };
        assert_eq!(check_gift(Some(&retired)), Err(CashShopFailure::SoldOut));
    }

    #[test]
    fn birthday_codes_are_dates() {
        assert!(birthday_code(19830113));
        assert!(birthday_code(20000229));
        assert!(!birthday_code(19000229));
        assert!(!birthday_code(19831301));
        assert!(!birthday_code(19830100));
        assert!(!birthday_code(0));
        assert!(!birthday_code(-19830113));
    }

    #[test]
    fn cash_types_follow_the_client() {
        assert_eq!(cash_type(1), Some(CashType::Credit));
        assert_eq!(cash_type(2), Some(CashType::MaplePoints));
        assert_eq!(cash_type(4), Some(CashType::Prepaid));
        assert_eq!(cash_type(3), None);
    }

    #[test]
    fn wish_lists_drop_empty_unknown_and_repeated_slots() {
        let sold = |sn: i32| sn != 99;
        assert_eq!(
            wish_list(&[0, 5, 99, 5, 0, 7, 0, 0, 0, 0], sold),
            vec![5, 7]
        );
        assert_eq!(wish_list(&(1..=12).collect::<Vec<_>>(), sold).len(), 10);
    }
}
