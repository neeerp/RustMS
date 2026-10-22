//! Item templates (`Item.nx`, `Character.nx`), loaded once, and helpers that
//! turn them into inventory items.

use crate::load::{self, asset_path};
use game::inventory::FLAG_LOCK;
use game::{EquipStats, Inventory, InventoryChange, InventoryError, InventoryType, Item, Player};
use game_data::items::{EquipInfo, ItemData, ItemInfo, ItemNames};
use game_data::maker::MakerData;
use game_data::vega_spell::VegaSpells;
use rand::Rng;
use std::sync::OnceLock;
use std::time::SystemTime;

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

/// An item's name, as String.nx has it.
pub fn item_name(item_id: i32) -> Option<String> {
    names().ok()?.item(item_id).ok().flatten()
}

/// The map's name, as the client shows it.
pub fn map_name(map_id: i32) -> Option<String> {
    names().ok()?.map_name(map_id).ok().flatten()
}

/// The template for `item_id`, if the data has it.
pub fn info(item_id: i32) -> Option<ItemInfo> {
    data().ok()?.info(item_id).ok().flatten()
}

/// The success rate in percent of `scroll_id`, if Vega's Spell takes it
/// (`Etc.nx`).
pub fn vega_rate(scroll_id: i32) -> Option<i32> {
    static SPELLS: OnceLock<Result<VegaSpells, String>> = OnceLock::new();
    load::once(&SPELLS, || {
        VegaSpells::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
    .ok()?
    .rate(scroll_id)
}

/// The Item Maker's recipes and what makes each monster crystal (`Etc.nx`,
/// `Item.nx`).
pub fn maker() -> Option<&'static MakerData> {
    static MAKER: OnceLock<Result<MakerData, String>> = OnceLock::new();
    load::once(&MAKER, || {
        MakerData::load(
            asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"),
            asset_path("RUSTMS_ITEM_NX_PATH", "assets/game-data/Item.nx"),
        )
    })
    .ok()
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

/// Whether `quantity` of `item_id` can't come into `inventory` because the
/// item is one of a kind (`info/only`): it comes alone, and not to someone
/// who holds one, worn or not.
pub fn refuses_only(inventory: &Inventory, item_id: i32, quantity: i32) -> bool {
    if !info(item_id).is_some_and(|info| info.only) {
        return false;
    }
    quantity > 1
        || inventory.count(item_id) > 0
        || inventory
            .items(InventoryType::Equipped)
            .any(|(_, item)| item.item_id == item_id)
}

/// Put `quantity` of `item_id` into `inventory`, one slot per piece of
/// equipment. Nothing changes unless all of it fits.
pub fn give(
    inventory: &mut Inventory,
    item_id: i32,
    quantity: i16,
) -> Result<Vec<InventoryChange>, InventoryError> {
    give_until(inventory, item_id, quantity, None)
}

/// [`give`] items that expire at `until`; `None` never does.
pub fn give_until(
    inventory: &mut Inventory,
    item_id: i32,
    quantity: i16,
    until: Option<SystemTime>,
) -> Result<Vec<InventoryChange>, InventoryError> {
    let mut item = create(item_id, quantity).ok_or(InventoryError::UnknownItem)?;
    item.expiration = until;
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

/// What a sweep of expired items took from a player.
#[derive(Debug, Default)]
pub struct Expired {
    /// The items gone, for the "has been expired" lines.
    pub item_ids: Vec<i32>,
    /// The items whose seal ran out, for the "seal has expired" lines.
    pub released: Vec<i32>,
    pub changes: Vec<InventoryChange>,
    /// Whether one was worn, which changes the player's look.
    pub worn: bool,
}

/// When `item` expires: pets age instead (they turn into dolls), and v83
/// takes 2079 or later for never.
fn expires(item: &Item) -> Option<SystemTime> {
    item.expiration
        .filter(|&at| item.pet.is_none() && at < game::pet::doll_date())
}

/// Take every item, worn or in the bags, whose date passed by `now`
/// (Cosmic's expirationTask). v83's client removes none itself. A timed
/// seal's date is the seal's: the item stays, unsealed and lasting forever.
pub fn expire_items(player: &mut Player, now: SystemTime) -> Expired {
    let due: Vec<(InventoryType, i16, Item)> = player
        .inventory
        .all()
        .filter(|(_, _, item)| expires(item).is_some_and(|at| at <= now))
        .map(|(kind, slot, item)| (kind, slot, item.clone()))
        .collect();
    let mut expired = Expired::default();
    for (kind, slot, mut item) in due {
        let item_id = item.item_id;
        let Ok(changes) = player.inventory_mut().take(kind, slot, item.quantity) else {
            continue;
        };
        expired.changes.extend(changes);
        if item.sealed() {
            item.flags &= !FLAG_LOCK;
            item.expiration = None;
            player.inventory_mut().insert(kind, slot, item.clone());
            expired
                .changes
                .push(InventoryChange::Add { kind, slot, item });
            expired.released.push(item_id);
        } else {
            expired.item_ids.push(item_id);
            expired.worn |= kind == InventoryType::Equipped;
        }
    }
    expired
}

/// Take every item, worn or in the bags, that lasts only until its owner
/// logs out (`info/expireOnLogout`): event weapons and a few set-up and
/// etc items. v83's client says nothing of it (report c005). Whether any
/// went.
pub fn drop_logout_items(player: &mut Player) -> bool {
    let due: Vec<(InventoryType, i16, i16)> = player
        .inventory
        .all()
        .filter(|(_, _, item)| info(item.item_id).is_some_and(|info| info.expire_on_logout))
        .map(|(kind, slot, item)| (kind, slot, item.quantity))
        .collect();
    for &(kind, slot, quantity) in &due {
        let _ = player.inventory_mut().take(kind, slot, quantity);
    }
    !due.is_empty()
}

/// When the next of the player's items expires, if one ever does.
pub fn next_expiry(player: &Player) -> Option<SystemTime> {
    player
        .inventory
        .all()
        .filter_map(|(_, _, item)| expires(item))
        .min()
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
    fn items_whose_date_passed_go_but_pets_and_2079_stay() {
        use std::time::Duration;
        let mut player = crate::codec::stats::sample_player();
        let now = SystemTime::now();
        let dated = |item_id, at| Item {
            expiration: Some(at),
            ..Item::stack(item_id, 3)
        };
        let past = now - Duration::from_secs(1);
        let soon = now + Duration::from_secs(60);
        let bag = player.inventory_mut();
        bag.insert(InventoryType::Etc, 1, dated(4031281, past));
        bag.insert(InventoryType::Etc, 2, dated(4031126, soon));
        bag.insert(
            InventoryType::Etc,
            3,
            dated(4031017, game::pet::doll_date()),
        );
        let cape = Item {
            expiration: Some(past),
            ..Item::equipment(1102135, EquipStats::default())
        };
        bag.insert(InventoryType::Equipped, -9, cape);
        let kitty = Item {
            pet: Some(game::pet::PetData::new("Kitty")),
            ..dated(5000000, past)
        };
        bag.insert(InventoryType::Cash, 1, kitty);

        let expired = expire_items(&mut player, now);
        assert_eq!(expired.item_ids, [1102135, 4031281]);
        assert!(expired.worn);
        assert_eq!(
            expired.changes,
            [
                InventoryChange::Remove {
                    kind: InventoryType::Equipped,
                    slot: -9
                },
                InventoryChange::Remove {
                    kind: InventoryType::Etc,
                    slot: 1
                },
            ]
        );
        assert_eq!(next_expiry(&player), Some(soon));
        assert!(expire_items(&mut player, now).item_ids.is_empty());
    }

    /// The first one-of-a-kind etc item, as the trade tests find one.
    fn only_etc() -> i32 {
        (4031000..4032000)
            .find(|&id| info(id).is_some_and(|info| info.only))
            .expect("an only item")
    }

    #[test]
    fn a_one_of_a_kind_item_comes_alone_to_someone_without_one() {
        let mut player = crate::codec::stats::sample_player();
        let etc = only_etc();
        assert!(!refuses_only(&player.inventory, etc, 1));
        assert!(refuses_only(&player.inventory, etc, 2), "it comes alone");
        give(player.inventory_mut(), etc, 1).unwrap();
        assert!(refuses_only(&player.inventory, etc, 1));
        give(player.inventory_mut(), 4000000, 1).unwrap();
        assert!(
            !refuses_only(&player.inventory, 4000000, 2),
            "not one of a kind"
        );

        let medal = (1142000..1143000)
            .find(|&id| info(id).is_some_and(|info| info.only))
            .expect("an only medal");
        player.inventory.insert(
            InventoryType::Equipped,
            -49,
            Item::equipment(medal, EquipStats::default()),
        );
        assert!(refuses_only(&player.inventory, medal, 1));
    }

    #[test]
    fn logout_items_go_and_the_rest_stay() {
        assert!([1302050, 4001333, 3994085]
            .iter()
            .all(|&id| info(id).is_some_and(|info| info.expire_on_logout)));
        let mut player = crate::codec::stats::sample_player();
        player.inventory.insert(
            InventoryType::Equipped,
            -11,
            Item::equipment(1302050, EquipStats::default()),
        );
        give(player.inventory_mut(), 4001333, 3).unwrap();
        give(player.inventory_mut(), 4000000, 2).unwrap();
        assert!(drop_logout_items(&mut player));
        assert_eq!(player.inventory.count(4001333), 0);
        assert!(player.inventory.get(InventoryType::Equipped, -11).is_none());
        assert_eq!(player.inventory.count(4000000), 2);
        assert!(!drop_logout_items(&mut player), "nothing left to drop");
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
