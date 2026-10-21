//! NPC shops: what a shop lists, and what buying, selling, and recharging
//! (NPC_SHOP_ACTION) do to the player. The lists come from the content pack
//! (`content::shops`); prices to sell at and recharge rates from the item
//! data.

use crate::error::NetworkError;
use crate::packet::build::world::{
    inventory,
    shop::build_shop_result,
    stat::{self, StatUpdate},
};
use crate::{content, items};
use game::{InventoryChange, InventoryType, Player};
use packet::Packet;

/// Claw Mastery and Gun Mastery: each level adds 10 to a set of throwing
/// stars or bullets.
const CLAW_MASTERY: i32 = 4100000;
const GUN_MASTERY: i32 = 5200000;

/// CONFIRM_SHOP_TRANSACTION codes.
pub mod result {
    pub const BOUGHT: u8 = 0;
    pub const NOT_ENOUGH_MESOS: u8 = 2;
    pub const INVENTORY_FULL: u8 = 3;
    pub const SOLD: u8 = 8;
    pub const RECHARGED: u8 = 8;
}

/// What the client asked of the open shop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopRequest {
    /// `index` is the item's row in the shop's list.
    Buy {
        index: i16,
        item_id: i32,
        quantity: i16,
    },
    /// `slot` is in the bag the item belongs in.
    Sell {
        slot: i16,
        item_id: i32,
        quantity: i16,
    },
    /// Refill the set of stars or bullets in this USE slot.
    Recharge {
        slot: i16,
    },
    Leave,
}

