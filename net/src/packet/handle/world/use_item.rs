use super::change_map::warp;
use super::use_skill::show_given;
use crate::handler::{
    FieldRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::items;
use crate::packet::build::world::skill::{self, BookResult};
use crate::packet::build::world::stat::StatUpdate;
use crate::packet::build::world::{buff, inventory, stat};
use crate::skills;
use game::buff::BuffStat;
use game::inventory::equip_slot;
use game::{Buff, InventoryChange, InventoryType, Player, SkillRecord};
use game_data::items::ConsumeSpec;
use game_data::{
    is_use_item, FIELD_LIMIT_MAP_SCROLLS, FIELD_LIMIT_RETURN_SCROLLS, NO_DESTINATION_MAP,
};
use packet::{io::read::PktRead, Packet};
use rand::Rng;
use std::io::Cursor;
use std::time::{Duration, Instant};
use tracing::debug;

/// `USE_ITEM`: tick, slot in the USE bag, item id. Applies the item's `spec`
/// and consumes one: HP/MP (flat and percent), then the diseases it cures,
/// then the stats it raises for `time` ms as a buff of id `-item_id`, which
/// the client lists with the item's icon. A v83 client sends none while
/// Stop Potion or Seduce holds (FUN_00a092fb), and sends only the kinds
/// `is_use_item` lists. An item none of whose effects are supported is
/// refused rather than used up.
pub struct UseItemHandler;

impl WorldHandler for UseItemHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let Some(used) = use_item(ctx.player, slot, item_id, Instant::now()) else {
            debug!(slot, item_id, "Item use refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let stats = &ctx.player.stats;
        let mut result =
            HandlerResult::reply(inventory::build_inventory_operation(true, &used.changes)?)
                .with_reply(stat::build_stat_update(
                    true,
                    &[StatUpdate::Hp(stats.hp), StatUpdate::Mp(stats.mp)],
                )?);
        if !used.cured.is_empty() {
            result = result.with_reply(buff::build_cancel_buff(&used.cured)?);
            let unseen: Vec<_> = used.cured.into_iter().filter(|s| s.shown()).collect();
            if !unseen.is_empty() {
                let foreign = buff::build_cancel_foreign_buff(ctx.player.id(), &unseen)?;
                result = result.with(WorldAction::BuffsChanged(foreign));
            }
        }
        if !used.given.is_empty() {
            result = result.with_reply(buff::build_give_buff(-item_id, used.time, &used.given)?);
            if let Some(foreign) = show_given(ctx.player, &used.given)? {
                result = result.with(WorldAction::BuffsChanged(foreign));
            }
        }
        Ok(result)
    }
}

/// What using an item did.
struct Used {
    changes: Vec<InventoryChange>,
    /// The diseases it cured.
    cured: Vec<BuffStat>,
    /// The stats it raised, for `time` ms.
    given: Vec<(BuffStat, i16)>,
    time: i32,
}

fn use_item(player: &mut Player, slot: i16, item_id: i32, now: Instant) -> Option<Used> {
    if player.stats.hp <= 0 || player.buffs.has(BuffStat::StopPotion) {
        return None;
    }
    let field = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id));
    if !field.is_none_or(|field| field.may_use(item_id)) {
        return None;
    }
    if !is_use_item(item_id) || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let spec = items::info(item_id)?.spec.filter(ConsumeSpec::has_effect)?;
    let changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    let max_hp = i32::from(player.stats.max_hp);
    let max_mp = i32::from(player.stats.max_mp);
    player.recover_hp(spec.hp + max_hp * spec.hp_r / 100);
    player.add_mp(spec.mp + max_mp * spec.mp_r / 100);
    let cured = player.buffs.cure(&spec.cures);
    let given = if spec.time > 0 {
        spec.stats
    } else {
        Vec::new()
    };
    let lasts = Duration::from_millis(spec.time.max(0) as u64);
    for &(stat, value) in &given {
        player
            .buffs
            .give(Buff::new(-item_id, stat, value, now, lasts));
    }
    Some(Used {
        changes,
        cured,
        given,
        time: spec.time,
    })
}

