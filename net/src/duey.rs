//! Duey's parcel delivery (DUEY_ACTION, PARCEL; report b028): a character
//! sends mesos and an item (one stack) to another character, who claims or
//! deletes the parcel in the Duey window. A normal send costs a fee of 5000
//! mesos plus a tax on the mesos sent; a quick send uses up a Quick
//! Delivery Coupon instead of the 5000 and carries a message. The runtime
//! looks up the receiver, holds the open window, and saves the parcel with
//! the player in one transaction.

use crate::error::NetworkError;
use crate::packet::build::world::{
    inventory,
    parcel::build_parcel_removed,
    stat::{self, StatUpdate},
};
use crate::shop::is_rechargeable;
use crate::{items, storage};
use db::parcel::Parcel;
use game::{InventoryType, Item, Player};
use packet::Packet;

/// The Duey NPC, in every town with a post office.
pub const DUEY: i32 = 9010009;
/// The Quick Delivery Coupon (cash item type 533).
pub const QUICK_DELIVERY_COUPON: i32 = 5330000;
/// What a normal send costs on top of the tax.
pub const BASE_FEE: i32 = 5000;
/// The parcels the window lists, and so the most that may wait for one
/// character.
pub const INBOX_LIMIT: usize = 10;
/// The longest message a quick send carries, as the client's edit allows.
pub const MESSAGE_LIMIT: usize = 100;
/// Up to this level a character may send at most `LOW_LEVEL_MESOS` a day.
pub const LOW_LEVEL: u8 = 15;
pub const LOW_LEVEL_MESOS: i32 = 1_000_000;

/// PARCEL result modes, as the v83 client reads them (FUN_006f5be2).
pub mod result {
    /// "You do not have enough mesos."
    pub const NO_MESOS: u8 = 0x0a;
    /// "You have made an incorrect request."
    pub const INCORRECT: u8 = 0x0b;
    /// "Please double-check the receiver's name."
    pub const NO_RECEIVER: u8 = 0x0c;
    /// "You may not send the package to a character under the same
    /// account."
    pub const SAME_ACCOUNT: u8 = 0x0d;
    /// "The receiver's package storage is full."
    pub const INBOX_FULL: u8 = 0x0e;
    /// The receiver already has this one-of-a-kind item waiting.
    pub const ONLY_ONE_WAITING: u8 = 0x10;
    /// Level 15 and below may send at most a million mesos.
    pub const LOW_LEVEL_LIMIT: u8 = 0x11;
    /// "The package has been successfully sent."
    pub const SENT: u8 = 0x12;
    /// "An unknown error has occurred."
    pub const UNKNOWN: u8 = 0x13;
    /// The inventory has no room for the parcel's item.
    pub const INVENTORY_FULL: u8 = 0x15;
    /// The player already has the parcel's one-of-a-kind item.
    pub const ONLY_ONE: u8 = 0x16;
}

/// What the client asked of Duey (DUEY_ACTION).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DueyRequest {
    /// Open the window: the notifier of a quick delivery asks for it.
    Open,
    /// The player used a Quick Delivery Coupon (USE_CASH_ITEM): open the
    /// window for a quick send.
    OpenQuickSend,
    /// Send `mesos` and `quantity` of the item in `slot` of bag `kind` (no
    /// item if `kind` is 0 or `slot` is 0) to the character named `to`.
    /// A quick send carries a message.
    Send {
        kind: u8,
        slot: i16,
        quantity: i16,
        mesos: i32,
        to: String,
        quick: Option<String>,
    },
    Claim {
        id: i32,
    },
    Delete {
        id: i32,
    },
    Close,
}

/// The tax on sending `mesos`: none below 100,000, then a share of the
/// whole amount that grows with it (FUN_006eedfe).
pub fn tax(mesos: i32) -> i32 {
    let per_mille: i64 = match mesos {
        ..100_000 => 0,
        100_000..1_000_000 => 8,
        1_000_000..5_000_000 => 18,
        5_000_000..10_000_000 => 30,
        10_000_000..25_000_000 => 40,
        25_000_000..100_000_000 => 50,
        _ => 60,
    };
    (i64::from(mesos) * per_mille / 1000) as i32
}

