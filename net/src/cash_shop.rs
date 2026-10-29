//! The Cash Shop's commodities (`Etc.nx`), loaded once, and the requests a
//! player makes in the shop.

use crate::load::{self, asset_path};
use crate::packet::build::world::cash_shop::{CashShopFailure, WISH_LIST_SLOTS};
use db::cash_shop::{CashItem, CashType};
use game::{InventoryChange, InventoryType, Item, Player};
use game_data::commodities::{Commodity, CommodityData};
use std::sync::OnceLock;

pub fn commodities() -> Result<&'static CommodityData, &'static str> {
    static COMMODITIES: OnceLock<Result<CommodityData, String>> = OnceLock::new();
    load::once(&COMMODITIES, || {
        CommodityData::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
}

/// The commodity with serial number `sn`, if the data has it or it is the
/// second pendant slot's.
pub fn commodity(sn: i32) -> Option<&'static Commodity> {
    let found = commodities().ok()?.get(sn);
    found.or((sn == PENDANT_SLOT.sn).then_some(&PENDANT_SLOT))
}

/// The second pendant slot for 30 days (5550000), which no `Commodity.img`
/// sells: the shop adds it to the client's list on entry (report b179), in
/// Etc > Game beside the Extra Character Slot Coupon. Its price is a pick.
pub const PENDANT_SLOT: Commodity = Commodity {
    sn: 50_200_172,
    item_id: 5_550_000,
    count: 1,
    price: 2_000,
    period: PENDANT_SLOT_DAYS as i32,
    gender: 2,
    on_sale: true,
};

/// How long one 5550000 keeps the second pendant slot open: its `addDay` in
/// `Item.nx`, which v83 names in its question (report b179).
pub const PENDANT_SLOT_DAYS: u16 = 30;

/// The most slots a bag can have.
pub const MAX_BAG_SLOTS: u8 = 96;
/// The most slots an account's storage can have.
pub const MAX_STORAGE_SLOTS: i16 = 48;
/// The most character slots an account can have: v83's shop refuses a
/// sixteenth (0x1ed) and its character list holds 15.
pub const MAX_CHARACTER_SLOTS: i16 = 15;
/// What the slot buttons charge, and how many slots they add.
pub const EXPANSION_PRICE: i32 = 4_000;
pub const EXPANSION_SLOTS: u8 = 4;
/// How many slots an expansion commodity (`Special/0911.img`) adds.
pub const COMMODITY_EXPANSION_SLOTS: u8 = 8;

/// What a slot expansion grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expansion {
    /// One of the character's bags, not the cash bag.
    Bag(InventoryType),
    /// The account's storage.
    Storage,
}

