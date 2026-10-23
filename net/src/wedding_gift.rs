//! The wedding registry (reports b084, b085): at a wedding a guest gives
//! one of the couple an item from Pila's give window, beside the partner's
//! wishlist; the item waits for them, kept in the database, until they take
//! it in Pila's receive window. The runtime holds the open window, and
//! saves each gift with the player who gave or took it in one transaction.

use crate::duey;
use crate::error::NetworkError;
use crate::items;
use crate::packet::build::world::{inventory, marriage::gift_bag};
use crate::shop::is_rechargeable;
use db::wedding_gift::Gift;
use game::{InventoryType, Item, Player};
use packet::Packet;

/// Whether `item` may be a gift: what may go in a parcel, which the give
/// window's list of the player's items keeps to as well.
pub fn giftable(item: &Item) -> bool {
    duey::sendable(item)
}

/// Take from `player` the gift the give window asked for: `quantity` of
/// `item_id` in its bag's `slot`, or the whole of an equip or a stack of
/// stars. The gift and the reply that shows it gone; `None`, and the
/// player as they were, if it isn't there or may not be given.
pub fn give(
    player: &mut Player,
    slot: i16,
    item_id: i32,
    quantity: i16,
) -> Result<Option<(Item, Packet)>, NetworkError> {
    let Some(bag) = InventoryType::for_item(item_id).filter(|_| slot > 0) else {
        return Ok(None);
    };
    let Some(item) = player
        .inventory
        .get(bag, slot)
        .filter(|item| item.item_id == item_id && giftable(item))
    else {
        return Ok(None);
    };
    let quantity = if is_rechargeable(item_id) || item.equip.is_some() {
        item.quantity
    } else if (1..=item.quantity).contains(&quantity) {
        quantity
    } else {
        return Ok(None);
    };
    let item = item.clone();
    let Ok(changes) = player.inventory_mut().take(bag, slot, quantity) else {
        return Ok(None);
    };
    player.mark_dirty();
    let mut gift = Item { quantity, ..item };
    gift.use_karma();
    let packet = inventory::build_inventory_operation(true, &changes)?;
    Ok(Some((gift, packet)))
}

/// Give `player` the gift `item`, with the reply that shows it; `None`, and
/// the player as they were, if it doesn't fit or is one of a kind they
/// already hold.
pub fn receive(player: &mut Player, item: &Item) -> Result<Option<Packet>, NetworkError> {
    let only = items::info(item.item_id).is_some_and(|info| info.only);
    if only && player.inventory.count(item.item_id) > 0 {
        return Ok(None);
    }
    let added = if is_rechargeable(item.item_id) || item.equip.is_some() {
        player.inventory_mut().add_alone(item.clone())
    } else {
        player
            .inventory_mut()
            .add(item.clone(), items::slot_max(item.item_id))
    };
    let Ok(changes) = added else {
        return Ok(None);
    };
    player.mark_dirty();
    Ok(Some(inventory::build_inventory_operation(true, &changes)?))
}

/// Where among `gifts`, as the receive window last listed them, is the
/// `index`th row of the bag of type `kind`, which the take request names.
/// A bag lists at most 255.
pub fn shown(gifts: &[Gift], kind: u8, index: u8) -> Option<usize> {
    let bag = InventoryType::from_i16(i16::from(kind))?;
    gifts
        .iter()
        .enumerate()
        .filter(|(_, gift)| gift_bag(&gift.item) == Some(bag))
        .take(usize::from(u8::MAX))
        .nth(usize::from(index))
        .map(|(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;

    fn gift(id: i64, item_id: i32) -> Gift {
        Gift {
            id,
            item: Item::stack(item_id, 1),
        }
    }

    #[test]
    fn a_take_names_a_row_of_its_bag() {
        let gifts = [
            gift(1, 4000000),
            gift(2, RED_POTION),
            gift(3, 4000001),
            gift(4, 4000002),
        ];
        let etc = InventoryType::Etc as u8;
        assert_eq!(shown(&gifts, etc, 0), Some(0));
        assert_eq!(shown(&gifts, etc, 1), Some(2));
        assert_eq!(shown(&gifts, etc, 2), Some(3));
        assert_eq!(shown(&gifts, etc, 3), None);
        assert_eq!(shown(&gifts, InventoryType::Use as u8, 0), Some(1));
        assert_eq!(shown(&gifts, InventoryType::Equip as u8, 0), None);
        assert_eq!(shown(&gifts, 0, 0), None);
        assert_eq!(shown(&gifts, 9, 0), None);
    }

    #[test]
    fn a_gift_is_what_the_window_named() {
        if items::data().is_err() {
            return;
        }
        let mut giver = crate::codec::stats::sample_player();
        items::give(giver.inventory_mut(), RED_POTION, 30).unwrap();
        assert!(give(&mut giver, 1, RED_POTION, 31).unwrap().is_none());
        assert!(give(&mut giver, 1, RED_POTION, 0).unwrap().is_none());
        assert!(
            give(&mut giver, 1, 2000001, 1).unwrap().is_none(),
            "another item"
        );
        assert!(
            give(&mut giver, 2, RED_POTION, 1).unwrap().is_none(),
            "an empty slot"
        );
        assert_eq!(giver.inventory.count(RED_POTION), 30);
        let (gift, _) = give(&mut giver, 1, RED_POTION, 10).unwrap().unwrap();
        assert_eq!(gift.quantity, 10);
        assert_eq!(giver.inventory.count(RED_POTION), 20);

        // Stars go whole.
        items::give(giver.inventory_mut(), SUBI, 500).unwrap();
        let (gift, _) = give(&mut giver, 2, SUBI, 1).unwrap().unwrap();
        assert_eq!(gift.quantity, 500);
        assert_eq!(giver.inventory.count(SUBI), 0);

        let mut receiver = crate::codec::stats::sample_player();
        assert!(receive(&mut receiver, &gift).unwrap().is_some());
        assert_eq!(receiver.inventory.count(SUBI), 500);
    }

    #[test]
    fn cash_items_are_no_gifts() {
        if items::data().is_err() {
            return;
        }
        let mut giver = crate::codec::stats::sample_player();
        let cash = Item {
            cash_serial: Some(1),
            ..Item::stack(5000000, 1)
        };
        giver.inventory_mut().insert(InventoryType::Cash, 1, cash);
        assert!(give(&mut giver, 1, 5000000, 1).unwrap().is_none());
        assert_eq!(giver.inventory.count(5000000), 1);
    }
}
