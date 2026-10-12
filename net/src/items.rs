//! Item templates (`Item.nx`, `Character.nx`), loaded once, and helpers that
//! turn them into inventory items.

use crate::load::{self, asset_path};
use game::{EquipStats, Inventory, InventoryChange, InventoryError, Item};
use game_data::items::{EquipInfo, ItemData, ItemInfo};
use std::sync::OnceLock;

pub fn data() -> Result<&'static ItemData, &'static str> {
    static ITEMS: OnceLock<Result<ItemData, String>> = OnceLock::new();
    load::once(&ITEMS, || {
        ItemData::load(
            asset_path("RUSTMS_ITEM_NX_PATH", "assets/game-data/Item.nx"),
            asset_path("RUSTMS_CHARACTER_NX_PATH", "assets/game-data/Character.nx"),
        )
    })
}

/// The template for `item_id`, if the data has it.
pub fn info(item_id: i32) -> Option<ItemInfo> {
    data().ok()?.info(item_id).ok().flatten()
}

/// How many of `item_id` fit in one slot.
pub fn slot_max(item_id: i32) -> i16 {
    info(item_id).map_or(game_data::items::DEFAULT_SLOT_MAX, |info| info.slot_max)
}

/// A new item of `item_id`: equipment with its base stats, or a stack.
pub fn create(item_id: i32, quantity: i16) -> Option<Item> {
    let info = info(item_id)?;
    Some(match info.equip {
        Some(equip) => Item::equipment(item_id, base_stats(&equip)),
        None => Item::stack(item_id, quantity),
    })
}

/// Put `quantity` of `item_id` into `inventory`, one slot per piece of
/// equipment. Nothing changes unless all of it fits.
pub fn give(
    inventory: &mut Inventory,
    item_id: i32,
    quantity: i16,
) -> Result<Vec<InventoryChange>, InventoryError> {
    let item = create(item_id, quantity).ok_or(InventoryError::UnknownItem)?;
    if item.equip.is_none() {
        return inventory.add(item, slot_max(item_id));
    }
    let mut next = inventory.clone();
    let mut changes = Vec::new();
    for _ in 0..quantity.max(1) {
        changes.extend(next.add(item.clone(), 1)?);
    }
    *inventory = next;
    Ok(changes)
}

pub fn base_stats(equip: &EquipInfo) -> EquipStats {
    EquipStats {
        upgrade_slots: equip.upgrade_slots,
        str: equip.str,
        dex: equip.dex,
        int: equip.int,
        luk: equip.luk,
        hp: equip.hp,
        mp: equip.mp,
        watk: equip.watk,
        matk: equip.matk,
        wdef: equip.wdef,
        mdef: equip.mdef,
        acc: equip.acc,
        avoid: equip.avoid,
        hands: equip.hands,
        speed: equip.speed,
        jump: equip.jump,
        ..EquipStats::default()
    }
}

/// Whether `player` meets an equipment's level, stat, and job requirements.
pub fn can_wear(equip: &EquipInfo, player: &game::Player) -> bool {
    let stats = &player.stats;
    let branch = match stats.job / 100 {
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 8,
        5 => 16,
        _ => 0,
    };
    let job_ok = equip.req_job == 0 || (branch != 0 && equip.req_job & branch != 0);
    job_ok
        && i16::from(stats.level) >= equip.req_level
        && stats.str >= equip.req_str
        && stats.dex >= equip.req_dex
        && stats.int >= equip.req_int
        && stats.luk >= equip.req_luk
}
