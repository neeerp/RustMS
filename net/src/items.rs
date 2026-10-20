//! Item templates (`Item.nx`, `Character.nx`), loaded once, and helpers that
//! turn them into inventory items.

use crate::load::{self, asset_path};
use game::{EquipStats, Inventory, InventoryChange, InventoryError, Item};
use game_data::items::{EquipInfo, ItemData, ItemInfo, ItemNames};
use rand::Rng;
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

fn names() -> Result<&'static ItemNames, &'static str> {
    static NAMES: OnceLock<Result<ItemNames, String>> = OnceLock::new();
    load::once(&NAMES, || {
        ItemNames::load(asset_path(
            "RUSTMS_STRING_NX_PATH",
            "assets/game-data/String.nx",
        ))
    })
}

/// A new pet's name: the name of its kind, cut to the 12 bytes a pet's name
/// has room for before its terminator.
pub fn pet_name(item_id: i32) -> String {
    let mut name = names()
        .ok()
        .and_then(|names| names.pet(item_id).ok().flatten())
        .unwrap_or_default();
    while name.len() > 12 {
        name.pop();
    }
    name
}

/// A cash item's message, if it has one, such as a weather item's
/// `%s's snow : %s`.
pub fn cash_message(item_id: i32) -> Option<String> {
    names().ok()?.cash_message(item_id).ok().flatten()
}

/// The template for `item_id`, if the data has it.
pub fn info(item_id: i32) -> Option<ItemInfo> {
    data().ok()?.info(item_id).ok().flatten()
}

/// Whether the hair or face `style` exists. Without the data, every one
/// does.
pub fn has_style(style: i32) -> bool {
    data().map_or(true, |data| data.has_style(style).unwrap_or(false))
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

/// Put a mob's or reactor's drop of `item_id` into `inventory`: a piece of
/// equipment with its stats varied ([`vary`]), or `quantity` of anything
/// else.
pub fn give_dropped(
    inventory: &mut Inventory,
    item_id: i32,
    quantity: i16,
    rng: &mut impl Rng,
) -> Result<Vec<InventoryChange>, InventoryError> {
    let mut item = create(item_id, quantity).ok_or(InventoryError::UnknownItem)?;
    let Some(stats) = item.equip.as_mut() else {
        return inventory.add(item, slot_max(item_id));
    };
    vary(stats, rng);
    inventory.add(item, 1)
}

/// Vary each stat a piece of equipment has by up to a tenth of it either
/// way, at most 5, or 10 for defense, HP and MP (Cosmic's
/// `randomizeStats`).
fn vary(stats: &mut EquipStats, rng: &mut impl Rng) {
    let mut vary = |stat: &mut i16, most: i16| {
        if *stat > 0 {
            let range = ((*stat - 1) / 10 + 1).min(most);
            *stat = stat.saturating_add(rng.gen_range(-range, range + 1));
        }
    };
    for stat in [
        &mut stats.str,
        &mut stats.dex,
        &mut stats.int,
        &mut stats.luk,
        &mut stats.watk,
        &mut stats.matk,
        &mut stats.acc,
        &mut stats.avoid,
        &mut stats.speed,
        &mut stats.jump,
    ] {
        vary(stat, 5);
    }
    for stat in [
        &mut stats.wdef,
        &mut stats.mdef,
        &mut stats.hp,
        &mut stats.mp,
    ] {
        vary(stat, 10);
    }
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

/// Whether `player` meets an equipment's requirements, counting what they
/// wear (v83 FUN_004f2cee).
pub fn can_wear(equip: &EquipInfo, player: &game::Player) -> bool {
    let stats = &player.stats;
    // GMs wear anything.
    let branch = stats.job % 1000 / 100;
    if branch == 8 || branch == 9 {
        return true;
    }
    let mask = if branch == 0 { 0 } else { 1 << (branch - 1) };
    let job_ok = match equip.req_job {
        0 => true,
        -1 => mask == 0,
        req => req > 0 && req & mask != 0,
    };
    let (str, dex, luk) = crate::skills::total_stats(player);
    job_ok
        && i16::from(stats.level) >= equip.req_level
        && str >= i32::from(equip.req_str)
        && dex >= i32::from(equip.req_dex)
        && crate::skills::total_int(player) >= i32::from(equip.req_int)
        && luk >= i32::from(equip.req_luk)
        && (equip.req_fame == 0 || stats.fame >= equip.req_fame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn requirements_follow_the_job_bits_and_what_is_worn() {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 0;
        let beginners = EquipInfo {
            req_job: -1,
            ..EquipInfo::default()
        };
        let warriors = EquipInfo {
            req_job: 1 | 8,
            req_str: player.stats.str + 5,
            ..EquipInfo::default()
        };
        assert!(can_wear(&beginners, &player));
        assert!(!can_wear(&warriors, &player));

        player.stats.job = 2110;
        assert!(!can_wear(&beginners, &player));
        assert!(!can_wear(&warriors, &player), "short of STR");
        let gloves = Item::equipment(
            1082002,
            EquipStats {
                str: 5,
                ..EquipStats::default()
            },
        );
        player
            .inventory
            .insert(game::InventoryType::Equipped, -8, gloves);
        assert!(can_wear(&warriors, &player));

        let famous = EquipInfo {
            req_fame: 1,
            ..EquipInfo::default()
        };
        player.stats.fame = -3;
        assert!(can_wear(&EquipInfo::default(), &player));
        assert!(!can_wear(&famous, &player));
    }

    #[test]
    fn dropped_equipment_varies_by_a_tenth_at_most_five_or_ten() {
        let base = EquipStats {
            upgrade_slots: 7,
            str: 3,
            watk: 80,
            wdef: 200,
            hp: 40,
            hands: 4,
            speed: -5,
            ..EquipStats::default()
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let rolls: Vec<EquipStats> = (0..500)
            .map(|_| {
                let mut stats = base.clone();
                vary(&mut stats, &mut rng);
                stats
            })
            .collect();
        let spread = |stat: fn(&EquipStats) -> i16| {
            let values = rolls.iter().map(stat);
            (values.clone().min().unwrap(), values.max().unwrap())
        };
        assert_eq!(spread(|s| s.str), (2, 4));
        assert_eq!(spread(|s| s.watk), (75, 85));
        assert_eq!(spread(|s| s.wdef), (190, 210));
        assert_eq!(spread(|s| s.hp), (36, 44));
        // Nothing to vary, or a penalty: left as they are.
        assert_eq!(spread(|s| s.dex), (0, 0));
        assert_eq!(spread(|s| s.speed), (-5, -5));
        assert!(rolls.iter().all(|s| (s.upgrade_slots, s.hands) == (7, 4)));
    }
}