/// A Cash Shop request the runtime carries out against the account.
#[derive(Debug, PartialEq, Eq)]
pub enum CashShopRequest {
    /// Pay for `commodity` and put it, or each item of a package, in the
    /// Cash Inventory.
    Buy {
        cash_type: CashType,
        commodity: Commodity,
    },
    /// Send `commodity` to the character named `to`, whose gender the
    /// runtime still has to check. v83 asks twice (report b331): the first
    /// ask is answered with the receiver's character (GIFT_INFO_RESULT), for
    /// the client's own checks and question, and the same ask again, or a
    /// package's `confirmed` one, pays for the gift.
    Gift {
        birthday: i32,
        commodity: Commodity,
        to: String,
        message: String,
        confirmed: bool,
    },
    /// Pay for the crush or friendship ring `commodity`: one for the
    /// Cash Inventory and its partner for the character named `to`, whose
    /// gender the runtime still has to check.
    BuyRing {
        cash_type: CashType,
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
    /// Pay `price` and add `slots` slots to a bag or the storage.
    Expand {
        cash_type: CashType,
        expansion: Expansion,
        slots: u8,
        price: i32,
    },
    /// Send the account's balances again: the Check button, and v83 on every
    /// activation of the game window.
    Balance,
    /// Redeem the coupon with this code, as typed.
    Coupon(String),
    /// Whether the character may ask for a new name (op 0x10): CS 0x149.
    NameChangeRules,
    /// Whether the name typed in the Name Change dialog is free (op 0x15 in
    /// the shop): CS 0x148.
    CheckNewName(String),
    /// Pay for the Extra Character Slot Coupon `commodity` and add a slot.
    BuyCharacterSlot {
        cash_type: CashType,
        commodity: Commodity,
    },
    /// Pay for `commodity` and keep the second pendant slot open its days
    /// longer.
    BuyPendantSlot {
        cash_type: CashType,
        commodity: Commodity,
    },
    /// Pay for Name Change `commodity` from the NX Prepaid and ask for
    /// `new_name`, already checked against the name rules.
    RequestNameChange {
        commodity: Commodity,
        new_name: String,
    },
    /// Pay mesos for the quest item `commodity`, which goes straight to the
    /// character's bags.
    BuyWithMesos(Commodity),
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

/// Whether `item_id` is a crush or friendship ring, which is bought as a
/// pair with its own request (report b061).
pub fn pair_ring(item_id: i32) -> bool {
    matches!(
        game::ring::kind(item_id),
        Some(game::ring::RingKind::Crush | game::ring::RingKind::Friendship)
    )
}

/// Name Change, whose receipt is a request for a new name (report c060).
pub const NAME_CHANGE: i32 = 5_400_000;

/// The Extra Character Slot Coupon: buying it adds a character slot to the
/// account at once (report c085).
pub const CHARACTER_SLOT: i32 = 5_430_000;

/// Whether `item_id` is bought only with its own request: the pair rings,
/// Name Change, the Extra Character Slot Coupon and the pendant slot. None
/// can be a gift.
pub fn own_request(item_id: i32) -> bool {
    pair_ring(item_id) || [NAME_CHANGE, CHARACTER_SLOT, PENDANT_SLOT.item_id].contains(&item_id)
}

/// The lowest level that may ask for a new name (the rules sheet, Text02).
pub const NAME_CHANGE_LEVEL: u8 = 10;

/// Miu Miu the Traveling Merchant: each use opens Mu Mu's shop.
pub const MIU_MIU: i32 = 5_450_000;

/// Whether a character of `gender` can buy `commodity`.
pub fn check_purchase(
    commodity: Option<&Commodity>,
    gender: i16,
) -> Result<&Commodity, CashShopFailure> {
    let commodity = check_sale(commodity)?;
    if !commodity.for_gender(gender) {
        return Err(CashShopFailure::Gender);
    }
    Ok(commodity)
}

/// Whether `commodity` can be sent as a gift, to anyone; a gift's receiver
/// is checked once it is found. A package goes as its items (report b331).
pub fn check_gift(commodity: Option<&Commodity>) -> Result<&Commodity, CashShopFailure> {
    check_sale(commodity)
}

/// The classes (`item_id / 10_000`) of cash items on sale that do nothing
/// here yet: Character Transfer (540, all but Name Change), Maple Life
/// (543, all but the Extra Character Slot Coupon), and Remote Gachapon
/// (545, all but Miu Miu and the Remote Gachapon Ticket).
/// The shop calls them sold out rather than take NX for nothing (report
/// c046). Take a class out once its items work.
const INERT_CLASSES: [i32; 3] = [540, 543, 545];

fn inert(item_id: i32) -> bool {
    ![
        NAME_CHANGE,
        MIU_MIU,
        CHARACTER_SLOT,
        crate::gachapon::REMOTE_TICKET,
    ]
    .contains(&item_id)
        && INERT_CLASSES.contains(&(item_id / 10_000))
}

/// Whether `commodity` is for sale as an item or a package. The other
/// `9xxxxxx` items, the slot expansions, are bought with their own request,
/// as are the commodities sold for mesos.
fn check_sale(commodity: Option<&Commodity>) -> Result<&Commodity, CashShopFailure> {
    let commodity = commodity.ok_or(CashShopFailure::Unknown)?;
    if for_mesos(commodity.sn) {
        return Err(CashShopFailure::Unknown);
    }
    if !commodity.on_sale || commodity.price <= 0 {
        return Err(CashShopFailure::SoldOut);
    }
    let contents = package(commodity).unwrap_or_else(|| vec![commodity]);
    if contents.iter().any(|item| inert(item.item_id)) {
        return Err(CashShopFailure::SoldOut);
    }
    if commodity.item_id / 1_000_000 == 9 && package(commodity).is_none() {
        return Err(CashShopFailure::Unknown);
    }
    Ok(commodity)
}

/// Whether the commodity `sn` costs mesos rather than NX: v83 shows the
/// ones on sale (quest items, for 1 meso) among the others, and buys them
/// with their own request, 0x20 (report b094).
pub fn for_mesos(sn: i32) -> bool {
    (80_000_000..90_000_000).contains(&sn)
}

/// Whether a character of `gender` can buy `commodity` for mesos.
pub fn check_meso_sale(
    commodity: Option<&Commodity>,
    gender: i16,
) -> Result<&Commodity, CashShopFailure> {
    match commodity {
        Some(commodity) if for_mesos(commodity.sn) && commodity.on_sale && commodity.price > 0 => {
            if !commodity.for_gender(gender) {
                return Err(CashShopFailure::Gender);
            }
            Ok(commodity)
        }
        _ => Err(CashShopFailure::SoldOut),
    }
}

/// Pay the price of the mesos commodity `commodity` once, as v83 does,
/// whatever its count, and put the items in the player's bags.
pub fn buy_with_mesos(
    player: &mut Player,
    commodity: &Commodity,
) -> Result<Vec<InventoryChange>, CashShopFailure> {
    if commodity.price > player.meso {
        return Err(CashShopFailure::NotEnoughMesos);
    }
    let (item_id, count) = (commodity.item_id, commodity.count);
    if crate::items::refuses_only(&player.inventory, item_id, count.into()) {
        return Err(CashShopFailure::InventoryFull);
    }
    let changes = crate::items::give(player.inventory_mut(), item_id, count)
        .map_err(|_| CashShopFailure::InventoryFull)?;
    player.meso -= commodity.price;
    player.mark_dirty();
    Ok(changes)
}

/// The commodities in `commodity`, if it is a package (`CashPackage.img`).
pub fn package(commodity: &Commodity) -> Option<Vec<&'static Commodity>> {
    commodities().ok()?.package(commodity.item_id)
}

/// What an expansion commodity grows: `9110000` the storage, and
/// `9111000` to `9114000` the Equip, Use, Set-up and Etc bags.
pub fn expansion(commodity: &Commodity) -> Option<Expansion> {
    match commodity.item_id {
        9_110_000 => Some(Expansion::Storage),
        9_111_000 => Some(Expansion::Bag(InventoryType::Equip)),
        9_112_000 => Some(Expansion::Bag(InventoryType::Use)),
        9_113_000 => Some(Expansion::Bag(InventoryType::Setup)),
        9_114_000 => Some(Expansion::Bag(InventoryType::Etc)),
        _ => None,
    }
}

/// The Best Items for each gender: the best sellers still on sale, five at
/// most.
pub fn best_items(best_sellers: &[i32]) -> [Vec<i32>; 2] {
    [0, 1].map(|gender| {
        best_sellers
            .iter()
            .filter(|&&sn| check_purchase(commodity(sn), gender).is_ok())
            .copied()
            .take(5)
            .collect()
    })
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
/// stats, a pet comes out new and named after its kind (one that was out
/// before is looked up by the caller), and the item keeps its cash serial and
/// expiry. `None` if the item data has no such item.
pub fn inventory_item(cash_item: &CashItem) -> Option<Item> {
    let item = crate::items::create(cash_item.item_id, cash_item.quantity)?;
    let pet = game::pet::is_pet(item.item_id)
        .then(|| game::PetData::new(&crate::items::pet_name(item.item_id)));
    Some(Item {
        cash_serial: Some(cash_item.id),
        expiration: cash_item.expires_at,
        pet,
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
pub(crate) fn commodity_sn(item_id: i32, count: i16) -> i32 {
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
    fn pets_come_out_named_after_their_kind() {
        let pet = CashItem {
            id: 7,
            account_id: 3,
            item_id: 5000066,
            sn: 60000000,
            quantity: 1,
            gift_from: String::new(),
            expires_at: None,
            gift_message: None,
        };
        let item = inventory_item(&pet).unwrap();
        assert_eq!(item.cash_serial, Some(7));
        assert_eq!(item.pet, Some(game::PetData::new("Baby Tiger")));
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

        // Slot expansions are bought with their own request.
        let slots = Commodity {
            item_id: 9111000,
            ..ticket
        };
        assert_eq!(
            check_purchase(Some(&slots), 0),
            Err(CashShopFailure::Unknown)
        );
        assert_eq!(
            expansion(&slots),
            Some(Expansion::Bag(InventoryType::Equip))
        );
    }

    /// The Golden Bell, sold for 1 meso.
    fn golden_bell() -> Commodity {
        Commodity {
            sn: 80000001,
            item_id: 4031191,
            count: 1,
            price: 1,
            period: 0,
            gender: -1,
            on_sale: true,
        }
    }

    #[test]
    fn mesos_buy_only_the_mesos_commodities_on_sale() {
        let bell = golden_bell();
        assert_eq!(check_meso_sale(Some(&bell), 1), Ok(&bell));
        let for_men = Commodity {
            gender: 0,
            ..bell.clone()
        };
        assert_eq!(check_meso_sale(Some(&for_men), 0), Ok(&for_men));
        assert_eq!(
            check_meso_sale(Some(&for_men), 1),
            Err(CashShopFailure::Gender)
        );
        assert_eq!(check_meso_sale(None, 0), Err(CashShopFailure::SoldOut));
        assert_eq!(
            check_meso_sale(Some(&ticket()), 0),
            Err(CashShopFailure::SoldOut)
        );
        for retired in [
            Commodity {
                on_sale: false,
                ..bell.clone()
            },
            Commodity {
                price: 0,
                ..bell.clone()
            },
        ] {
            assert_eq!(
                check_meso_sale(Some(&retired), 0),
                Err(CashShopFailure::SoldOut)
            );
        }
        // NX doesn't buy them, for anyone.
        assert_eq!(
            check_purchase(Some(&bell), 0),
            Err(CashShopFailure::Unknown)
        );
        assert_eq!(check_gift(Some(&bell)), Err(CashShopFailure::Unknown));
    }

    #[test]
    fn mesos_pay_once_for_the_items_if_they_fit() {
        if crate::items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.meso = 4;
        let bells = Commodity {
            count: 3,
            price: 5,
            ..golden_bell()
        };
        assert_eq!(
            buy_with_mesos(&mut player, &bells),
            Err(CashShopFailure::NotEnoughMesos)
        );
        player.meso = 7;
        player.inventory.set_slot_limit(InventoryType::Etc, 0);
        let before = player.inventory.clone();
        assert_eq!(
            buy_with_mesos(&mut player, &bells),
            Err(CashShopFailure::InventoryFull)
        );
        assert_eq!((player.meso, &player.inventory), (7, &before));

        player.inventory.set_slot_limit(InventoryType::Etc, 24);
        let changes = buy_with_mesos(&mut player, &bells).unwrap();
        assert!(matches!(changes.as_slice(), [InventoryChange::Add { .. }]));
        assert_eq!((player.meso, player.inventory.count(4031191)), (2, 3));
    }

    #[test]
    fn packages_are_sold_and_gifted() {
        if commodities().is_err() {
            return;
        }
        // Package > Pet: the Baby Tiger Package.
        let package = commodity(10002307).unwrap();
        assert_eq!(check_purchase(Some(package), 1), Ok(package));
        assert_eq!(check_gift(Some(package)), Ok(package));
        let items: Vec<_> = super::package(package)
            .unwrap()
            .iter()
            .map(|c| c.item_id)
            .collect();
        assert_eq!(items, [5000066, 1802063, 5240027]);
        assert!(super::package(&ticket()).is_none());
    }

    #[test]
    fn wedding_tickets_are_sold_under_etc_wedding() {
        if commodities().is_err() {
            return;
        }
        // Etc > Wedding sells the four tickets; the Event-tab copies of
        // them are off sale.
        let tickets: Vec<_> = (50400000..=50400003)
            .map(|sn| check_purchase(commodity(sn), 1).map(|c| (c.item_id, c.price)))
            .collect();
        assert_eq!(
            tickets,
            [
                Ok((5251000, 25000)),
                Ok((5251001, 20000)),
                Ok((5251002, 24000)),
                Ok((5251003, 29000)),
            ]
        );
        assert_eq!(
            check_purchase(commodity(10100885), 0),
            Err(CashShopFailure::SoldOut)
        );
    }

    #[test]
    fn items_that_do_nothing_yet_are_sold_out() {
        let transfer = Commodity {
            item_id: 5401000,
            ..ticket()
        };
        assert_eq!(
            check_purchase(Some(&transfer), 0),
            Err(CashShopFailure::SoldOut)
        );
        assert_eq!(check_gift(Some(&transfer)), Err(CashShopFailure::SoldOut));
        let name_change = Commodity {
            item_id: NAME_CHANGE,
            ..ticket()
        };
        assert!(check_purchase(Some(&name_change), 0).is_ok());
        assert!(own_request(NAME_CHANGE) && !own_request(5401000));
        // The coupon is sold, with its own request; Maple Life is not.
        let coupon = Commodity {
            item_id: CHARACTER_SLOT,
            ..ticket()
        };
        assert!(check_purchase(Some(&coupon), 1).is_ok());
        assert!(own_request(CHARACTER_SLOT) && !own_request(5431000));
        if commodities().is_err() {
            return;
        }
        let on_sale = |item_id| {
            commodities()
                .unwrap()
                .iter()
                .any(|c| c.item_id == item_id && c.on_sale)
        };
        // Character Transfer, Maple Life.
        for item_id in [5401000, 5431000] {
            assert!(on_sale(item_id), "{item_id}");
            let sn = commodity_sn(item_id, 1);
            assert_eq!(
                check_purchase(commodity(sn), 0),
                Err(CashShopFailure::SoldOut),
                "{item_id}"
            );
        }
        // Gachapon Tickets, Pet Snacks, guild emoticons, Name Change, Miu
        // Miu, the Remote Gachapon Ticket, the Rock of Evolution, a hired
        // merchant and the Store Remote Controller still sell.
        for item_id in [
            5220000,
            crate::gachapon::REMOTE_TICKET,
            5460000,
            5290000,
            NAME_CHANGE,
            MIU_MIU,
            5380000,
            5030000,
            5470000,
        ] {
            let sn = commodity_sn(item_id, 1);
            assert!(check_purchase(commodity(sn), 0).is_ok(), "{item_id}");
        }
    }

    #[test]
    fn best_items_are_on_sale_for_each_gender() {
        if commodities().is_err() {
            return;
        }
        // A package, men's equipment, a retired hat, Gachapon Tickets.
        let sellers = [10002307, 20000053, 20000000, 50200003];
        assert_eq!(
            best_items(&sellers),
            [vec![10002307, 20000053, 50200003], vec![10002307, 50200003]]
        );
        assert_eq!(best_items(&[50200003; 7])[0].len(), 5);
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
