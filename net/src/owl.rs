//! The Owl of Minerva (2310000 in USE, 5230000 in Cash; report c024): in
//! the Free Market, it
//! searches every channel's personal shops for an item and lists who sells
//! it and for how much. The client filters item names itself and sends
//! only the id it settled on; it is used up only if something turns up.
//! GO on a row takes the player to the shop, on their own channel.

use game::{InventoryType, Item};
use std::collections::HashMap;

/// The Owl of Minerva, the only USE item of class 231.
pub const OWL: i32 = 2_310_000;

/// The Cash Owl, the only Cash item of class 523. It opens the same
/// window (FUN_00a0a63f type 0x1d calls FUN_00a0a2dc).
pub const CASH_OWL: i32 = 5_230_000;

/// The Owl that tab holds.
pub fn owl_in(inventory: InventoryType) -> Option<i32> {
    match inventory {
        InventoryType::Use => Some(OWL),
        InventoryType::Cash => Some(CASH_OWL),
        _ => None,
    }
}

/// The most rows a search answers with ("Show 200 results").
pub const MAX_RESULTS: usize = 200;

/// How many items the window's "most searched" list shows.
pub const POPULAR: usize = 10;

/// The Free Market entrance and its 22 rooms, where the client lets the
/// Owl be used (FUN_00a0a2dc: 910000000 to 910000022).
pub fn in_free_market(map_id: i32) -> bool {
    (910_000_000..=910_000_022).contains(&map_id)
}

/// The Free Market rooms, whose shops the Owl searches.
pub fn is_market_room(map_id: i32) -> bool {
    (910_000_001..=910_000_022).contains(&map_id)
}

/// What the Owl's windows ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwlRequest {
    /// The search window opened and wants the most searched items (0x42).
    Popular,
    /// Search the shops for `item_id`, using the Owl in `slot` of
    /// `inventory` (USE, or Cash for the Cash Owl); the cheapest first
    /// unless `descending` (0x53, or 0x4F for the Cash Owl).
    Search {
        inventory: InventoryType,
        slot: i16,
        owl_id: i32,
        item_id: i32,
        descending: bool,
    },
    /// GO on the row of shop `room` on `map_id` (0x43).
    Go { room: i32, map_id: i32 },
}

/// One shop's listing of the item searched for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopHit {
    pub owner: String,
    pub title: String,
    pub map_id: i32,
    pub room: i32,
    /// 0-based, as the client numbers its channels.
    pub channel_id: u8,
    /// One bundle.
    pub item: Item,
    pub bundles: i16,
    pub price: i32,
}

/// The rows a search shows: by price a bundle, cheapest first unless
/// `descending`, at most `MAX_RESULTS`.
pub fn rank(mut hits: Vec<ShopHit>, descending: bool) -> Vec<ShopHit> {
    hits.sort_by_key(|hit| (hit.price, hit.channel_id, hit.map_id, hit.room));
    if descending {
        hits.reverse();
    }
    hits.truncate(MAX_RESULTS);
    hits
}

/// Why GO fails, or 0 for go (OWL_LINK_RESULT, FUN_008a4e7a).
pub mod go_result {
    pub const GO: u8 = 0;
    /// "The room is already closed."
    pub const CLOSED: u8 = 1;
    /// "You can't enter the room due to full capacity."
    pub const FULL: u8 = 2;
    /// "Other requests are being fulfilled this minute."
    pub const BUSY: u8 = 3;
    /// "You can't do it while you're dead."
    pub const DEAD: u8 = 4;
    /// "You may not enter this store."
    pub const BARRED: u8 = 0x11;
    /// "The owner of the store is currently undergoing store maintenance.
    /// ..." (c024).
    pub const MAINTENANCE: u8 = 0x12;
}

/// How often each item was searched for in the world since it started.
#[derive(Default)]
pub struct Searches {
    counts: HashMap<i32, u32>,
}

impl Searches {
    pub fn record(&mut self, item_id: i32) {
        *self.counts.entry(item_id).or_default() += 1;
    }

    /// The `POPULAR` most searched items, most first.
    pub fn popular(&self) -> Vec<i32> {
        let mut items: Vec<(i32, u32)> = self.counts.iter().map(|(&id, &n)| (id, n)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        items.into_iter().take(POPULAR).map(|(id, _)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(price: i32, channel_id: u8) -> ShopHit {
        ShopHit {
            owner: "a".into(),
            title: "b".into(),
            map_id: 910_000_001,
            room: 1,
            channel_id,
            item: Item::stack(4_000_000, 1),
            bundles: 1,
            price,
        }
    }

    #[test]
    fn each_owl_has_its_tab() {
        assert_eq!(owl_in(InventoryType::Use), Some(OWL));
        assert_eq!(owl_in(InventoryType::Cash), Some(CASH_OWL));
        assert_eq!(owl_in(InventoryType::Etc), None);
    }

    #[test]
    fn the_market_takes_the_entrance_and_its_rooms_only() {
        assert!(in_free_market(910_000_000));
        assert!(in_free_market(910_000_022));
        assert!(!in_free_market(910_000_023));
        assert!(!in_free_market(100_000_000));
    }

    #[test]
    fn results_go_by_price_either_way_and_stop_at_200() {
        let hits = vec![hit(30, 0), hit(10, 1), hit(20, 0)];
        let prices = |hits: Vec<ShopHit>| hits.iter().map(|h| h.price).collect::<Vec<_>>();
        assert_eq!(prices(rank(hits.clone(), false)), [10, 20, 30]);
        assert_eq!(prices(rank(hits, true)), [30, 20, 10]);
        assert_eq!(rank(vec![hit(1, 0); 250], false).len(), MAX_RESULTS);
    }

    #[test]
    fn the_most_searched_come_first() {
        let mut searches = Searches::default();
        for id in [3, 1, 3, 2, 3, 1] {
            searches.record(id);
        }
        assert_eq!(searches.popular(), [3, 1, 2]);
        for id in 10..30 {
            searches.record(id);
        }
        assert_eq!(searches.popular().len(), POPULAR);
    }
}
