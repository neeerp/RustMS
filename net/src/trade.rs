//! Trading between two players in a trading room (PLAYER_INTERACTION, mini
//! room type 3). The room itself lives in the field; each client actor holds
//! what its player put up, already out of their inventory, until the trade
//! ends. These are the rules for putting things up and for taking them in.

use crate::items;
use crate::loot::Loot;
use crate::packet::build::world::mini_room::leave_reason;
use crate::shop::is_rechargeable;
use crate::storage::storable;
use game::{Inventory, InventoryChange, InventoryError, InventoryType, Item, Player};
use std::collections::BTreeMap;

/// What the client asked of its trading room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradeRequest {
    /// Open a trading room.
    Create,
    /// Invite the character with this id, on the same map.
    Invite {
        target: i32,
    },
    /// Turn down room `sn`'s invitation; `reason` is an `invite_result`.
    Decline {
        sn: i32,
        reason: u8,
    },
    /// Accept room `sn`'s invitation.
    Visit {
        sn: i32,
    },
    Chat {
        text: String,
    },
    /// Put `count` of the item in `slot` of bag `kind` up in trade slot
    /// `trade_slot`.
    PutItem {
        kind: i16,
        slot: i16,
        count: i16,
        trade_slot: u8,
    },
    /// Put up this many more mesos.
    Meso(i32),
    /// The player pressed Trade.
    Confirm,
    Leave,
}

/// How many items each side may put up.
pub const TRADE_SLOTS: u8 = 9;

/// Below this level a player may put up at most `LOW_LEVEL_MESO_LIMIT`.
pub const LOW_LEVEL_MAX: u8 = 15;
pub const LOW_LEVEL_MESO_LIMIT: i32 = 1_000_000;

/// What one side put up: items by trade slot, and mesos.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Offer {
    pub items: BTreeMap<u8, Item>,
    pub meso: i32,
}

/// Why a trade couldn't go through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeFailure {
    /// No room for the items, or too many mesos.
    NoRoom,
    /// A one-of-a-kind item the receiver already has, or two of them.
    OnlyOne,
}

impl TradeFailure {
    /// The LEAVE reason that tells the client.
    pub fn reason(self) -> u8 {
        match self {
            Self::NoRoom => leave_reason::FAILED,
            Self::OnlyOne => leave_reason::ONE_OF_A_KIND,
        }
    }
}

/// The fee on `meso` received in a trade, rounded down.
pub fn fee(meso: i32) -> i32 {
    let meso = i64::from(meso.max(0));
    let per_thousand = match meso {
        100_000_000.. => 60,
        25_000_000.. => 50,
        10_000_000.. => 40,
        5_000_000.. => 30,
        1_000_000.. => 18,
        100_000.. => 8,
        _ => 0,
    };
    (meso * per_thousand / 1000) as i32
}

/// What the partner's offer comes to on this side: the same items with
/// their Karma used up, and the mesos less the fee.
pub fn received(offer: &Offer) -> Offer {
    let mut items = offer.items.clone();
    items.values_mut().for_each(Item::use_karma);
    Offer {
        items,
        meso: offer.meso - fee(offer.meso),
    }
}

/// Whether the item may be put up in a trade (FUN_007c359f): what storage
/// takes, short of items that stay within the account.
pub fn tradeable(item: &Item) -> bool {
    storable(item) && !items::info(item.item_id).is_some_and(|info| info.account_sharable)
}

/// Take `count` of the item in a bag slot out of the inventory and into
/// `offer` at `trade_slot`. Returns the item as offered and the inventory
/// changes, or why the request is refused (nothing changes then).
pub fn put_item(
    player: &mut Player,
    offer: &mut Offer,
    kind: i16,
    slot: i16,
    count: i16,
    trade_slot: u8,
) -> Result<(Item, Vec<InventoryChange>), &'static str> {
    if !(1..=TRADE_SLOTS).contains(&trade_slot) || offer.items.contains_key(&trade_slot) {
        return Err("trade slot taken or out of range");
    }
    let kind = InventoryType::from_i16(kind)
        .filter(|&kind| kind != InventoryType::Equipped)
        .ok_or("not a bag")?;
    let item = player
        .inventory
        .get(kind, slot)
        .filter(|_| slot > 0)
        .ok_or("no item in that slot")?;
    if !tradeable(item) {
        return Err("item can't be traded");
    }
    // A set of stars or bullets goes whole, like equipment.
    let count = if is_rechargeable(item.item_id) || item.equip.is_some() {
        item.quantity
    } else if (1..=item.quantity).contains(&count) {
        count
    } else {
        return Err("bad quantity");
    };
    let offered = Item {
        quantity: count,
        ..item.clone()
    };
    let changes = player
        .inventory_mut()
        .take(kind, slot, count)
        .map_err(|_| "bad quantity")?;
    offer.items.insert(trade_slot, offered.clone());
    Ok((offered, changes))
}