/// Why a shop refused. The client is told about mesos and space; anything
/// else is a request the client should never have made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopError {
    NotEnoughMesos,
    InventoryFull,
    Invalid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopItem {
    pub item_id: i32,
    /// 0 for stars and bullets that are only recharged here.
    pub price: i32,
    /// The most bought at once: a stack, or one set of stars or bullets.
    pub slot_max: i16,
    /// What one star or bullet costs to recharge; 0 for anything else.
    pub unit_price: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shop {
    pub npc_id: i32,
    pub items: Vec<ShopItem>,
}

/// Throwing stars and bullets come in sets that are recharged, not stacked.
pub fn is_rechargeable(item_id: i32) -> bool {
    matches!(item_id / 10_000, 207 | 233)
}

/// How many stars or bullets make a full set for `player`; the stack size
/// for anything else.
pub fn set_size(player: &Player, item_id: i32) -> i16 {
    let mastery = match item_id / 10_000 {
        207 => player.skills.level(CLAW_MASTERY),
        233 => player.skills.level(GUN_MASTERY),
        _ => 0,
    };
    let bonus = i16::try_from(mastery * 10).unwrap_or(0);
    items::slot_max(item_id).saturating_add(bonus)
}

impl Shop {
    /// The shop `npc_id` keeps, as `player` sees it. Items our data doesn't
    /// have, and price-less entries that aren't stars or bullets, are left
    /// out.
    pub fn open(npc_id: i32, player: &Player) -> Option<Self> {
        let entries = content::shops().shop(npc_id)?;
        let items = entries
            .iter()
            .filter(|entry| entry.price > 0 || is_rechargeable(entry.item))
            .filter_map(|entry| {
                let info = items::info(entry.item)?;
                Some(ShopItem {
                    item_id: entry.item,
                    price: entry.price,
                    slot_max: set_size(player, entry.item),
                    unit_price: info.unit_price,
                })
            })
            .collect();
        Some(Self { npc_id, items })
    }

    /// Buy `quantity` of the item in row `index`; stars and bullets come as
    /// one full set for the listed price.
    pub fn buy(
        &self,
        player: &mut Player,
        index: i16,
        item_id: i32,
        quantity: i16,
    ) -> Result<Vec<InventoryChange>, ShopError> {
        let entry = usize::try_from(index)
            .ok()
            .and_then(|index| self.items.get(index))
            .filter(|entry| entry.item_id == item_id && entry.price > 0)
            .ok_or(ShopError::Invalid)?;
        let rechargeable = is_rechargeable(item_id);
        if !rechargeable && !(1..=entry.slot_max).contains(&quantity) {
            return Err(ShopError::Invalid);
        }
        let cost = if rechargeable {
            i64::from(entry.price)
        } else {
            i64::from(entry.price) * i64::from(quantity)
        };
        if cost > i64::from(player.meso) {
            return Err(ShopError::NotEnoughMesos);
        }
        // A one-of-a-kind item comes alone, and not to someone who has it;
        // v83's buy result has no code of its own for this (report c005).
        if items::refuses_only(&player.inventory, item_id, quantity.into()) {
            return Err(ShopError::InventoryFull);
        }
        let changes = if rechargeable {
            let set =
                items::create(item_id, set_size(player, item_id)).ok_or(ShopError::Invalid)?;
            player.inventory_mut().add_alone(set)
        } else {
            items::give(player.inventory_mut(), item_id, quantity)
        }
        .map_err(|_| ShopError::InventoryFull)?;
        player.meso -= cost as i32;
        player.mark_dirty();
        Ok(changes)
    }

    /// Refill the set of stars or bullets in USE `slot`, if this shop lists
    /// them, for `unit_price` each.
    pub fn recharge(
        &self,
        player: &mut Player,
        slot: i16,
    ) -> Result<Vec<InventoryChange>, ShopError> {
        let item = player
            .inventory
            .get(InventoryType::Use, slot)
            .ok_or(ShopError::Invalid)?;
        let (item_id, quantity) = (item.item_id, item.quantity);
        let entry = self
            .items
            .iter()
            .find(|entry| entry.item_id == item_id && is_rechargeable(item_id))
            .ok_or(ShopError::Invalid)?;
        let full = set_size(player, item_id);
        if quantity >= full {
            return Err(ShopError::Invalid);
        }
        let cost = (entry.unit_price * f64::from(full - quantity)).ceil() as i64;
        if cost > i64::from(player.meso) {
            return Err(ShopError::NotEnoughMesos);
        }
        let changes = player
            .inventory_mut()
            .refill(InventoryType::Use, slot, full)
            .map_err(|_| ShopError::Invalid)?;
        player.meso -= cost as i32;
        player.mark_dirty();
        Ok(changes)
    }
}

/// Sell `quantity` of the item in `slot` of its bag at the item's `price`
/// each. Stars and bullets go as the whole set, for their price per full set.
pub fn sell(
    player: &mut Player,
    slot: i16,
    item_id: i32,
    quantity: i16,
) -> Result<Vec<InventoryChange>, ShopError> {
    let kind = InventoryType::for_item(item_id).ok_or(ShopError::Invalid)?;
    let item = player
        .inventory
        .get(kind, slot)
        .filter(|item| item.item_id == item_id && !item.is_cash())
        .ok_or(ShopError::Invalid)?;
    let info = items::info(item_id).ok_or(ShopError::Invalid)?;
    let (quantity, mesos) = if is_rechargeable(item_id) {
        let per_star = f64::from(info.price) / f64::from(info.slot_max.max(1));
        let whole = item.quantity;
        (whole, (per_star * f64::from(whole)).ceil() as i64)
    } else {
        if !(1..=item.quantity).contains(&quantity) {
            return Err(ShopError::Invalid);
        }
        (quantity, i64::from(info.price) * i64::from(quantity))
    };
    let meso = i32::try_from(i64::from(player.meso) + mesos).map_err(|_| ShopError::Invalid)?;
    let changes = player
        .inventory_mut()
        .take(kind, slot, quantity)
        .map_err(|_| ShopError::Invalid)?;
    player.meso = meso;
    player.mark_dirty();
    Ok(changes)
}

/// The replies to a buy, sell, or recharge at `shop`: the item and meso
/// changes and how it went. A request the client should never have made
/// only unlocks its item actions.
pub fn respond(
    shop: &Shop,
    player: &mut Player,
    request: ShopRequest,
) -> Result<Vec<Packet>, NetworkError> {
    let (outcome, done) = match request {
        ShopRequest::Buy {
            index,
            item_id,
            quantity,
        } => (shop.buy(player, index, item_id, quantity), result::BOUGHT),
        ShopRequest::Sell {
            slot,
            item_id,
            quantity,
        } => (sell(player, slot, item_id, quantity), result::SOLD),
        ShopRequest::Recharge { slot } => (shop.recharge(player, slot), result::RECHARGED),
        ShopRequest::Leave => return Ok(Vec::new()),
    };
    let code = match outcome {
        Ok(changes) => {
            return Ok(vec![
                inventory::build_inventory_operation(true, &changes)?,
                stat::build_stat_update(true, &[StatUpdate::Meso(player.meso)])?,
                build_shop_result(done)?,
            ])
        }
        Err(ShopError::NotEnoughMesos) => result::NOT_ENOUGH_MESOS,
        Err(ShopError::InventoryFull) => result::INVENTORY_FULL,
        Err(ShopError::Invalid) => return Ok(vec![stat::build_stat_update(true, &[])?]),
    };
    Ok(vec![build_shop_result(code)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Item;

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;
    const SWORD: i32 = 1302000;

    /// A grocer with Red Potions, Subi Throwing-Stars, and a sword, plus
    /// Wolbi Throwing-Stars to recharge only.
    fn shop() -> Shop {
        let row = |item_id, price, slot_max, unit_price| ShopItem {
            item_id,
            price,
            slot_max,
            unit_price,
        };
        Shop {
            npc_id: 1011100,
            items: vec![
                row(RED_POTION, 50, 100, 0.0),
                row(SUBI, 500, 500, 0.3),
                row(SWORD, 50, 1, 0.0),
                row(2070001, 0, 500, 0.4),
            ],
        }
    }

    fn player(meso: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player
    }

    #[test]
    fn buying_takes_the_price_and_fills_the_bag() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(1000);
        let changes = shop().buy(&mut player, 0, RED_POTION, 3).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(player.inventory.count(RED_POTION), 3);
        assert_eq!(player.meso, 850);

        // Equipment comes with its stats; stars come as one full set.
        shop().buy(&mut player, 2, SWORD, 1).unwrap();
        let (_, sword) = player.inventory.items(InventoryType::Equip).next().unwrap();
        assert!(sword.equip.as_ref().unwrap().watk > 0);
        shop().buy(&mut player, 1, SUBI, 1).unwrap();
        assert_eq!(player.inventory.count(SUBI), 500);
        assert_eq!(player.meso, 300);
    }

    #[test]
    fn buying_needs_the_mesos_and_the_space() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(100);
        assert_eq!(
            shop().buy(&mut player, 0, RED_POTION, 3),
            Err(ShopError::NotEnoughMesos)
        );
        player.meso = 10_000;
        player.inventory.set_slot_limit(InventoryType::Use, 0);
        let before = player.inventory.clone();
        assert_eq!(
            shop().buy(&mut player, 0, RED_POTION, 1),
            Err(ShopError::InventoryFull)
        );
        assert_eq!(
            shop().buy(&mut player, 1, SUBI, 1),
            Err(ShopError::InventoryFull)
        );
        assert_eq!((player.meso, &player.inventory), (10_000, &before));
    }

    #[test]
    fn a_one_of_a_kind_item_is_sold_once() {
        if items::data().is_err() {
            return;
        }
        let only = (4031000..4032000)
            .find(|&id| items::info(id).is_some_and(|info| info.only))
            .unwrap();
        let shop = Shop {
            npc_id: 1011100,
            items: vec![ShopItem {
                item_id: only,
                price: 10,
                slot_max: 100,
                unit_price: 0.0,
            }],
        };
        let mut player = player(1000);
        assert_eq!(
            shop.buy(&mut player, 0, only, 2),
            Err(ShopError::InventoryFull)
        );
        shop.buy(&mut player, 0, only, 1).unwrap();
        assert_eq!(
            shop.buy(&mut player, 0, only, 1),
            Err(ShopError::InventoryFull)
        );
        assert_eq!((player.inventory.count(only), player.meso), (1, 990));
    }

    #[test]
    fn buying_what_the_shop_does_not_sell_is_refused() {
        let mut player = player(100_000);
        for (index, item_id, quantity) in [
            (0, SUBI, 1),         // the wrong row
            (9, RED_POTION, 1),   // no such row
            (-1, RED_POTION, 1),  // no such row
            (3, 2070001, 1),      // recharge only
            (0, RED_POTION, 0),   // nothing
            (0, RED_POTION, 101), // more than a stack
            (2, SWORD, 2),        // more than one sword
        ] {
            assert_eq!(
                shop().buy(&mut player, index, item_id, quantity),
                Err(ShopError::Invalid),
                "{index} {item_id} {quantity}"
            );
        }
        assert_eq!(player.meso, 100_000);
    }

    #[test]
    fn selling_pays_the_item_price() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        player
            .inventory
            .insert(InventoryType::Use, 4, Item::stack(RED_POTION, 10));
        let price = items::info(RED_POTION).unwrap().price;
        sell(&mut player, 4, RED_POTION, 4).unwrap();
        assert_eq!(player.inventory.count(RED_POTION), 6);
        assert_eq!(player.meso, price * 4);

        assert_eq!(sell(&mut player, 4, RED_POTION, 7), Err(ShopError::Invalid));
        assert_eq!(sell(&mut player, 3, RED_POTION, 1), Err(ShopError::Invalid));
        assert_eq!(sell(&mut player, 4, SUBI, 1), Err(ShopError::Invalid));

        // A partial set of stars goes whole, for its share of a set's price.
        player
            .inventory
            .insert(InventoryType::Use, 5, Item::stack(SUBI, 100));
        let before = player.meso;
        sell(&mut player, 5, SUBI, 1).unwrap();
        assert_eq!(player.inventory.count(SUBI), 0);
        assert_eq!(player.meso - before, 250 * 100 / 500);
    }

    #[test]
    fn selling_never_overflows_the_mesos() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(i32::MAX);
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(RED_POTION, 1));
        assert_eq!(sell(&mut player, 1, RED_POTION, 1), Err(ShopError::Invalid));
        assert_eq!(player.inventory.count(RED_POTION), 1);
    }

    #[test]
    fn recharging_refills_a_set_at_its_unit_price() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(1000);
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(SUBI, 100));
        shop().recharge(&mut player, 2).unwrap();
        assert_eq!(player.inventory.count(SUBI), 500);
        assert_eq!(player.meso, 1000 - 120);

        // Already full, not stars, or not enough mesos.
        assert_eq!(shop().recharge(&mut player, 2), Err(ShopError::Invalid));
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(RED_POTION, 1));
        assert_eq!(shop().recharge(&mut player, 3), Err(ShopError::Invalid));
        player
            .inventory
            .insert(InventoryType::Use, 4, Item::stack(2070001, 1));
        player.meso = 10;
        assert_eq!(
            shop().recharge(&mut player, 4),
            Err(ShopError::NotEnoughMesos)
        );
        assert_eq!(player.inventory.count(2070001), 1);
    }

    #[test]
    fn claw_mastery_makes_bigger_sets() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(0);
        assert_eq!(set_size(&player, SUBI), 500);
        player.skills = game::SkillBook::from_records([game::SkillRecord {
            skill_id: CLAW_MASTERY,
            level: 3,
            master_level: 0,
            expiration: None,
        }]);
        assert_eq!(set_size(&player, SUBI), 530);
        assert_eq!(set_size(&player, RED_POTION), 100);
    }
}