/// `USE_RETURN_SCROLL` (0x55, v83 FUN_00a1e1ce): tick, slot in the USE bag,
/// item id. A return scroll (203xxxx) takes the player to its `moveTo` map,
/// or to the map's return map for 999999999, and is used up. The server
/// checks what the client does before sending: the map's field limits and,
/// unless the scroll ignores continents, that the map it names is in the
/// player's area.
pub struct ReturnScrollHandler;

impl WorldHandler for ReturnScrollHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let taken = return_scroll_target(ctx.player, slot, item_id).and_then(|map_id| {
            let changes = ctx
                .player
                .inventory_mut()
                .take(InventoryType::Use, slot, 1)
                .ok()?;
            Some((map_id, changes))
        });
        let Some((map_id, changes)) = taken else {
            debug!(slot, item_id, "Return scroll refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let mut result =
            HandlerResult::reply(inventory::build_inventory_operation(true, &changes)?);
        result
            .actions
            .extend(warp(ctx.player, map_id, "sp", ctx.channel_id)?);
        Ok(result)
    }
}

/// The map the return scroll in `slot` takes the player to, unless it may
/// not be used.
fn return_scroll_target(player: &Player, slot: i16, item_id: i32) -> Option<i32> {
    if player.stats.hp <= 0
        || item_id / 10_000 != 203
        || player.inventory.get(InventoryType::Use, slot)?.item_id != item_id
    {
        return None;
    }
    let spec = items::info(item_id)?.spec?;
    // Event scrolls keep the way back in a quest record (`returnMapQR`),
    // and 2032000 moves the player within its PQ (`randomMoveInFieldSet`);
    // neither is supported yet.
    if spec.return_map_qr != 0 {
        return None;
    }
    let move_to = spec.move_to?;
    let data = crate::get_game_data().ok()?;
    let here = player.stats.map_id;
    let field = data.field(here)?;
    if field.field_limit & FIELD_LIMIT_RETURN_SCROLLS != 0 {
        return None;
    }
    let target = if move_to == NO_DESTINATION_MAP {
        field.return_map?
    } else if field.field_limit & FIELD_LIMIT_MAP_SCROLLS != 0
        || (!spec.ignore_continent && !data.same_area(here, move_to))
        || move_to == here
    {
        return None;
    } else {
        move_to
    };
    data.field_exists(target).then_some(target)
}

/// `USE_SKILL_BOOK` (0x52, v83 FUN_00a0a1b2): tick, slot in the USE bag,
/// item id. The book raises the master level of its skill for the reader's
/// job (OdinMS family: the job must match, the skill must be at
/// `reqSkillLevel` and below the book's master level). A usable book is spent
/// whether or not it works. Everyone on the map sees the result.
pub struct UseSkillBookHandler;

impl WorldHandler for UseSkillBookHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?; // client tick
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let Some(read) = read_book(ctx.player, slot, item_id, |success| {
            rand::thread_rng().gen_range(0, 100) < success
        }) else {
            debug!(slot, item_id, "Skill book use refused");
            return Ok(HandlerResult::reply(stat::build_stat_update(true, &[])?));
        };
        let mut result = HandlerResult::empty();
        if !read.changes.is_empty() {
            result = result.with_reply(inventory::build_inventory_operation(true, &read.changes)?);
        }
        if let Some(record) = &read.record {
            result = result.with_reply(skill::build_update_skill(record)?);
        }
        let shown = skill::build_skill_book_result(&read.result)?;
        Ok(result
            .with_reply(shown.clone())
            .with(WorldAction::Field(FieldRequest::Broadcast(shown))))
    }
}

struct BookRead {
    result: BookResult,
    changes: Vec<InventoryChange>,
    /// The skill with its new master level, when the book worked.
    record: Option<SkillRecord>,
}