/// Whether a player of `level` may not put `amount` more on top of
/// `offered`.
pub fn over_meso_limit(level: u8, offered: i32, amount: i32) -> bool {
    level <= LOW_LEVEL_MAX && i64::from(offered) + i64::from(amount) > LOW_LEVEL_MESO_LIMIT.into()
}

/// Move `amount` of the player's mesos into `offer`.
pub fn put_meso(player: &mut Player, offer: &mut Offer, amount: i32) -> Result<(), &'static str> {
    if amount < 1 || amount > player.meso {
        return Err("bad meso amount");
    }
    player.meso -= amount;
    player.mark_dirty();
    offer.meso += amount;
    Ok(())
}

/// Put one item in the inventory the way it came: equipment and sets of
/// stars or bullets in a slot of their own, anything else stacked.
fn add(inventory: &mut Inventory, item: Item) -> Result<Vec<InventoryChange>, InventoryError> {
    if is_rechargeable(item.item_id) || item.equip.is_some() {
        inventory.add_alone(item)
    } else {
        let slot_max = items::slot_max(item.item_id);
        inventory.add(item, slot_max)
    }
}

/// Whether `player` can take all of `incoming`: no one-of-a-kind item they
/// already have (or two of one), room for every item, and mesos under the
/// cap.
pub fn check(player: &Player, incoming: &Offer) -> Result<(), TradeFailure> {
    let mut seen = Vec::new();
    for item in incoming.items.values() {
        let only = items::info(item.item_id).is_some_and(|info| info.only);
        if only && (player.inventory.count(item.item_id) > 0 || seen.contains(&item.item_id)) {
            return Err(TradeFailure::OnlyOne);
        }
        seen.push(item.item_id);
    }
    let mut inventory = player.inventory.clone();
    for item in incoming.items.values() {
        add(&mut inventory, item.clone()).map_err(|_| TradeFailure::NoRoom)?;
    }
    if i64::from(player.meso) + i64::from(incoming.meso) > i64::from(i32::MAX) {
        return Err(TradeFailure::NoRoom);
    }
    Ok(())
}

/// What giving a player an offer did.
#[derive(Debug, Default, PartialEq)]
pub struct Given {
    pub changes: Vec<InventoryChange>,
    /// What didn't fit, to put on the ground at the player's feet.
    pub overflow: Vec<Loot>,
}

