//! The Item Tag (5060000) and the Magical Sandglasses (550xxxx), which the
//! client sends through USE_CASH_ITEM once one is dropped on a worn equip
//! (v83 FUN_004f4bb7, report c009). The request ends with the equip's slot.
//! The tag prints its user's name on the equip, and a sandglass adds to the
//! time a time-limited equip lasts.

use crate::handler::{HandlerResult, WorldContext, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::build_stat_update;
use game::inventory::FLAG_LOCK;
use game::{InventoryChange, InventoryType, Item, Player};
use game_data::items::Sandglass;
use packet::io::read::PktRead;
use std::io::Cursor;
use std::time::{Duration, SystemTime};

/// Print the user's name on the worn equip the request names, unless it
/// has one already.
pub fn item_tag(ctx: &mut WorldContext, r: &mut Cursor<&Vec<u8>>, slot: i16) -> WorldResult {
    let target = r.read_short()?;
    let name = ctx.player.name().to_string();
    let changes = restamp_worn(ctx.player, slot, target, |item| {
        if !item.owner.is_empty() {
            return false;
        }
        item.owner = name;
        true
    });
    answer(changes)
}

/// Add the sandglass `item_id`'s time to the worn equip the request names.
pub fn sandglass(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let target = r.read_short()?;
    let Some(sandglass) = items::info(item_id).and_then(|info| info.sandglass) else {
        return answer(None);
    };
    let now = SystemTime::now();
    let changes = restamp_worn(ctx.player, slot, target, |item| {
        extend(item, sandglass, now)
    });
    answer(changes)
}

/// v83's checks (0x4f5080-0x4f51c6): only a time-limited equip that isn't
/// a cash item, sealed or `notExtend`, and not past `maxDays` from now.
fn extend(item: &mut Item, sandglass: Sandglass, now: SystemTime) -> bool {
    let Some(expiration) = item.expiration else {
        return false;
    };
    let not_extend = items::info(item.item_id).is_some_and(|info| info.not_extend);
    if item.is_cash() || item.flags & FLAG_LOCK != 0 || not_extend {
        return false;
    }
    let extended = expiration + Duration::from_secs(sandglass.add_seconds.max(0) as u64);
    let latest = now + Duration::from_secs(sandglass.max_days.max(0) as u64 * 86_400);
    if sandglass.max_days > 0 && extended > latest {
        return false;
    }
    item.expiration = Some(extended);
    true
}

/// [`restamp`] on the worn equip in `target`.
fn restamp_worn(
    player: &mut Player,
    slot: i16,
    target: i16,
    change: impl FnOnce(&mut Item) -> bool,
) -> Option<Vec<InventoryChange>> {
    if target >= 0 {
        return None;
    }
    restamp(player, slot, InventoryType::Equipped, target, change)
}

/// Spend the cash item in Cash `slot` on the item in `kind`'s `target`, if
/// `change` takes: the changes that put the item back changed.
pub fn restamp(
    player: &mut Player,
    slot: i16,
    kind: InventoryType,
    target: i16,
    change: impl FnOnce(&mut Item) -> bool,
) -> Option<Vec<InventoryChange>> {
    let mut item = player.inventory.get(kind, target)?.clone();
    if !change(&mut item) {
        return None;
    }
    let mut next = player.inventory.clone();
    let mut changes = next.take(InventoryType::Cash, slot, 1).ok()?;
    changes.extend(next.take(kind, target, item.quantity).ok()?);
    next.insert(kind, target, item.clone());
    changes.push(InventoryChange::Add {
        kind,
        slot: target,
        item,
    });
    *player.inventory_mut() = next;
    Some(changes)
}

/// The changes, or just the unlock the client waits for.
pub fn answer(changes: Option<Vec<InventoryChange>>) -> WorldResult {
    Ok(HandlerResult::reply(match changes {
        Some(changes) => build_inventory_operation(true, &changes)?,
        None => build_stat_update(true, &[])?,
    }))
}

#[cfg(test)]
mod tests {
    use crate::handler::{WorldContext, WorldHandler};
    use crate::packet::handle::world::UseCashItemHandler;
    use crate::packet::op::RecvOpcode;
    use game::inventory::{equip_slot, FLAG_LOCK};
    use game::{EquipStats, InventoryType, Item, Player};
    use packet::{io::write::PktWrite, Packet};
    use std::time::{Duration, SystemTime};

    const TAG: i32 = 5_060_000;
    /// [7days]Magical Sandglass: a week, at most 30 days from now.
    const WEEK: i32 = 5_500_001;
    const SWORD: i32 = 1_302_000;
    const DAY: Duration = Duration::from_secs(86_400);

    fn use_on(player: &mut Player, item_id: i32, target: i16) {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
        p.write_short(1).unwrap();
        p.write_int(item_id).unwrap();
        p.write_short(target).unwrap();
        p.write_int(1234).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 2,
            player,
        };
        UseCashItemHandler.handle(&mut p, &mut ctx).unwrap();
    }

    fn wearing(item_id: i32, expiration: Option<SystemTime>, cash_item: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        let mut sword = Item::equipment(item_id, EquipStats::default());
        sword.expiration = expiration;
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::WEAPON, sword);
        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(cash_item, 1));
        player
    }

    fn worn(player: &Player) -> &Item {
        player
            .inventory
            .get(InventoryType::Equipped, equip_slot::WEAPON)
            .unwrap()
    }

    fn spent(player: &Player) -> bool {
        player.inventory.get(InventoryType::Cash, 1).is_none()
    }

    #[test]
    fn the_tag_prints_the_name_once() {
        let mut player = wearing(SWORD, None, TAG);
        use_on(&mut player, TAG, equip_slot::WEAPON);
        assert_eq!(worn(&player).owner, player.name());
        assert!(spent(&player));

        player
            .inventory
            .insert(InventoryType::Cash, 1, Item::stack(TAG, 1));
        use_on(&mut player, TAG, equip_slot::WEAPON);
        assert!(!spent(&player), "an equip with a name keeps it");
    }

    #[test]
    fn the_tag_needs_a_worn_equip() {
        let mut player = wearing(SWORD, None, TAG);
        use_on(&mut player, TAG, equip_slot::CAP);
        assert!(!spent(&player));
        use_on(&mut player, TAG, 1);
        assert!(!spent(&player));
    }

    #[test]
    fn a_sandglass_adds_its_time() {
        if crate::items::info(WEEK).is_none() {
            return;
        }
        let ends = SystemTime::now() + 10 * DAY;
        let mut player = wearing(SWORD, Some(ends), WEEK);
        use_on(&mut player, WEEK, equip_slot::WEAPON);
        assert_eq!(worn(&player).expiration, Some(ends + 7 * DAY));
        assert!(spent(&player));
    }

    #[test]
    fn a_sandglass_goes_no_further_than_its_days() {
        if crate::items::info(WEEK).is_none() {
            return;
        }
        let ends = SystemTime::now() + 24 * DAY;
        let mut player = wearing(SWORD, Some(ends), WEEK);
        use_on(&mut player, WEEK, equip_slot::WEAPON);
        assert_eq!(worn(&player).expiration, Some(ends));
        assert!(!spent(&player));
    }

    #[test]
    fn a_sandglass_leaves_what_it_cant_extend() {
        if crate::items::info(WEEK).is_none() {
            return;
        }
        let ends = Some(SystemTime::now() + DAY);
        // A permanent equip.
        let mut player = wearing(SWORD, None, WEEK);
        use_on(&mut player, WEEK, equip_slot::WEAPON);
        assert!(!spent(&player));
        // A sealed one.
        let mut player = wearing(SWORD, ends, WEEK);
        let mut sealed = worn(&player).clone();
        sealed.flags |= FLAG_LOCK;
        player
            .inventory
            .insert(InventoryType::Equipped, equip_slot::WEAPON, sealed);
        use_on(&mut player, WEEK, equip_slot::WEAPON);
        assert!(!spent(&player));
        // A medal marked notExtend.
        let mut player = wearing(1_140_000, ends, WEEK);
        use_on(&mut player, WEEK, equip_slot::WEAPON);
        assert!(!spent(&player));
    }
}