/// Read the skill or mastery book in `slot`; `works(success)` rolls the
/// book's percent chance. `None` when there is no such book there.
fn read_book(
    player: &mut Player,
    slot: i16,
    item_id: i32,
    works: impl FnOnce(i32) -> bool,
) -> Option<BookRead> {
    if player.inventory.get(InventoryType::Use, slot)?.item_id != item_id {
        return None;
    }
    let book = items::info(item_id)?.book?;
    let mut read = BookRead {
        result: BookResult {
            character_id: player.id(),
            mastery: item_id / 10_000 == 229,
            skill_id: 0,
            master_level: 0,
            used: false,
            success: false,
        },
        changes: Vec::new(),
        record: None,
    };
    let job = i32::from(player.stats.job);
    let Some(&skill_id) = book.skills.iter().find(|&&skill| skill / 10_000 == job) else {
        return Some(read);
    };
    read.result.skill_id = skill_id;
    read.result.master_level = book.master_level;
    let mut record = player.skills.get(skill_id).cloned().unwrap_or(SkillRecord {
        skill_id,
        level: 0,
        master_level: 0,
        expiration: None,
    });
    if record.level < book.req_skill_level || record.master_level >= book.master_level {
        return Some(read);
    }
    read.changes = player
        .inventory_mut()
        .take(InventoryType::Use, slot, 1)
        .ok()?;
    read.result.used = true;
    if works(book.success) {
        record.master_level = book.master_level;
        player.skills.set(record.clone());
        read.result.success = true;
        read.record = Some(record);
    }
    Some(read)
}

/// `HEAL_OVER_TIME`: the client's periodic natural recovery. The runtime
/// checks the timing; this only parses and bounds the amounts.
pub struct HealOverTimeHandler;

/// v83 heals 1.5 times as much on a chair or seat.
const SEATED_RECOVERY_RATE: f64 = 1.5;

/// Most the player may recover per tick: their natural amount at the map's
/// `info/recovery` rate (and their top's, see `top_recovery`), seated, or a
/// portable chair's `recoveryHP`/`recoveryMP` while they sit on one they
/// still own.
fn max_heal(player: &Player) -> (i16, i16) {
    let natural = skills::natural_recovery(player);
    let map_rate = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(player.stats.map_id))
        .map_or(1.0, |field| field.recovery);
    let rate = map_rate * top_recovery(player, map_rate) * SEATED_RECOVERY_RATE;
    let chair = (player.chair != 0 && player.inventory.count(player.chair) > 0)
        .then(|| items::info(player.chair))
        .flatten();
    let (chair_hp, chair_mp) = chair.map_or((0, 0), |chair| (chair.recovery_hp, chair.recovery_mp));
    let bound = |natural: i32, chair: i32| {
        let natural = (f64::from(natural) * rate) as i32;
        i16::try_from(natural.max(chair)).unwrap_or(i16::MAX)
    };
    (bound(natural.hp, chair_hp), bound(natural.mp, chair_mp))
}

/// The worn top's `info/recovery`, which v83 only applies on a map that
/// already recovers faster than usual, such as a sauna robe in the
/// Sleepywood sauna.
fn top_recovery(player: &Player, map_rate: f64) -> f64 {
    if map_rate <= 1.0 {
        return 1.0;
    }
    player
        .inventory
        .get(InventoryType::Equipped, equip_slot::TOP)
        .and_then(|top| items::info(top.item_id))
        .and_then(|info| info.equip?.recovery)
        .unwrap_or(1.0)
}