/// The most mesos `player` may send in a day, if there is a most.
pub fn daily_mesos(player: &Player) -> Option<i32> {
    (player.stats.level <= LOW_LEVEL).then_some(LOW_LEVEL_MESOS)
}

/// What sending `mesos` costs on top of them.
pub fn fee(mesos: i32, quick: bool) -> i32 {
    tax(mesos) + if quick { 0 } else { BASE_FEE }
}

/// Whether the item may go in a parcel: what storage takes, but neither
/// items that expire nor those bound to an account, a party quest, or a
/// session. Nor sealed ones, which storage takes but v83 won't put in a
/// parcel (0x4f48d0).
pub fn sendable(item: &Item) -> bool {
    storage::storable(item)
        && !item.sealed()
        && item.expiration.is_none()
        && items::info(item.item_id)
            .is_some_and(|info| !info.account_sharable && !info.pquest && !info.expire_on_logout)
}

/// A send the player paid for: the parcel's item, and the replies that
/// show what left the player.
#[derive(Debug)]
pub struct Sending {
    pub item: Option<Item>,
    pub only: bool,
    pub packets: Vec<Packet>,
}

/// Take what a send costs from `player`: the mesos and fee, the item, and
/// for a quick send a coupon. A refusal is the result mode, and leaves the
/// player as they were.
pub fn send(
    player: &mut Player,
    kind: u8,
    slot: i16,
    quantity: i16,
    mesos: i32,
    quick: bool,
) -> Result<Result<Sending, u8>, NetworkError> {
    if mesos < 0 {
        return Ok(Err(result::INCORRECT));
    }
    let bag = InventoryType::from_i16(i16::from(kind))
        .filter(|&bag| bag != InventoryType::Equipped && slot > 0);
    let item = match bag {
        None => None,
        Some(bag) => match player
            .inventory
            .get(bag, slot)
            .filter(|item| sendable(item))
        {
            None => return Ok(Err(result::INCORRECT)),
            Some(item) => {
                let quantity = if is_rechargeable(item.item_id) || item.equip.is_some() {
                    item.quantity
                } else if (1..=item.quantity).contains(&quantity) {
                    quantity
                } else {
                    return Ok(Err(result::INCORRECT));
                };
                Some((bag, item.clone(), quantity))
            }
        },
    };
    if item.is_none() && mesos == 0 {
        return Ok(Err(result::INCORRECT));
    }
    if daily_mesos(player).is_some_and(|most| mesos > most) {
        return Ok(Err(result::LOW_LEVEL_LIMIT));
    }
    let cost = i64::from(mesos) + i64::from(fee(mesos, quick));
    if i64::from(player.meso) < cost {
        return Ok(Err(result::NO_MESOS));
    }
    let coupon = player
        .inventory
        .items(InventoryType::Cash)
        .find(|(_, item)| item.item_id == QUICK_DELIVERY_COUPON)
        .map(|(slot, _)| slot);
    if quick && coupon.is_none() {
        return Ok(Err(result::INCORRECT));
    }

    let mut changes = Vec::new();
    let sent = match item {
        None => None,
        Some((bag, item, quantity)) => {
            let Ok(taken) = player.inventory_mut().take(bag, slot, quantity) else {
                return Ok(Err(result::INCORRECT));
            };
            changes.extend(taken);
            let mut sent = Item { quantity, ..item };
            sent.use_karma();
            Some(sent)
        }
    };
    if let Some(coupon) = coupon.filter(|_| quick) {
        match player.inventory_mut().take(InventoryType::Cash, coupon, 1) {
            Ok(taken) => changes.extend(taken),
            Err(_) => return Ok(Err(result::INCORRECT)),
        }
    }
    player.meso -= cost as i32;
    player.mark_dirty();
    let mut packets = Vec::new();
    if !changes.is_empty() {
        packets.push(inventory::build_inventory_operation(true, &changes)?);
    }
    packets.push(stat::build_stat_update(
        true,
        &[StatUpdate::Meso(player.meso)],
    )?);
    let only = sent
        .as_ref()
        .is_some_and(|item| items::info(item.item_id).is_some_and(|info| info.only));
    Ok(Ok(Sending {
        item: sent,
        only,
        packets,
    }))
}

