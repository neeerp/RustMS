//! Cash Shop coupons (report b072): the Coupon dialog sends a code; one
//! account may redeem it once for what it gives. Cash items go to the
//! Cash Inventory and NX to the balances, both kept by the account; items
//! for the bags and mesos go to the character.

use crate::error::NetworkError;
use crate::items;
use crate::packet::build::world::cash_shop::CashShopFailure;
use crate::packet::build::world::{
    inventory,
    stat::{self, StatUpdate},
};
use db::cash_shop::Balance;
use db::coupon::Reward;
use game::Player;
use packet::Packet;

/// The longest code kept.
pub const CODE_LEN: usize = 32;
/// Wrong codes in a row on one connection before the shop sends the
/// player away (v83 leaves on 0xB1 itself).
pub const STRIKES: u8 = 3;

/// The code as kept: without the spaces around it, in upper case. `None`
/// if nothing is left or it is too long to be one.
pub fn normalize(code: &str) -> Option<String> {
    let code = code.trim().to_uppercase();
    (!code.is_empty() && code.len() <= CODE_LEN).then_some(code)
}

/// Whether `item_id` belongs in the Cash Inventory rather than a bag.
pub fn for_cash_inventory(item_id: i32) -> bool {
    item_id / 1_000_000 == 5
        || items::info(item_id).is_some_and(|info| info.equip.is_some_and(|equip| equip.cash))
}

/// What a coupon gives, sorted for where it goes.
#[derive(Debug, Default)]
pub struct Grant {
    /// For the Cash Inventory: item id, commodity serial (0 if none sells
    /// it) and quantity.
    pub cash_items: Vec<(i32, i32, i16)>,
    /// Put in the bags: item id and quantity.
    pub items: Vec<(i32, i16)>,
    pub mesos: i32,
    /// Added to the account's balances.
    pub add: Balance,
    /// What tells the client of the bag items and mesos.
    pub packets: Vec<Packet>,
}

/// Give `player` the bag items and mesos of `rewards`, and sort out the
/// rest for the account. Nothing changes unless all of it fits.
pub fn grant(
    player: &mut Player,
    rewards: &[Reward],
) -> Result<Result<Grant, CashShopFailure>, NetworkError> {
    let mut next = player.clone();
    let mut grant = Grant::default();
    let mut changes = Vec::new();
    for &reward in rewards {
        let total = match reward {
            Reward::Item { item_id, quantity } => {
                let Ok(quantity) = i16::try_from(quantity) else {
                    return Ok(Err(CashShopFailure::Unknown));
                };
                if items::info(item_id).is_none() {
                    return Ok(Err(CashShopFailure::Unknown));
                }
                if for_cash_inventory(item_id) {
                    let sn = crate::cash_shop::commodity_sn(item_id, quantity);
                    grant.cash_items.push((item_id, sn, quantity));
                    continue;
                }
                if items::refuses_only(&next.inventory, item_id, quantity.into()) {
                    return Ok(Err(CashShopFailure::InventoryFull));
                }
                match items::give(next.inventory_mut(), item_id, quantity) {
                    Ok(given) => changes.extend(given),
                    Err(_) => return Ok(Err(CashShopFailure::InventoryFull)),
                }
                grant.items.push((item_id, quantity));
                continue;
            }
            Reward::Mesos(amount) => {
                let Some(meso) = next.meso.checked_add(amount) else {
                    return Ok(Err(CashShopFailure::InventoryFull));
                };
                next.meso = meso;
                grant.mesos.checked_add(amount).map(|sum| grant.mesos = sum)
            }
            Reward::Credit(amount) => grant
                .add
                .credit
                .checked_add(amount)
                .map(|sum| grant.add.credit = sum),
            Reward::MaplePoints(amount) => grant
                .add
                .points
                .checked_add(amount)
                .map(|sum| grant.add.points = sum),
            Reward::Prepaid(amount) => grant
                .add
                .prepaid
                .checked_add(amount)
                .map(|sum| grant.add.prepaid = sum),
        };
        if total.is_none() {
            return Ok(Err(CashShopFailure::Unknown));
        }
    }
    if !changes.is_empty() {
        grant
            .packets
            .push(inventory::build_inventory_operation(true, &changes)?);
    }
    if grant.mesos > 0 {
        grant.packets.push(stat::build_stat_update(
            true,
            &[StatUpdate::Meso(next.meso)],
        )?);
    }
    if !changes.is_empty() || grant.mesos > 0 {
        next.mark_dirty();
    }
    *player = next;
    Ok(Ok(grant))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use game::{Inventory, InventoryType, Keymap, QuestLog, SkillBook};

    const RED_POTION: i32 = 2000000;
    const SAFETY_CHARM: i32 = 5130000;

    fn player() -> Player {
        Player::new(
            9,
            0,
            sample_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::defaults(),
            QuestLog::default(),
        )
    }

    #[test]
    fn codes_are_trimmed_and_upper_cased() {
        assert_eq!(normalize("  abc123 ").as_deref(), Some("ABC123"));
        assert_eq!(normalize("   "), None);
        assert_eq!(normalize(&"A".repeat(CODE_LEN + 1)), None);
    }

    #[test]
    fn rewards_go_to_the_bags_the_cash_inventory_and_the_account() {
        let mut player = player();
        let meso = player.meso;
        let rewards = [
            Reward::Item {
                item_id: RED_POTION,
                quantity: 20,
            },
            Reward::Item {
                item_id: SAFETY_CHARM,
                quantity: 1,
            },
            Reward::Mesos(1000),
            Reward::Credit(300),
            Reward::MaplePoints(500),
        ];
        let grant = grant(&mut player, &rewards).unwrap().unwrap();
        assert_eq!(grant.items, [(RED_POTION, 20)]);
        assert_eq!(
            grant
                .cash_items
                .iter()
                .map(|&(id, _, q)| (id, q))
                .collect::<Vec<_>>(),
            [(SAFETY_CHARM, 1)]
        );
        assert_eq!(grant.mesos, 1000);
        assert_eq!(
            grant.add,
            Balance {
                credit: 300,
                points: 500,
                prepaid: 0
            }
        );
        assert_eq!(grant.packets.len(), 2);
        assert_eq!(player.inventory.count(RED_POTION), 20);
        assert_eq!(player.meso, meso + 1000);
    }

    #[test]
    fn nothing_changes_unless_everything_fits() {
        let mut player = player();
        let slots = player.inventory.slot_limit(InventoryType::Use);
        let full = [Reward::Item {
            item_id: RED_POTION,
            quantity: 100,
        }; 1]
            .repeat(usize::from(slots) + 1);
        let before = player.clone();
        let rewards: Vec<_> = [Reward::Mesos(1000)].into_iter().chain(full).collect();
        assert_eq!(
            grant(&mut player, &rewards).unwrap().unwrap_err(),
            CashShopFailure::InventoryFull
        );
        assert_eq!(player.meso, before.meso);
        assert_eq!(player.inventory.count(RED_POTION), 0);
    }

    #[test]
    fn unknown_items_and_too_many_mesos_fail() {
        let mut player = player();
        let unknown = [Reward::Item {
            item_id: 1,
            quantity: 1,
        }];
        assert_eq!(
            grant(&mut player, &unknown).unwrap().unwrap_err(),
            CashShopFailure::Unknown
        );
        player.meso = i32::MAX - 5;
        assert_eq!(
            grant(&mut player, &[Reward::Mesos(10)])
                .unwrap()
                .unwrap_err(),
            CashShopFailure::InventoryFull
        );
        assert_eq!(player.meso, i32::MAX - 5);
    }
}
