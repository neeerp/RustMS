//! Personal shops (PLAYER_INTERACTION, mini room type 4): a player with a
//! store permit opens a shop on a Free Market map and sells items in
//! bundles to up to three visitors at a time. The room lives in the field
//! and shows copies of the listings; the owner's client actor holds the
//! stock itself, out of their inventory, until it sells or the shop closes.
//! These are the rules for listing, buying and selling.

use crate::items;
use crate::packet::build::world::mini_room::{buy_result, SHOP_ITEMS};
use crate::shop::is_rechargeable;
use crate::trade::{self, Offer, TradeFailure, LOW_LEVEL_MAX, LOW_LEVEL_MESO_LIMIT};
use game::inventory::FLAG_UNTRADEABLE;
pub use game::mini_room::Listing;
use game::{InventoryChange, InventoryType, Item, Player};

/// What the owner or a visitor asked of a personal shop. Visiting,
/// chatting and leaving are shared with the trading room
/// (`trade::TradeRequest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersonalShopRequest {
    /// Open a shop named `title` with the permit in slot `permit_slot` of
    /// the cash bag.
    Create {
        title: String,
        permit_slot: i16,
        permit_id: i32,
    },
    /// List `bundles` bundles of `per_bundle` of the item in `slot` of bag
    /// `kind`, at `price` a bundle.
    PutItem {
        kind: i16,
        slot: i16,
        bundles: i16,
        per_bundle: i16,
        price: i32,
    },
    /// The owner's blacklist, sent just before the shop opens.
    Blacklist(Vec<String>),
    BlacklistAdd(String),
    BlacklistRemove(String),
    /// Open the shop to visitors.
    Open,
    /// Buy `bundles` bundles of listing `index`.
    Buy {
        index: u8,
        bundles: i16,
    },
    /// The owner takes listing `index` back.
    TakeBack {
        index: i16,
    },
    /// The owner turns the visitor in `slot` out; `overstayed` when their
    /// client does it after an hour.
    Ban {
        slot: u8,
        name: String,
        overstayed: bool,
    },
}

/// How many visitors a shop seats besides its owner.
pub const VISITORS: u8 = 3;

/// Whether `item_id` is a store permit (514xxxx).
pub fn is_permit(item_id: i32) -> bool {
    item_id / 10_000 == 514
}

/// The balloon a permit gives its shop (`ChatBalloon.img/miniroom/PSSkin`).
pub fn permit_skin(item_id: i32) -> u8 {
    (item_id % 100) as u8
}

/// Whether players may open shops on `map_id` (Map.nx `info/personalShop`,
/// set in the Free Market rooms).
pub fn allowed_on(map_id: i32) -> bool {
    crate::get_game_data()
        .ok()
        .and_then(|data| data.field(map_id))
        .is_some_and(|field| field.personal_shop)
}

/// The tax on a sale of `gross`, which the owner pays: as the v83 client
/// works it out for "Total Received" (FUN_006fa066), the gross times a
/// double rate, truncated. (The x87 runs at the 53-bit precision Windows
/// sets by default, so doubles match.)
pub fn tax(gross: i32) -> i32 {
    let rate = match gross {
        100_000_000.. => 0.03,
        25_000_000.. => 0.025,
        10_000_000.. => 0.02,
        5_000_000.. => 0.015,
        1_000_000.. => 0.009,
        100_000.. => 0.004,
        _ => return 0,
    };
    (f64::from(gross) * rate) as i32
}

/// Whether nothing is left to sell.
pub fn sold_out(listings: &[Listing]) -> bool {
    listings.iter().all(|listing| listing.bundles == 0)
}

/// Whether the item may be listed (FUN_006fd96c): not sealed, party quest
/// or account items; a cash, quest or untradeable item only with Karma,
/// which, unlike in a trade, lets a cash item in too.
pub fn sellable(item: &Item) -> bool {
    let bound = item.equip.is_some() && item.flags & FLAG_UNTRADEABLE != 0;
    !item.sealed()
        && items::info(item.item_id).is_some_and(|info| {
            let cash = item.is_cash() || info.equip.is_some_and(|equip| equip.cash);
            !info.pquest
                && !info.account_sharable
                && (item.has_karma() || !cash && !info.quest && !info.trade_block && !bound)
        })
}

