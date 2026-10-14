//! The Cash Shop's commodities (`Etc.nx`), loaded once, and the requests a
//! player makes in the shop.

use crate::load::{self, asset_path};
use crate::packet::build::world::cash_shop::{CashShopFailure, WISH_LIST_SLOTS};
use db::cash_shop::CashType;
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
    /// Replace the wish list with these serial numbers.
    SetWishList(Vec<i32>),
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
    let commodity = commodity.ok_or(CashShopFailure::Unknown)?;
    if !commodity.on_sale || commodity.price <= 0 {
        return Err(CashShopFailure::SoldOut);
    }
    if !commodity.for_gender(gender) {
        return Err(CashShopFailure::Gender);
    }
    // Packages (CashPackage.img) hold several items; they aren't sold yet.
    if commodity.item_id / 1_000_000 == 9 {
        return Err(CashShopFailure::Unknown);
    }
    Ok(commodity)
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
