//! The Item Guards: 5060001 seals an item for good, 5061xxx for its
//! `protectTime` days (v83 CUIItemProtector, report c009). The client sends
//! USE_CASH_ITEM once an item dropped on the guard's window is confirmed,
//! with the item's inventory type and slot. A sealed item can't be sold,
//! traded or dropped; a timed seal's end is the item's date, and the
//! expiry sweep unseals it then.

use super::item_tag::{answer, restamp};
use crate::handler::{WorldContext, WorldResult};
use crate::items;
use game::inventory::FLAG_LOCK;
use game::{InventoryType, Item};
use packet::io::read::PktRead;
use std::io::Cursor;
use std::time::{Duration, SystemTime};

/// The guard that seals for good.
pub const ITEM_GUARD: i32 = 5_060_001;

/// Seal the item the request names with the guard `item_id` in Cash
/// `slot`; `r` is past the item id.
pub fn item_guard(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let tab = r.read_int()?;
    let target = r.read_int()?;
    // Only from the Item window: v83's Equip window doesn't drop on it.
    let kind = i16::try_from(tab)
        .ok()
        .and_then(InventoryType::from_i16)
        .filter(|kind| *kind != InventoryType::Equipped);
    let (Some(kind), Ok(target @ 1..)) = (kind, i16::try_from(target)) else {
        return answer(None);
    };
    let days = if item_id == ITEM_GUARD {
        0
    } else {
        match items::info(item_id).map(|info| info.protect_time) {
            Some(days @ 1..) => days,
            _ => return answer(None),
        }
    };
    let now = SystemTime::now();
    answer(restamp(ctx.player, slot, kind, target, |item| {
        seal(item, kind, days, now)
    }))
}

/// Seal `item`, for `days` from its seal's end or from `now`, or for good
/// with 0 days, if a guard takes it.
fn seal(item: &mut Item, kind: InventoryType, days: i32, now: SystemTime) -> bool {
    if !sealable(item, kind) {
        return false;
    }
    item.expiration = (days > 0).then(|| {
        let from = item.expiration.map_or(now, |end| end.max(now));
        from + Duration::from_secs(days as u64 * 86_400)
    });
    item.flags |= FLAG_LOCK;
    true
}

/// v83's test of an item dropped on the guard's window (0x4f434f-0x4f449e):
/// a timed seal may be extended or made lasting, but a time-limited item,
/// a cash one or one gone at logout can't be sealed. Of the others only
/// equipment, stars, bullets and 22500xx can.
fn sealable(item: &Item, kind: InventoryType) -> bool {
    let timed = item
        .expiration
        .is_some_and(|end| end < game::pet::doll_date());
    if timed != item.sealed() {
        return false;
    }
    let expire_on_logout = items::info(item.item_id).is_some_and(|info| info.expire_on_logout);
    if expire_on_logout || item.is_cash() || item.item_id / 100 == 11_123 {
        return false;
    }
    kind == InventoryType::Equip
        || matches!(item.item_id / 10_000, 207 | 233)
        || item.item_id / 100 == 22_500
}

#[cfg(test)]
mod tests {
    use super::ITEM_GUARD;
    use crate::handler::{WorldContext, WorldHandler};
    use crate::items::expire_items;
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::inventory::equip_slot;
    use game::{EquipStats, InventoryChange, InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};
    use std::time::{Duration, SystemTime};

    /// Item Guard : 7 Days.
    const WEEK: i32 = 5_061_000;
    const SWORD: i32 = 1_302_000;
    const STARS: i32 = 2_070_000;
    const RED_POTION: i32 = 2_000_000;
    const DAY: Duration = Duration::from_secs(86_400);

    fn guard(player: &mut Player, guard_id: i32, tab: i32, target: i32) {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(1).unwrap();
        p.write_int(guard_id).unwrap();
        p.write_int(tab).unwrap();
        p.write_int(target).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap();
    }