/// Take the item in a bag slot out of the inventory to list it. Equipment
/// and sets of stars or bullets go whole, as one bundle. Returns the
/// listing and the inventory changes, or why the request is refused
/// (nothing changes then).
pub fn list_item(
    player: &mut Player,
    listings: &[Listing],
    kind: i16,
    slot: i16,
    bundles: i16,
    per_bundle: i16,
    price: i32,
) -> Result<(Listing, Vec<InventoryChange>), &'static str> {
    if listings.len() >= usize::from(SHOP_ITEMS) {
        return Err("the shop is full");
    }
    let kind = InventoryType::from_i16(kind)
        .filter(|&kind| kind != InventoryType::Equipped)
        .ok_or("not a bag")?;
    let item = player
        .inventory
        .get(kind, slot)
        .filter(|_| slot > 0)
        .ok_or("no item in that slot")?;
    if !sellable(item) {
        return Err("item can't be sold");
    }
    let (bundles, per_bundle) = if is_rechargeable(item.item_id) || item.equip.is_some() {
        (1, item.quantity)
    } else {
        (bundles, per_bundle)
    };
    let count = i32::from(bundles) * i32::from(per_bundle);
    if bundles < 1 || per_bundle < 1 || count > i32::from(item.quantity) {
        return Err("bad quantity");
    }
    if price < 1 || i64::from(price) * i64::from(bundles) > i64::from(i32::MAX) {
        return Err("bad price");
    }
    let listing = Listing {
        item: Item {
            quantity: per_bundle,
            ..item.clone()
        },
        bundles,
        price,
    };
    let changes = player
        .inventory_mut()
        .take(kind, slot, count as i16)
        .map_err(|_| "bad quantity")?;
    Ok((listing, changes))
}

/// What `bundles` of `listing` cost `buyer`, or the `buy_result` that says
/// why they can't have them.
pub fn check_purchase(buyer: &Player, listing: &Listing, bundles: i16) -> Result<i32, u8> {
    if bundles < 1 || bundles > listing.bundles {
        return Err(buy_result::NO_STOCK);
    }
    let total = i32::try_from(i64::from(listing.price) * i64::from(bundles))
        .map_err(|_| buy_result::TOO_PRICEY)?;
    if total > buyer.meso {
        return Err(buy_result::NO_MESOS);
    }
    if buyer.stats.level <= LOW_LEVEL_MAX && total > LOW_LEVEL_MESO_LIMIT {
        return Err(buy_result::LOW_LEVEL_LIMIT);
    }
    trade::check(buyer, &goods(vec![listing.take(bundles)])).map_err(|failure| match failure {
        TradeFailure::NoRoom => buy_result::INVENTORY_FULL,
        TradeFailure::OnlyOne => buy_result::ONLY_ONE,
    })?;
    Ok(total)
}

/// Whether the owner can sell `bundles` of listing `index`, still the item
/// `item_id`, for `paid`: in stock at that price, and the proceeds after
/// tax fit in their purse. Returns the `buy_result` for the buyer if not.
pub fn check_sale(
    owner: &Player,
    listings: &[Listing],
    index: usize,
    item_id: i32,
    bundles: i16,
    paid: i32,
) -> Result<(), u8> {
    let listing = listings
        .get(index)
        .filter(|listing| listing.item.item_id == item_id)
        .filter(|listing| (1..=listing.bundles).contains(&bundles))
        .ok_or(buy_result::NO_STOCK)?;
    if i64::from(listing.price) * i64::from(bundles) != i64::from(paid) {
        return Err(buy_result::NO_STOCK);
    }
    if i64::from(owner.meso) + i64::from(paid - tax(paid)) > i64::from(i32::MAX) {
        return Err(buy_result::TOO_PRICEY);
    }
    Ok(())
}

/// Sell what `check_sale` allowed: the bundles leave the listing and the
/// owner gets `paid` less tax. Returns the items for the buyer.
pub fn sell(
    owner: &mut Player,
    listings: &mut [Listing],
    index: usize,
    bundles: i16,
    paid: i32,
) -> Item {
    let listing = &mut listings[index];
    listing.bundles -= bundles;
    owner.meso += paid - tax(paid);
    owner.mark_dirty();
    listing.take(bundles)
}

/// The items of `listings` still unsold, to give back to the owner.
pub fn unsold(listings: &[Listing]) -> Offer {
    goods(
        listings
            .iter()
            .filter(|listing| listing.bundles > 0)
            .map(|listing| listing.take(listing.bundles))
            .collect(),
    )
}