impl WorldHandler for HealOverTimeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_int()?;
        reader.read_int()?;
        let hp = reader.read_short()?;
        let mp = reader.read_short()?;
        let (max_hp, max_mp) = max_heal(ctx.player);
        if !(0..=max_hp).contains(&hp) || !(0..=max_mp).contains(&mp) {
            debug!(hp, mp, "Natural recovery out of range");
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::NaturalHeal { hp, mp }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::Item;
    use packet::io::write::PktWrite;

    fn request(slot: i16, item_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseItem as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p
    }

    fn handle(player: &mut Player, packet: Packet) -> Vec<WorldAction> {
        let mut packet = packet;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        UseItemHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn eating_an_apple_heals_and_consumes_it() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 50;
        player.stats.hp = 25;
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2010007, 1));
        let actions = handle(&mut player, request(3, 2010007));
        assert!(
            matches!(actions.as_slice(), [WorldAction::Reply(a), WorldAction::Reply(b)]
            if a.opcode() == SendOpcode::InventoryOperation as i16 && b.opcode() == SendOpcode::StatChange as i16)
        );
        assert_eq!(player.stats.hp, 50);
        assert!(player.inventory.get(InventoryType::Use, 3).is_none());
    }

    #[test]
    fn stop_potion_refuses_items_and_undead_halves_their_hp() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 500;
        player.stats.hp = 100;
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2000000, 2));
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(10);
        player.buffs.give(game::Buff::new(
            134 | 1 << 16,
            BuffStat::StopPotion,
            1,
            now,
            lasts,
        ));
        assert!(use_item(&mut player, 3, 2000000, now).is_none());
        assert_eq!(player.stats.hp, 100);

        // A Red Potion heals 50; undead, 25.
        player.buffs.clear();
        player.buffs.give(game::Buff::new(
            133 | 1 << 16,
            BuffStat::Undead,
            70,
            now,
            lasts,
        ));
        assert!(use_item(&mut player, 3, 2000000, now).is_some());
        assert_eq!(player.stats.hp, 125);
    }

    #[test]
    fn maps_that_forbid_consumables_allow_only_their_list() {
        if items::data().is_err() || crate::get_game_data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 500;
        player.stats.hp = 100;
        player
            .inventory
            .insert(InventoryType::Use, 3, Item::stack(2000000, 2));
        let now = std::time::Instant::now();
        player.stats.map_id = 913_040_000;
        assert!(use_item(&mut player, 3, 2000000, now).is_none());
        assert_eq!(player.inventory.count(2000000), 2);

        // 109090300 forbids consumables but lists 2022539 (whose only
        // effect, BFSkill, isn't supported, so it isn't used up either).
        player.stats.map_id = 109_090_300;
        assert!(use_item(&mut player, 3, 2000000, now).is_none());
        let field = crate::get_game_data().unwrap().field(109_090_300).unwrap();
        assert!(field.may_use(2022539) && !field.may_use(2000000));
    }

    #[test]
    fn a_pill_buffs_for_its_time_and_all_cure_cures() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2002004, 1));
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2050004, 1));
        let now = std::time::Instant::now();
        // Warrior Pill: +5 weapon attack for three minutes.
        let used = use_item(&mut player, 1, 2002004, now).unwrap();
        assert_eq!((used.given, used.time), (vec![(BuffStat::Pad, 5)], 180_000));
        let buff = &player.buffs.active()[0];
        assert_eq!(
            (buff.skill_id, buff.stat, buff.value),
            (-2002004, BuffStat::Pad, 5)
        );
        let ended = player.buffs.cancel(-2002004);
        assert_eq!(ended, [BuffStat::Pad]);

        // All Cure ends Poison and Seal but not Stun.
        for (n, stat) in [BuffStat::Poison, BuffStat::Seal, BuffStat::Stun]
            .into_iter()
            .enumerate()
        {
            player.buffs.give(game::Buff::new(
                (120 + n as i32) | 1 << 16,
                stat,
                1,
                now,
                std::time::Duration::from_secs(10),
            ));
        }
        let mut packet = request(2, 2050004);
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = UseItemHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        let opcodes: Vec<_> = actions
            .iter()
            .map(|action| match action {
                WorldAction::Reply(p) | WorldAction::BuffsChanged(p) => p.opcode(),
                _ => 0,
            })
            .collect();
        assert_eq!(
            opcodes,
            [
                SendOpcode::InventoryOperation as i16,
                SendOpcode::StatChange as i16,
                SendOpcode::CancelBuff as i16,
                SendOpcode::CancelForeignBuff as i16,
            ]
        );
        assert!(player.buffs.has(BuffStat::Stun));
        assert!(!player.buffs.has(BuffStat::Poison) && !player.buffs.has(BuffStat::Seal));
    }

    #[test]
    fn items_with_nothing_supported_are_not_used_up() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        // A Return Scroll is no USE_ITEM; a Warp Scroll for the Kerning PQ
        // (2032000) has a spec but nothing USE_ITEM applies.
        for (slot, item_id) in [(1, 2030000), (2, 2032000)] {
            player
                .inventory
                .insert(InventoryType::Use, slot, Item::stack(item_id, 1));
            assert!(use_item(&mut player, slot, item_id, Instant::now()).is_none());
            assert_eq!(player.inventory.count(item_id), 1);
        }
    }

    fn scroll_request(slot: i16, item_id: i32) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseReturnScroll as i16).unwrap();
        p.write_int(0).unwrap();
        p.write_short(slot).unwrap();
        p.write_int(item_id).unwrap();
        p
    }

    fn read_scroll(player: &mut Player, slot: i16, item_id: i32) -> Vec<WorldAction> {
        let mut packet = scroll_request(slot, item_id);
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ReturnScrollHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
    }

    #[test]
    fn a_return_scroll_goes_to_its_map_or_the_return_map() {
        if items::data().is_err() || crate::get_game_data().is_err() {
            return;
        }
        // From Ellinia's Tree Dungeon (101010000), Nearest Town goes to
        // Ellinia and Return to Henesys goes to Henesys.
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 101_010_000;
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2030000, 2));
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2030004, 1));
        assert_eq!(return_scroll_target(&player, 1, 2030000), Some(101_000_000));
        let actions = read_scroll(&mut player, 2, 2030004);
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::Reply(op), WorldAction::Reply(_), WorldAction::ChangeField { map_id: 100_000_000, .. }]
            if op.opcode() == SendOpcode::InventoryOperation as i16
        ));
        assert_eq!(player.stats.map_id, 100_000_000);
        assert_eq!(player.inventory.count(2030004), 0);
        // Already in Henesys: not used.
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2030004, 1));
        assert_eq!(return_scroll_target(&player, 2, 2030004), None);
    }

    #[test]
    fn a_return_scroll_stays_in_its_area_and_obeys_field_limits() {
        if items::data().is_err() || crate::get_game_data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 200_000_000;
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2030004, 1));
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2030000, 1));
        // Return to Henesys doesn't cross from Orbis; Nearest Town does go.
        let actions = read_scroll(&mut player, 1, 2030004);
        assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
            if p.opcode() == SendOpcode::StatChange as i16));
        assert_eq!(player.inventory.count(2030004), 1);
        assert_eq!(return_scroll_target(&player, 2, 2030000), Some(200_000_000));

        // The Kerning PQ's first stage forbids every return scroll.
        player.stats.map_id = 103_000_800;
        let field = crate::get_game_data().unwrap().field(103_000_800).unwrap();
        assert_ne!(field.field_limit & FIELD_LIMIT_RETURN_SCROLLS, 0);
        assert_eq!(return_scroll_target(&player, 2, 2030000), None);
    }

    fn hero_with_book(slot: i16, item_id: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 112;
        player
            .inventory
            .insert(InventoryType::Use, slot, Item::stack(item_id, 1));
        player
    }

    fn skill(player: &Player, skill_id: i32) -> (i32, i32) {
        player
            .skills
            .get(skill_id)
            .map_or((0, 0), |s| (s.level, s.master_level))
    }

    #[test]
    fn a_mastery_book_needs_its_skill_level_then_raises_the_master_level() {
        if items::data().is_err() {
            return;
        }
        // Mastery Book 2290000: Monster Magnet for Heroes, Paladins and Dark
        // Knights to master level 20 from skill level 5, 70%.
        let mut player = hero_with_book(2, 2290000);
        let read = read_book(&mut player, 2, 2290000, |_| true).unwrap();
        assert_eq!((read.result.skill_id, read.result.used), (1121001, false));
        assert!(read.result.mastery && read.changes.is_empty());

        player.skills.set(SkillRecord {
            skill_id: 1121001,
            level: 5,
            master_level: 10,
            expiration: None,
        });
        let read = read_book(&mut player, 2, 2290000, |success| {
            assert_eq!(success, 70);
            false
        })
        .unwrap();
        assert!(read.result.used && !read.result.success && read.record.is_none());
        assert_eq!(skill(&player, 1121001), (5, 10));
        assert!(player.inventory.get(InventoryType::Use, 2).is_none());

        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2290000, 1));
        let read = read_book(&mut player, 2, 2290000, |_| true).unwrap();
        assert!(read.result.used && read.result.success);
        assert_eq!(read.result.master_level, 20);
        assert_eq!(skill(&player, 1121001), (5, 20));

        // Already at the book's master level: not used, not spent.
        player
            .inventory
            .insert(InventoryType::Use, 2, Item::stack(2290000, 1));
        let read = read_book(&mut player, 2, 2290000, |_| true).unwrap();
        assert!(!read.result.used);
        assert!(player.inventory.get(InventoryType::Use, 2).is_some());
    }

    #[test]
    fn a_book_for_another_job_is_not_used() {
        if items::data().is_err() {
            return;
        }
        // Skill Book 2280000 teaches Fire Demon (2121003) to Arch Mage (F/P).
        let mut player = hero_with_book(1, 2280000);
        let read = read_book(&mut player, 1, 2280000, |_| true).unwrap();
        assert_eq!(
            read.result,
            BookResult {
                character_id: player.id(),
                mastery: false,
                skill_id: 0,
                master_level: 0,
                used: false,
                success: false,
            }
        );
        assert!(read_book(&mut player, 1, 2290000, |_| true).is_none());
    }

    #[test]
    fn mismatched_or_missing_items_are_refused() {
        let mut player = crate::codec::stats::sample_player();
        player
            .inventory
            .insert(InventoryType::Use, 1, Item::stack(2000000, 5));
        for packet in [request(1, 2000001), request(2, 2000000)] {
            let actions = handle(&mut player, packet);
            assert!(matches!(actions.as_slice(), [WorldAction::Reply(p)]
                if p.opcode() == SendOpcode::StatChange as i16));
        }
        assert_eq!(
            player
                .inventory
                .get(InventoryType::Use, 1)
                .unwrap()
                .quantity,
            5
        );
    }

    fn heal_over_time(player: &mut Player, hp: i16, mp: i16) -> bool {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::HealOverTime as i16).unwrap();
        packet.write_int(0).unwrap();
        packet.write_int(0).unwrap();
        packet.write_short(hp).unwrap();
        packet.write_short(mp).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let actions = HealOverTimeHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions;
        matches!(actions.as_slice(), [WorldAction::NaturalHeal { hp: h, mp: m }] if (*h, *m) == (hp, mp))
    }

    #[test]
    fn a_portable_chair_raises_the_heal_bound() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        assert!(heal_over_time(&mut player, 10, 3));
        assert!(!heal_over_time(&mut player, 50, 0));
        // The Relaxer heals 50 HP.
        player
            .inventory
            .insert(InventoryType::Setup, 1, Item::stack(3010000, 1));
        player.chair = 3010000;
        assert!(heal_over_time(&mut player, 50, 0));
        assert!(heal_over_time(&mut player, 0, 4));
        assert!(!heal_over_time(&mut player, 51, 0));
        // Not once the chair has left the inventory.
        player.inventory.take(InventoryType::Setup, 1, 1).unwrap();
        assert!(!heal_over_time(&mut player, 50, 0));
    }

    #[test]
    fn a_sauna_robe_speeds_recovery_only_where_the_map_does() {
        if items::data().is_err() {
            return;
        }
        let mut player = crate::codec::stats::sample_player();
        assert_eq!(top_recovery(&player, 2.0), 1.0);
        player.inventory.insert(
            InventoryType::Equipped,
            equip_slot::TOP,
            Item::stack(1050018, 1),
        );
        assert_eq!(top_recovery(&player, 2.0), 1.5);
        assert_eq!(top_recovery(&player, 1.0), 1.0);
    }

    #[test]
    fn recovery_skills_raise_the_heal_bound() {
        if skills::data().is_err() {
            return;
        }
        let learn = |player: &mut Player, skill_id, level| {
            player.skills.set(game::SkillRecord {
                skill_id,
                level,
                master_level: 0,
                expiration: None,
            })
        };
        // Improved HP Recovery 16 adds 50 HP, seated 1.5 times (10 + 50).
        let mut player = crate::codec::stats::sample_player();
        learn(&mut player, skills::IMPROVED_HP_RECOVERY, 16);
        assert!(heal_over_time(&mut player, 90, 4));
        assert!(!heal_over_time(&mut player, 91, 0));
        // A level 50 magician with Improved MP Recovery 16 adds 80 MP.
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 200;
        player.stats.level = 50;
        learn(&mut player, skills::IMPROVED_MP_RECOVERY, 16);
        assert!(heal_over_time(&mut player, 15, 124));
        assert!(!heal_over_time(&mut player, 0, 125));
    }

    #[test]
    fn a_recovery_map_raises_the_heal_bound() {
        if crate::get_game_data().is_err() {
            return;
        }
        // The Sleepywood sauna heals twice as fast.
        let mut player = crate::codec::stats::sample_player();
        player.stats.map_id = 105040401;
        assert!(heal_over_time(&mut player, 30, 9));
        assert!(!heal_over_time(&mut player, 31, 0));
    }
}