    fn holding(kind: InventoryType, item: Item, guard_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(kind, 1, item);
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(guard_id, 1));
        player
    }

    fn sword() -> Item {
        Item::equipment(SWORD, EquipStats::default())
    }

    fn spent(player: &Player) -> bool {
        player.inventory.get(InventoryType::Cash, 1).is_none()
    }

    fn held(player: &Player, kind: InventoryType) -> &Item {
        player.inventory.get(kind, 1).unwrap()
    }

    #[test]
    fn the_guard_seals_for_good() {
        let mut player = holding(InventoryType::Equip, sword(), ITEM_GUARD);
        guard(&mut player, ITEM_GUARD, 1, 1);
        let sealed = held(&player, InventoryType::Equip);
        assert!(sealed.sealed() && sealed.expiration.is_none());
        assert!(spent(&player));
    }

    #[test]
    fn the_guard_seals_stars_but_not_worn_equips() {
        let mut player = holding(InventoryType::Use, Item::stack(STARS, 500), ITEM_GUARD);
        guard(&mut player, ITEM_GUARD, 2, 1);
        let stars = held(&player, InventoryType::Use);
        assert!(stars.sealed() && stars.quantity == 500);

        let mut player = holding(InventoryType::Etc, Item::stack(4_000_000, 1), ITEM_GUARD);
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::WEAPON, sword());
        guard(&mut player, ITEM_GUARD, 1, i32::from(equip_slot::WEAPON));
        guard(&mut player, ITEM_GUARD, -1, i32::from(equip_slot::WEAPON));
        assert!(!spent(&player));
    }

    #[test]
    fn the_guard_leaves_what_it_cant_seal() {
        let refused = |kind, item: Item, tab| {
            let mut player = holding(kind, item, ITEM_GUARD);
            guard(&mut player, ITEM_GUARD, tab, 1);
            assert!(!spent(&player));
            assert!(!held(&player, kind).sealed());
        };
        refused(InventoryType::Use, Item::stack(RED_POTION, 5), 2);
        let mut timed = sword();
        timed.expiration = Some(SystemTime::now() + DAY);
        refused(InventoryType::Equip, timed, 1);
        let mut cash = sword();
        cash.cash_serial = Some(7);
        refused(InventoryType::Equip, cash, 1);
        // The tab must be the item's.
        let mut player = holding(InventoryType::Equip, sword(), ITEM_GUARD);
        guard(&mut player, ITEM_GUARD, 2, 1);
        assert!(!spent(&player));
    }

    #[test]
    fn a_lasting_seal_takes_no_more() {
        let mut player = holding(InventoryType::Equip, sword(), ITEM_GUARD);
        guard(&mut player, ITEM_GUARD, 1, 1);
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(ITEM_GUARD, 1));
        guard(&mut player, ITEM_GUARD, 1, 1);
        assert!(!spent(&player));
    }

    #[test]
    fn a_timed_guard_seals_for_its_days_and_extends() {
        if crate::items::info(WEEK).is_none() {
            return;
        }
        let before = SystemTime::now();
        let mut player = holding(InventoryType::Equip, sword(), WEEK);
        guard(&mut player, WEEK, 1, 1);
        let end = held(&player, InventoryType::Equip).expiration.unwrap();
        assert!(end >= before + 7 * DAY && end <= SystemTime::now() + 7 * DAY);
        assert!(held(&player, InventoryType::Equip).sealed());

        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(WEEK, 1));
        guard(&mut player, WEEK, 1, 1);
        assert_eq!(
            held(&player, InventoryType::Equip).expiration,
            Some(end + 7 * DAY)
        );
    }

    #[test]
    fn a_timed_seal_runs_out_and_the_item_stays() {
        let mut player = crate::codec::stats::sample_player();
        let mut sealed = sword();
        sealed.flags |= game::inventory::FLAG_LOCK | game::inventory::FLAG_UNTRADEABLE;
        sealed.expiration = Some(SystemTime::now() - DAY);
        player
            .inventory
            .insert(InventoryType::Equip, 1, sealed.clone());
        let expired = expire_items(&mut player, SystemTime::now());
        assert_eq!(expired.released, vec![SWORD]);
        assert!(expired.item_ids.is_empty() && !expired.worn);
        let released = held(&player, InventoryType::Equip);
        assert!(!released.sealed() && released.expiration.is_none());
        assert_eq!(released.flags, game::inventory::FLAG_UNTRADEABLE);
        assert!(matches!(
            expired.changes.last(),
            Some(InventoryChange::Add { slot: 1, .. })
        ));
    }
}