/// Put `offer` in the player's inventory and purse: their own offer back,
/// or the partner's after a trade. Whatever no longer fits (the inventory
/// may have filled since `check`) comes back as overflow.
pub fn give(player: &mut Player, offer: Offer) -> Given {
    let mut given = Given::default();
    for item in offer.items.into_values() {
        match add(player.inventory_mut(), item.clone()) {
            Ok(changes) => given.changes.extend(changes),
            Err(_) => given.overflow.push(Loot::Dropped(item)),
        }
    }
    if offer.meso > 0 {
        let room = i32::MAX - player.meso.max(0);
        let kept = offer.meso.min(room);
        player.meso += kept;
        player.mark_dirty();
        if kept < offer.meso {
            given.overflow.push(Loot::Mesos(offer.meso - kept));
        }
    }
    given
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::inventory::{FLAG_BUNDLE_KARMA, FLAG_EQUIP_KARMA, FLAG_UNTRADEABLE};

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;
    const SWORD: i32 = 1302000;
    const SNAIL_SHELL: i32 = 4000000;

    fn player(meso: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player
    }

    fn offer(items: Vec<Item>, meso: i32) -> Offer {
        Offer {
            items: (1..).zip(items).collect(),
            meso,
        }
    }

    #[test]
    fn the_fee_follows_the_table_and_rounds_down() {
        assert_eq!(fee(0), 0);
        assert_eq!(fee(99_999), 0);
        assert_eq!(fee(100_000), 800);
        assert_eq!(fee(999_999), 7_999);
        assert_eq!(fee(1_000_000), 18_000);
        assert_eq!(fee(1_234_567), 22_222);
        assert_eq!(fee(5_000_000), 150_000);
        assert_eq!(fee(10_000_000), 400_000);
        assert_eq!(fee(25_000_000), 1_250_000);
        assert_eq!(fee(100_000_000), 6_000_000);
        assert_eq!(fee(i32::MAX), 128_849_018);
        assert_eq!(received(&offer(Vec::new(), 200_000)).meso, 198_400);
    }

    #[test]
    fn putting_an_item_up_takes_it_from_the_bag() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        items::give(player.inventory_mut(), RED_POTION, 30).unwrap();
        let mut offer = Offer::default();
        let (item, changes) = put_item(&mut player, &mut offer, 2, 1, 10, 3).unwrap();
        assert_eq!((item.item_id, item.quantity), (RED_POTION, 10));
        assert_eq!(
            changes,
            vec![InventoryChange::Quantity {
                kind: InventoryType::Use,
                slot: 1,
                quantity: 20
            }]
        );
        assert_eq!(offer.items[&3], item);
        assert_eq!(player.inventory.count(RED_POTION), 20);
    }

    #[test]
    fn stars_and_equipment_go_whole() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        items::give(player.inventory_mut(), SUBI, 500).unwrap();
        items::give(player.inventory_mut(), SWORD, 1).unwrap();
        let mut offer = Offer::default();
        let (stars, _) = put_item(&mut player, &mut offer, 2, 1, 1, 1).unwrap();
        assert_eq!(stars.quantity, 500);
        let (sword, _) = put_item(&mut player, &mut offer, 1, 1, 0, 2).unwrap();
        assert!(sword.equip.is_some());
        assert_eq!(
            player.inventory.count(SUBI) + player.inventory.count(SWORD),
            0
        );
    }

    #[test]
    fn bad_item_requests_change_nothing() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        items::give(player.inventory_mut(), RED_POTION, 5).unwrap();
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(5000000, 1));
        let mut offer = offer(vec![Item::stack(SNAIL_SHELL, 1)], 0);
        let before = (player.inventory.clone(), offer.clone());
        for (kind, slot, count, trade_slot) in [
            (2, 1, 1, 1),    // trade slot taken
            (2, 1, 1, 0),    // trade slot out of range
            (2, 1, 1, 10),   // trade slot out of range
            (-1, -11, 1, 2), // equipped
            (2, 2, 1, 2),    // empty slot
            (2, 1, 6, 2),    // more than there is
            (2, 1, 0, 2),    // none
            (5, 1, 1, 2),    // cash item
        ] {
            assert!(
                put_item(&mut player, &mut offer, kind, slot, count, trade_slot).is_err(),
                "{kind} {slot} {count} {trade_slot}"
            );
        }
        assert_eq!((player.inventory, offer), before);
    }

    #[test]
    fn untradeable_items_go_up_once_with_karma() {
        if items::data().is_err() {
            return;
        }
        // Tigun Transformation Bundle, a quest item that can't be traded;
        // earrings made untradeable once worn; a ring kept to the account.
        const TIGUN: i32 = 2210005;
        let bound = Item {
            flags: FLAG_UNTRADEABLE,
            ..items::create(1032031, 1).unwrap()
        };
        let karma = Item {
            flags: FLAG_EQUIP_KARMA,
            ..bound.clone()
        };
        let mut player = player(0);
        let inventory = &mut player.inventory;
        inventory.insert(InventoryType::Use, 1, Item::stack(TIGUN, 3));
        inventory.insert(
            InventoryType::Use,
            2,
            Item {
                flags: FLAG_BUNDLE_KARMA,
                ..Item::stack(TIGUN, 3)
            },
        );
        inventory.insert(InventoryType::Equip, 1, bound);
        inventory.insert(InventoryType::Equip, 2, karma);
        inventory.insert(InventoryType::Equip, 3, items::create(1112405, 1).unwrap());
        let mut offer = Offer::default();
        for (kind, slot) in [(2, 1), (1, 1), (1, 3)] {
            assert!(put_item(&mut player, &mut offer, kind, slot, 1, 1).is_err());
        }
        put_item(&mut player, &mut offer, 2, 2, 3, 1).unwrap();
        put_item(&mut player, &mut offer, 1, 2, 1, 2).unwrap();

        // The partner gets them with the Karma used up; given back, they
        // keep it.
        let got = received(&offer);
        assert_eq!(
            (got.items[&1].flags, got.items[&2].flags),
            (0, FLAG_UNTRADEABLE)
        );
        assert!(offer.items.values().all(Item::has_karma));
    }

    #[test]
    fn mesos_move_into_the_offer_within_what_is_carried() {
        let mut player = player(1000);
        let mut offer = Offer::default();
        put_meso(&mut player, &mut offer, 300).unwrap();
        put_meso(&mut player, &mut offer, 200).unwrap();
        assert_eq!((player.meso, offer.meso), (500, 500));
        assert!(put_meso(&mut player, &mut offer, 501).is_err());
        assert!(put_meso(&mut player, &mut offer, 0).is_err());
        assert!(put_meso(&mut player, &mut offer, -5).is_err());
        assert_eq!((player.meso, offer.meso), (500, 500));

        assert!(over_meso_limit(15, 900_000, 100_001));
        assert!(!over_meso_limit(15, 900_000, 100_000));
        assert!(!over_meso_limit(16, 900_000, 100_001));
    }

    #[test]
    fn checking_counts_stacks_slots_and_the_meso_cap() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        items::give(player.inventory_mut(), SNAIL_SHELL, 10).unwrap();
        let limit = usize::from(player.inventory.slot_limit(InventoryType::Etc));
        while player.inventory.items(InventoryType::Etc).count() < limit - 1 {
            player
                .inventory_mut()
                .add_alone(Item::stack(4000001, 1))
                .unwrap();
        }
        // One free slot: shells stack onto the existing ones, one new item fits.
        let fits = offer(
            vec![Item::stack(SNAIL_SHELL, 50), Item::stack(4000002, 3)],
            1000,
        );
        assert_eq!(check(&player, &fits), Ok(()));
        let too_many = offer(vec![Item::stack(4000002, 3), Item::stack(4000003, 3)], 0);
        assert_eq!(check(&player, &too_many), Err(TradeFailure::NoRoom));

        player.meso = i32::MAX - 10;
        assert_eq!(check(&player, &offer(Vec::new(), 10)), Ok(()));
        assert_eq!(
            check(&player, &offer(Vec::new(), 11)),
            Err(TradeFailure::NoRoom)
        );
    }

    #[test]
    fn one_of_a_kind_items_come_once() {
        if items::data().is_err() {
            return;
        }
        let Some(only) =
            (4031000..4032000).find(|&id| items::info(id).is_some_and(|info| info.only))
        else {
            return;
        };
        let mut player = player(0);
        let one = offer(vec![Item::stack(only, 1)], 0);
        assert_eq!(check(&player, &one), Ok(()));
        let two = offer(vec![Item::stack(only, 1), Item::stack(only, 1)], 0);
        assert_eq!(check(&player, &two), Err(TradeFailure::OnlyOne));
        player
            .inventory_mut()
            .add_alone(Item::stack(only, 1))
            .unwrap();
        assert_eq!(check(&player, &one), Err(TradeFailure::OnlyOne));
    }

    #[test]
    fn giving_puts_what_fits_away_and_drops_the_rest() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(i32::MAX - 5);
        player.inventory.set_slot_limit(InventoryType::Use, 1);
        let given = give(
            &mut player,
            offer(vec![Item::stack(RED_POTION, 10), Item::stack(SUBI, 100)], 8),
        );
        assert_eq!(player.inventory.count(RED_POTION), 10);
        assert_eq!(player.inventory.count(SUBI), 0);
        assert_eq!(player.meso, i32::MAX);
        assert_eq!(given.changes.len(), 1);
        assert_eq!(
            given.overflow,
            vec![Loot::Dropped(Item::stack(SUBI, 100)), Loot::Mesos(3)]
        );
    }
}