/// Give `player` what `parcel` holds, with the replies ending in the
/// window's "claimed". A refusal is the result mode, and leaves the player
/// as they were.
pub fn claim(
    player: &mut Player,
    parcel: &Parcel,
) -> Result<Result<Vec<Packet>, u8>, NetworkError> {
    let Ok(id) = i32::try_from(parcel.id) else {
        return Ok(Err(result::UNKNOWN));
    };
    let Ok(meso) = i32::try_from(i64::from(player.meso) + i64::from(parcel.mesos)) else {
        return Ok(Err(result::UNKNOWN));
    };
    let mut packets = Vec::new();
    if let Some(item) = &parcel.item {
        let only = items::info(item.item_id).is_some_and(|info| info.only);
        if only && player.inventory.count(item.item_id) > 0 {
            return Ok(Err(result::ONLY_ONE));
        }
        let added = if is_rechargeable(item.item_id) || item.equip.is_some() {
            player.inventory_mut().add_alone(item.clone())
        } else {
            let slot_max = items::slot_max(item.item_id);
            player.inventory_mut().add(item.clone(), slot_max)
        };
        let Ok(changes) = added else {
            return Ok(Err(result::INVENTORY_FULL));
        };
        packets.push(inventory::build_inventory_operation(true, &changes)?);
    }
    if parcel.mesos > 0 {
        player.meso = meso;
        packets.push(stat::build_stat_update(true, &[StatUpdate::Meso(meso)])?);
    }
    player.mark_dirty();
    packets.push(build_parcel_removed(
        id,
        crate::packet::build::world::parcel::CLAIMED,
    )?);
    Ok(Ok(packets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;

    fn player(meso: i32, level: u8) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player.stats.level = level;
        player
    }

    fn parcel(mesos: i32, item: Option<Item>) -> Parcel {
        Parcel {
            id: 7,
            sender: "Sender".into(),
            mesos,
            message: None,
            quick: false,
            sent_at: SystemTime::now(),
            item,
        }
    }

    #[test]
    fn the_tax_grows_with_the_mesos() {
        assert_eq!(tax(99_999), 0);
        assert_eq!(tax(100_000), 800);
        assert_eq!(tax(999_999), 7_999);
        assert_eq!(tax(1_000_000), 18_000);
        assert_eq!(tax(5_000_000), 150_000);
        assert_eq!(tax(10_000_000), 400_000);
        assert_eq!(tax(25_000_000), 1_250_000);
        assert_eq!(tax(100_000_000), 6_000_000);
        assert_eq!(tax(i32::MAX), 128_849_018);
        assert_eq!(fee(0, false), 5000);
        assert_eq!(fee(200_000, true), 1_600);
    }

    #[test]
    fn sending_mesos_takes_them_and_the_fee() {
        let mut sender = player(20_000, 30);
        let sent = send(&mut sender, 0, 0, 0, 10_000, false).unwrap().unwrap();
        assert!(sent.item.is_none());
        assert_eq!(sender.meso, 5_000);
        assert_eq!(sent.packets.len(), 1);

        let mut broke = player(14_999, 30);
        assert_eq!(
            send(&mut broke, 0, 0, 0, 10_000, false)
                .unwrap()
                .unwrap_err(),
            result::NO_MESOS
        );
        assert_eq!(broke.meso, 14_999);
        assert_eq!(
            send(&mut broke, 0, 0, 0, 0, false).unwrap().unwrap_err(),
            result::INCORRECT,
            "nothing to send"
        );
        assert_eq!(
            send(&mut broke, 0, 0, 0, -1, false).unwrap().unwrap_err(),
            result::INCORRECT
        );
    }

    #[test]
    fn a_low_level_sends_at_most_a_million() {
        let mut low = player(3_000_000, LOW_LEVEL);
        assert_eq!(
            send(&mut low, 0, 0, 0, 1_000_001, false)
                .unwrap()
                .unwrap_err(),
            result::LOW_LEVEL_LIMIT
        );
        assert!(send(&mut low, 0, 0, 0, 1_000_000, false).unwrap().is_ok());
        assert_eq!(low.meso, 3_000_000 - 1_000_000 - 18_000 - 5_000);
    }

    #[test]
    fn only_low_levels_have_a_most_a_day() {
        assert_eq!(daily_mesos(&player(0, LOW_LEVEL)), Some(LOW_LEVEL_MESOS));
        assert_eq!(daily_mesos(&player(0, LOW_LEVEL + 1)), None);
    }

    #[test]
    fn a_quick_send_needs_a_coupon_and_uses_it() {
        if items::data().is_err() {
            return;
        }
        let mut sender = player(1_000, 30);
        assert_eq!(
            send(&mut sender, 0, 0, 0, 1_000, true)
                .unwrap()
                .unwrap_err(),
            result::INCORRECT
        );
        items::give(sender.inventory_mut(), QUICK_DELIVERY_COUPON, 2).unwrap();
        let sent = send(&mut sender, 0, 0, 0, 1_000, true).unwrap().unwrap();
        assert_eq!(sender.meso, 0, "no base fee");
        assert_eq!(sender.inventory.count(QUICK_DELIVERY_COUPON), 1);
        assert_eq!(sent.packets.len(), 2);
    }

    #[test]
    fn sending_an_item_takes_part_of_its_stack() {
        if items::data().is_err() {
            return;
        }
        let mut sender = player(10_000, 30);
        items::give(sender.inventory_mut(), RED_POTION, 30).unwrap();
        let use_bag = InventoryType::Use as u8;
        assert_eq!(
            send(&mut sender, use_bag, 1, 31, 0, false)
                .unwrap()
                .unwrap_err(),
            result::INCORRECT
        );
        assert_eq!(
            send(&mut sender, use_bag, 2, 1, 0, false)
                .unwrap()
                .unwrap_err(),
            result::INCORRECT,
            "an empty slot"
        );
        let sent = send(&mut sender, use_bag, 1, 10, 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(sent.item.unwrap().quantity, 10);
        assert_eq!(sender.inventory.count(RED_POTION), 20);
        assert_eq!(sender.meso, 5_000);

        // Stars go whole.
        let mut thrower = player(10_000, 30);
        items::give(thrower.inventory_mut(), SUBI, 500).unwrap();
        let sent = send(&mut thrower, use_bag, 1, 1, 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(sent.item.unwrap().quantity, 500);
        assert_eq!(thrower.inventory.count(SUBI), 0);
    }

    #[test]
    fn an_item_that_expires_stays() {
        if items::data().is_err() {
            return;
        }
        let mut sender = player(10_000, 30);
        let expiring = Item {
            expiration: Some(SystemTime::now()),
            ..Item::stack(RED_POTION, 1)
        };
        sender
            .inventory_mut()
            .insert(InventoryType::Use, 1, expiring);
        assert_eq!(
            send(&mut sender, InventoryType::Use as u8, 1, 1, 0, false)
                .unwrap()
                .unwrap_err(),
            result::INCORRECT
        );
        assert_eq!(sender.meso, 10_000);
    }

    #[test]
    fn a_sealed_item_stays() {
        if items::data().is_err() {
            return;
        }
        let mut sender = player(10_000, 30);
        let sealed = Item {
            flags: game::inventory::FLAG_LOCK,
            ..Item::stack(RED_POTION, 1)
        };
        sender.inventory_mut().insert(InventoryType::Use, 1, sealed);
        assert_eq!(
            send(&mut sender, InventoryType::Use as u8, 1, 1, 0, false)
                .unwrap()
                .unwrap_err(),
            result::INCORRECT
        );
        assert_eq!(sender.inventory.count(RED_POTION), 1);
    }

    #[test]
    fn claiming_gives_the_mesos_and_item() {
        if items::data().is_err() {
            return;
        }
        let mut receiver = player(100, 30);
        let packets = claim(
            &mut receiver,
            &parcel(500, Some(Item::stack(RED_POTION, 5))),
        )
        .unwrap()
        .unwrap();
        assert_eq!(receiver.meso, 600);
        assert_eq!(receiver.inventory.count(RED_POTION), 5);
        assert_eq!(packets.len(), 3);
        assert_eq!(&packets[2].bytes[2..], &[0x17, 7, 0, 0, 0, 4]);

        let mut rich = player(i32::MAX - 10, 30);
        assert_eq!(
            claim(&mut rich, &parcel(11, None)).unwrap().unwrap_err(),
            result::UNKNOWN
        );
        assert_eq!(rich.meso, i32::MAX - 10);
    }
}