/// Items as an offer without mesos, for `trade::check` and `trade::give`.
pub fn goods(items: Vec<Item>) -> Offer {
    Offer {
        items: (1..).zip(items).collect(),
        meso: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items;

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;
    const SWORD: i32 = 1302000;

    fn player(meso: i32, level: u8) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player.stats.level = level;
        player
    }

    fn potions(bundles: i16, per_bundle: i16, price: i32) -> Listing {
        Listing {
            item: Item::stack(RED_POTION, per_bundle),
            bundles,
            price,
        }
    }

    #[test]
    fn tax_follows_the_client_table_in_doubles() {
        assert_eq!(tax(0), 0);
        assert_eq!(tax(99_999), 0);
        assert_eq!(tax(100_000), 400);
        assert_eq!(tax(1_000_000), 9_000);
        assert_eq!(tax(1_234_567), 11_111);
        assert_eq!(tax(5_000_000), 75_000);
        assert_eq!(tax(10_000_000), 200_000);
        assert_eq!(tax(25_000_000), 625_000);
        assert_eq!(tax(100_000_000), 3_000_000);
        assert_eq!(tax(i32::MAX), 64_424_509);
    }

    #[test]
    fn permits_pick_the_balloon() {
        assert!(is_permit(5140000) && is_permit(5140006));
        assert!(!is_permit(5150000) && !is_permit(2000000));
        assert_eq!((permit_skin(5140000), permit_skin(5140006)), (0, 6));
    }

    #[test]
    fn listing_takes_whole_bundles_from_the_bag() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0, 30);
        items::give(player.inventory_mut(), RED_POTION, 30).unwrap();
        let (listing, changes) = list_item(&mut player, &[], 2, 1, 4, 5, 100).unwrap();
        assert_eq!(listing, potions(4, 5, 100));
        assert_eq!(
            changes,
            vec![InventoryChange::Quantity {
                kind: InventoryType::Use,
                slot: 1,
                quantity: 10
            }]
        );
        assert_eq!(player.inventory.count(RED_POTION), 10);
    }

    #[test]
    fn stars_and_equipment_list_whole() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0, 30);
        items::give(player.inventory_mut(), SUBI, 500).unwrap();
        items::give(player.inventory_mut(), SWORD, 1).unwrap();
        let (stars, _) = list_item(&mut player, &[], 2, 1, 7, 3, 9_000).unwrap();
        assert_eq!((stars.bundles, stars.item.quantity), (1, 500));
        let (sword, _) = list_item(&mut player, &[], 1, 1, 1, 1, 50_000).unwrap();
        assert!(sword.item.equip.is_some());
        assert_eq!(sword.bundles, 1);
        assert_eq!(
            player.inventory.count(SUBI) + player.inventory.count(SWORD),
            0
        );
    }

    #[test]
    fn bad_listings_change_nothing() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0, 30);
        items::give(player.inventory_mut(), RED_POTION, 10).unwrap();
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(5140000, 1));
        let before = player.inventory.clone();
        let full = vec![potions(1, 1, 1); 16];
        for (listings, kind, slot, bundles, per_bundle, price) in [
            (&full[..], 2, 1, 1, 1, 1),      // sixteen listed
            (&[][..], 2, 1, 3, 4, 1),        // more than there is
            (&[][..], 2, 1, 0, 1, 1),        // no bundles
            (&[][..], 2, 1, 1, 0, 1),        // empty bundles
            (&[][..], 2, 1, 1, 1, 0),        // free
            (&[][..], 2, 1, 2, 1, i32::MAX), // more than a purse holds
            (&[][..], 2, 2, 1, 1, 1),        // empty slot
            (&[][..], -1, -11, 1, 1, 1),     // equipped
            (&[][..], 5, 1, 1, 1, 1),        // cash item
        ] {
            assert!(
                list_item(
                    &mut player,
                    listings,
                    kind,
                    slot,
                    bundles,
                    per_bundle,
                    price
                )
                .is_err(),
                "{kind} {slot} {bundles} {per_bundle} {price}"
            );
        }
        assert_eq!(player.inventory, before);
    }

    #[test]
    fn karma_items_list_but_party_quest_items_never_do() {
        if items::data().is_err() {
            return;
        }
        let karma = |item_id| Item {
            flags: game::inventory::FLAG_BUNDLE_KARMA,
            ..Item::stack(item_id, 3)
        };
        // Tigun Transformation Bundle, a quest item that can't be traded,
        // and a first party quest coupon.
        let mut player = player(0, 10);
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2210005, 3));
        player
            .inventory
            .insert(InventoryType::Use, 2, karma(2210005));
        player
            .inventory
            .insert(InventoryType::Etc, 1, karma(4001007));
        assert!(list_item(&mut player, &[], 2, 1, 1, 1, 100).is_err());
        assert!(list_item(&mut player, &[], 4, 1, 1, 1, 100).is_err());
        assert!(list_item(&mut player, &[], 2, 2, 1, 1, 100).is_ok());
    }

    #[test]
    fn cash_items_list_only_with_karma_and_sealed_ones_never() {
        if items::data().is_err() {
            return;
        }
        let rock = Item::stack(5040000, 1);
        let sword = Item {
            cash_serial: Some(7),
            ..Item::equipment(1302000, Default::default())
        };
        assert!(!sellable(&rock) && !sellable(&sword));
        let (mut rock, mut sword) = (rock, sword);
        rock.give_karma();
        sword.give_karma();
        assert!(sellable(&rock) && sellable(&sword));
        rock.flags |= game::inventory::FLAG_LOCK;
        assert!(!sellable(&rock));
        assert!(
            !trade::tradeable(&sword),
            "a trade still refuses cash items"
        );
    }

    #[test]
    fn buyers_need_stock_mesos_and_room() {
        if items::data().is_err() {
            return;
        }
        let listing = potions(3, 10, 400_000);
        let buyer = player(1_000_000, 30);
        assert_eq!(check_purchase(&buyer, &listing, 2), Ok(800_000));
        assert_eq!(
            check_purchase(&buyer, &listing, 0),
            Err(buy_result::NO_STOCK)
        );
        assert_eq!(
            check_purchase(&buyer, &listing, 4),
            Err(buy_result::NO_STOCK)
        );
        assert_eq!(
            check_purchase(&buyer, &listing, 3),
            Err(buy_result::NO_MESOS)
        );
        let pricey = potions(3, 10, i32::MAX);
        assert_eq!(
            check_purchase(&player(i32::MAX, 30), &pricey, 2),
            Err(buy_result::TOO_PRICEY)
        );
        // Up to level 15, a million a purchase.
        let low = player(2_000_000, 15);
        assert_eq!(
            check_purchase(&low, &potions(3, 10, 500_000), 2),
            Ok(1_000_000)
        );
        assert_eq!(
            check_purchase(&low, &listing, 3),
            Err(buy_result::LOW_LEVEL_LIMIT)
        );

        let mut full = player(1_000_000, 30);
        full.inventory.set_slot_limit(InventoryType::Use, 1);
        items::give(full.inventory_mut(), 2000001, 1).unwrap();
        assert_eq!(
            check_purchase(&full, &listing, 1),
            Err(buy_result::INVENTORY_FULL)
        );
    }

    #[test]
    fn a_sale_moves_bundles_and_pays_the_owner_less_tax() {
        let mut owner = player(1_000, 30);
        let mut listings = vec![potions(3, 10, 400_000), potions(1, 5, 10)];
        assert_eq!(
            check_sale(&owner, &listings, 0, RED_POTION, 2, 800_000),
            Ok(())
        );
        for (index, item_id, bundles, paid) in [
            (2, RED_POTION, 1, 400_000),   // no such listing
            (0, SUBI, 1, 400_000),         // another item now
            (0, RED_POTION, 4, 1_600_000), // more than is left
            (0, RED_POTION, 1, 300_000),   // another price now
        ] {
            assert_eq!(
                check_sale(&owner, &listings, index, item_id, bundles, paid),
                Err(buy_result::NO_STOCK)
            );
        }
        let sold = sell(&mut owner, &mut listings, 0, 2, 800_000);
        assert_eq!(sold, Item::stack(RED_POTION, 20));
        assert_eq!(listings[0].bundles, 1);
        assert_eq!(owner.meso, 1_000 + 800_000 - 3_200);
        assert!(!sold_out(&listings));

        sell(&mut owner, &mut listings, 0, 1, 400_000);
        assert_eq!(unsold(&listings), goods(vec![Item::stack(RED_POTION, 5)]));
        sell(&mut owner, &mut listings, 1, 1, 10);
        assert!(sold_out(&listings));
        assert_eq!(unsold(&listings), goods(Vec::new()));

        owner.meso = i32::MAX - 100;
        let listings = vec![potions(1, 1, 1_000)];
        assert_eq!(
            check_sale(&owner, &listings, 0, RED_POTION, 1, 1_000),
            Err(buy_result::TOO_PRICEY)
        );
    }
}
