//! ITEM_MAKE (0x71, v83 FUN_00827096): the Item Maker's Start, sent once
//! the window's gauge is full (reports c006, c006b and c006e). Its mode
//! says what to make: 1 a recipe, 2 the "???" random make, 3 a monster
//! crystal, 4 a disassembly. The window checks everything first but sends
//! no price, so the recipe and the fee come from the data here.
//!
//! Equipment may take its recipe's stimulator and up to its `tuc` gems.
//! The random make fails so far. The window waits for an answer to every
//! request.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::maker::{
    build_maker_crystal, build_maker_disassembled, build_maker_effect, build_maker_made,
    build_maker_refusal, MakerCost, MakerRefusal,
};
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::quest::QuestState;
use game::{EquipStats, InventoryChange, InventoryError, InventoryType, Player};
use game_data::maker::{Gem, Recipe, CRYSTALS};
use packet::{io::read::PktRead, Packet};
use rand::Rng;
use std::io::Cursor;

/// How many drops one monster crystal takes.
const CRYSTAL_DROPS: i16 = 100;
/// The most gem slots the window has (four for the random make).
const MOST_GEMS: i32 = 4;
/// In ten, how often a stimulator destroys what it goes into (Cosmic).
const DESTROYED_IN_TEN: i32 = 1;

pub struct ItemMakeHandler;

impl WorldHandler for ItemMakeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let roll = &mut |total| rand::thread_rng().gen_range(0, total);
        let made = request(packet).and_then(|request| match request {
            Request::Make {
                item_id,
                stimulator,
                gems,
            } => make(ctx.player, item_id, stimulator, &gems, roll),
            Request::Crystal(item_id) => crystal(ctx.player, item_id),
            Request::Disassemble { item_id, slot } => disassemble(ctx.player, item_id, slot),
        });
        let made = match made {
            Ok(made) => made,
            Err(why) => return Ok(HandlerResult::reply(build_maker_refusal(why)?)),
        };
        ctx.player.mark_dirty();
        let mut result = HandlerResult::empty();
        if made.meso != 0 {
            result = result.with_reply(build_stat_update(
                true,
                &[StatUpdate::Meso(ctx.player.meso)],
            )?);
        }
        Ok(result
            .with_reply(build_inventory_operation(true, &made.changes)?)
            .with_reply(made.result)
            .with_reply(build_maker_effect(made.success)?))
    }
}

/// What the Maker can be asked so far.
enum Request {
    /// A recipe, and for equipment whether its stimulator goes in and the
    /// gems that do.
    Make {
        item_id: i32,
        stimulator: bool,
        gems: Vec<i32>,
    },
    Crystal(i32),
    /// The equipment `item_id` in the Equip tab's `slot`.
    Disassemble {
        item_id: i32,
        slot: i16,
    },
}

/// Read a request, refusing what the Maker doesn't make yet and anything
/// cut short, since the window waits for an answer either way.
fn request(packet: &Packet) -> Result<Request, MakerRefusal> {
    let read = || -> Result<Request, std::io::Error> {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let mode = r.read_int()?;
        let item_id = r.read_int()?;
        Ok(match mode {
            1 => {
                let stimulator = r.read_byte()? != 0;
                let count = r.read_int()?;
                if !(0..=MOST_GEMS).contains(&count) {
                    return Err(std::io::ErrorKind::InvalidData.into());
                }
                let gems = (0..count).map(|_| r.read_int()).collect::<Result<_, _>>()?;
                Request::Make {
                    item_id,
                    stimulator,
                    gems,
                }
            }
            3 => Request::Crystal(item_id),
            4 => {
                let kind = r.read_int()?;
                let slot = i16::try_from(r.read_int()?);
                match (kind, slot) {
                    (1, Ok(slot)) => Request::Disassemble { item_id, slot },
                    _ => return Err(std::io::ErrorKind::InvalidData.into()),
                }
            }
            _ => return Err(std::io::ErrorKind::Unsupported.into()),
        })
    };
    read().map_err(|_| MakerRefusal::Failed)
}

/// What a request made: the inventory changes, the mesos it cost, the
/// MAKER_RESULT that tells the window, and whether the effect glows (a
/// stimulator may destroy what it made).
struct Made {
    changes: Vec<InventoryChange>,
    meso: i32,
    result: Packet,
    success: bool,
}

/// Make `item_id` from its recipe, with its stimulator and `gems` if they
/// go in; `roll(total)` picks a number below `total` for a gem recipe's
/// outcome, the stimulator and the stats. Nothing changes unless it works,
/// though a stimulator may destroy what it made, and the cost with it.
fn make(
    player: &mut Player,
    item_id: i32,
    stimulator: bool,
    gems: &[i32],
    roll: &mut impl FnMut(i32) -> i32,
) -> Result<Made, MakerRefusal> {
    use MakerRefusal::Failed;
    let maker = items::maker().ok_or(Failed)?;
    let recipe = maker.recipe(item_id).ok_or(Failed)?;
    if !may_make(player, recipe) {
        return Err(Failed);
    }
    let stimulator = stimulator.then_some(recipe.catalyst);
    if stimulator == Some(0) || !gems_fit(recipe, gems) {
        return Err(Failed);
    }
    let meso = fee(recipe.meso, stimulator.is_some(), gems);
    if player.meso < meso {
        return Err(Failed);
    }
    let destroyed = stimulator.is_some() && roll(10) < DESTROYED_IN_TEN;
    let mut next = player.inventory.clone();
    let mut changes = Vec::new();
    let mut take = |next: &mut game::Inventory, id: i32, count: i32| {
        let count = i16::try_from(count).map_err(|_| Failed)?;
        changes.extend(next.remove(id, count).map_err(|_| Failed)?);
        Ok::<_, MakerRefusal>(())
    };
    for &(ingredient, needed) in &recipe.ingredients {
        take(&mut next, ingredient, needed)?;
    }
    for &gem in gems.iter().chain(&stimulator) {
        take(&mut next, gem, 1)?;
    }
    let (made, count) = outcome(recipe, roll);
    let count = i16::try_from(count).map_err(|_| Failed)?;
    if !destroyed {
        if items::refuses_only(&player.inventory, made, i32::from(count)) {
            return Err(Failed);
        }
        let added = match items::create(made, count) {
            Some(mut item) if item.equip.is_some() => {
                let stats = item.equip.as_mut().expect("equipment");
                for gem in gems.iter().filter_map(|&gem| maker.gem(gem)) {
                    put_gem(stats, gem, roll);
                }
                if stimulator.is_some() {
                    stimulate(stats, roll);
                }
                next.add(item, 1)
            }
            _ => items::give(&mut next, made, count),
        };
        changes.extend(added.map_err(|why| no_room(why, made))?);
    }
    let cost = MakerCost {
        lost: recipe.ingredients.clone(),
        gems: gems.to_vec(),
        stimulator,
        meso,
    };
    let result = build_maker_made((!destroyed).then_some((made, i32::from(count))), &cost)
        .map_err(|_| Failed)?;
    *player.inventory_mut() = next;
    player.meso -= meso;
    Ok(Made {
        changes,
        meso,
        result,
        success: !destroyed,
    })
}

/// Whether `gems` may go into what `recipe` makes, as the window lets them
/// (FUN_0082637e): no more than its slots, gems all, one of each kind, and
/// attack gems on weapons only.
fn gems_fit(recipe: &Recipe, gems: &[i32]) -> bool {
    let Some(maker) = items::maker() else {
        return gems.is_empty();
    };
    let kind = |gem: i32| gem % 10_000 / 100;
    let weapon = (130..150).contains(&(recipe.item_id / 10_000));
    gems.len() <= usize::try_from(recipe.gem_slots).unwrap_or(0)
        && gems.iter().enumerate().all(|(i, &gem)| {
            maker.gem(gem).is_some()
                && gems[..i].iter().all(|&other| kind(other) != kind(gem))
                && (weapon || kind(gem) > 1)
        })
}

/// Put a gem's stat into a new piece of equipment. A gem that lowers the
/// level to wear it does nothing, since v83 equipment has no such stat;
/// the random ones move each stat the item has by up to their amount
/// either way (Cosmic's chaos scroll roll), the stat gems first.
fn put_gem(stats: &mut EquipStats, gem: Gem, roll: &mut impl FnMut(i32) -> i32) {
    let mut shake = |stat: &mut i16, by: i16| {
        if *stat > 0 {
            let by = i32::from(by);
            let moved = roll(2 * by + 1) - by;
            *stat = (i32::from(*stat) + moved).clamp(0, i32::from(i16::MAX)) as i16;
        }
    };
    match gem {
        Gem::Stat(name, by) => {
            if let Some(stat) = gem_stat(stats, name) {
                *stat = stat.saturating_add(by);
            }
        }
        Gem::ReqLevel(_) => {}
        Gem::RandStat(by) => {
            for stat in [
                &mut stats.str,
                &mut stats.dex,
                &mut stats.int,
                &mut stats.luk,
                &mut stats.acc,
                &mut stats.avoid,
            ] {
                shake(stat, by);
            }
        }
        Gem::RandOption(by) => {
            for stat in [
                &mut stats.watk,
                &mut stats.wdef,
                &mut stats.matk,
                &mut stats.mdef,
                &mut stats.speed,
                &mut stats.jump,
                &mut stats.hp,
                &mut stats.mp,
            ] {
                shake(stat, by);
            }
        }
    }
}

/// The stat of a gem's `inc` key.
fn gem_stat<'a>(stats: &'a mut EquipStats, name: &str) -> Option<&'a mut i16> {
    Some(match name {
        "PAD" => &mut stats.watk,
        "MAD" => &mut stats.matk,
        "ACC" => &mut stats.acc,
        "EVA" => &mut stats.avoid,
        "Speed" => &mut stats.speed,
        "Jump" => &mut stats.jump,
        "MaxHP" => &mut stats.hp,
        "MaxMP" => &mut stats.mp,
        "STR" => &mut stats.str,
        "INT" => &mut stats.int,
        "LUK" => &mut stats.luk,
        "DEX" => &mut stats.dex,
        _ => return None,
    })
}

/// What a stimulator does to the equipment it doesn't destroy: each stat
/// it has but speed gains up to 2, or 5 for defense, HP and MP (Cosmic's
/// `randomizeUpgradeStats`).
fn stimulate(stats: &mut EquipStats, roll: &mut impl FnMut(i32) -> i32) {
    let mut raise = |stat: &mut i16, most: i32| {
        if *stat > 0 {
            *stat = stat.saturating_add(roll(most + 1) as i16);
        }
    };
    for stat in [
        &mut stats.str,
        &mut stats.dex,
        &mut stats.int,
        &mut stats.luk,
        &mut stats.matk,
        &mut stats.watk,
        &mut stats.acc,
        &mut stats.avoid,
        &mut stats.jump,
    ] {
        raise(stat, 2);
    }
    for stat in [
        &mut stats.wdef,
        &mut stats.mdef,
        &mut stats.hp,
        &mut stats.mp,
    ] {
        raise(stat, 5);
    }
}

/// Take the equipment in the Equip tab's `slot`, which must be `item_id`
/// and made by a recipe (FUN_0082317f), apart into half the monster
/// crystals its recipe takes.
fn disassemble(player: &mut Player, item_id: i32, slot: i16) -> Result<Made, MakerRefusal> {
    use MakerRefusal::Failed;
    let recipe = items::maker().and_then(|maker| maker.recipe(item_id));
    let recipe = recipe.ok_or(Failed)?;
    let held = player.inventory.get(InventoryType::Equip, slot);
    let stats = held
        .filter(|item| item.item_id == item_id)
        .and_then(|item| item.equip.as_ref())
        .ok_or(Failed)?;
    let own = items::create(item_id, 1).and_then(|item| item.equip);
    let own = own.ok_or(Failed)?;
    if player.stats.hp <= 0 || maker_level(player) < 1 {
        return Err(Failed);
    }
    let gained = crystals_from(recipe);
    let meso = disassembly_fee(recipe.meso, stat_grade(stats, &own));
    if gained.is_empty() || player.meso < meso {
        return Err(Failed);
    }
    let mut next = player.inventory.clone();
    let mut changes = next
        .take(InventoryType::Equip, slot, 1)
        .map_err(|_| Failed)?;
    for &(crystal, count) in &gained {
        let count = i16::try_from(count).map_err(|_| Failed)?;
        changes
            .extend(items::give(&mut next, crystal, count).map_err(|why| no_room(why, crystal))?);
    }
    let result = build_maker_disassembled(item_id, &gained, meso).map_err(|_| Failed)?;
    *player.inventory_mut() = next;
    player.meso -= meso;
    Ok(Made {
        changes,
        meso,
        result,
        success: true,
    })
}

/// The crystals taking apart what `recipe` makes gives: half of each its
/// recipe takes (Cosmic).
fn crystals_from(recipe: &Recipe) -> Vec<(i32, i32)> {
    recipe
        .ingredients
        .iter()
        .filter(|(id, count)| CRYSTALS.contains(id) && count / 2 > 0)
        .map(|&(id, count)| (id, count / 2))
        .collect()
}

/// How far equipment's stats are above its `own` (FUN_005ca1b8): -1 when
/// below them, else 0 to 3. HP and MP count a tenth.
fn stat_grade(stats: &EquipStats, own: &EquipStats) -> i32 {
    let above = |stat: fn(&EquipStats) -> i16| i32::from(stat(stats)) - i32::from(stat(own));
    let score: i32 = [
        above(|s| s.str),
        above(|s| s.dex),
        above(|s| s.int),
        above(|s| s.luk),
        above(|s| s.hp) / 10,
        above(|s| s.mp) / 10,
        above(|s| s.watk),
        above(|s| s.matk),
        above(|s| s.wdef),
        above(|s| s.mdef),
        above(|s| s.acc),
        above(|s| s.avoid),
        above(|s| s.speed),
        above(|s| s.jump),
    ]
    .into_iter()
    .sum();
    match score {
        ..=-1 => -1,
        0..=5 => 0,
        6..=22 => 1,
        23..=39 => 2,
        _ => 3,
    }
}

/// The fee the window shows for taking apart what a recipe of `base`
/// mesos makes, of `grade` (FUN_00824f9d): a tenth of it, and half that
/// again for each grade above -1, rounded down to 1000.
pub fn disassembly_fee(base: i32, grade: i32) -> i32 {
    let fee = i64::from(base / 10) * i64::from(150 + 50 * grade) / 100;
    i32::try_from(fee - fee % 1000).unwrap_or(i32::MAX).max(0)
}

/// Make the monster crystal of `from`'s level from 100 of it.
fn crystal(player: &mut Player, from: i32) -> Result<Made, MakerRefusal> {
    use MakerRefusal::Failed;
    let crystal = items::maker().and_then(|maker| maker.crystal(from));
    let crystal = crystal.ok_or(Failed)?;
    if player.stats.hp <= 0 || maker_level(player) < 1 {
        return Err(Failed);
    }
    let mut next = player.inventory.clone();
    let mut changes = next.remove(from, CRYSTAL_DROPS).map_err(|_| Failed)?;
    changes.extend(items::give(&mut next, crystal, 1).map_err(|why| no_room(why, crystal))?);
    let result = build_maker_crystal(crystal, from).map_err(|_| Failed)?;
    *player.inventory_mut() = next;
    Ok(Made {
        changes,
        meso: 0,
        result,
        success: true,
    })
}

/// What the window checks before it lists a recipe (FUN_00822abb): the
/// level, the Maker's level, and the items held and worn and quests it
/// needs.
fn may_make(player: &Player, recipe: &Recipe) -> bool {
    let quests = recipe.req_quests.iter().all(|&(quest, want)| {
        let state =
            i16::try_from(quest).map_or(QuestState::NotStarted, |quest| player.quests.state(quest));
        match want {
            3 => state != QuestState::NotStarted,
            _ => state as i32 == want,
        }
    });
    let worn = |item_id| {
        player
            .inventory
            .items(InventoryType::Equipped)
            .any(|(_, item)| item.item_id == item_id)
    };
    player.stats.hp > 0
        && i32::from(player.stats.level) >= recipe.req_level
        && maker_level(player) >= recipe.req_skill_level.max(1)
        && (recipe.req_item == 0 || player.inventory.count(recipe.req_item) > 0)
        && (recipe.req_equip == 0 || worn(recipe.req_equip))
        && quests
}

/// The level of the player's Maker skill: 1007 for explorers, 10001007
/// for Cygnus Knights, 20001007 for Aran and 20011007 for Evan.
fn maker_level(player: &Player) -> i32 {
    let job = i32::from(player.stats.job);
    let base = if job / 100 == 22 || job == 2001 {
        2001
    } else {
        job / 1000 * 1000
    };
    player.skills.level(base * 10_000 + 1007)
}

/// The fee the window shows for a recipe of `base` mesos (FUN_00824e8d):
/// more for a stimulator and for each gem, rounded down to 1000.
pub fn fee(base: i32, stimulator: bool, gems: &[i32]) -> i32 {
    let base = i64::from(base);
    let gems: i64 = gems.iter().map(|gem| 1 + i64::from(gem % 10)).sum();
    let extra = ((if stimulator { 5 } else { 0 } + 1 + gems * 3) * (base / 10)).max(0);
    let total = base + extra;
    i32::try_from(total - total % 1000).unwrap_or(i32::MAX)
}

/// What a recipe makes: for a gem, one of its rewards by weight.
fn outcome(recipe: &Recipe, roll: &mut impl FnMut(i32) -> i32) -> (i32, i32) {
    let total: i32 = recipe.rewards.iter().map(|reward| reward.weight).sum();
    if total <= 0 {
        return (recipe.item_id, recipe.count);
    }
    let mut left = roll(total);
    for reward in &recipe.rewards {
        if left < reward.weight {
            return (reward.item_id, reward.count);
        }
        left -= reward.weight;
    }
    (recipe.item_id, recipe.count)
}

/// Why `item_id` couldn't be given: no room in its tab, or anything else.
fn no_room(why: InventoryError, item_id: i32) -> MakerRefusal {
    match (why, InventoryType::for_item(item_id)) {
        (InventoryError::Full, Some(kind)) => MakerRefusal::NoRoom(kind),
        _ => MakerRefusal::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::WorldAction;
    use crate::packet::op::{RecvOpcode, SendOpcode};
    use game::quest::{QuestRecord, QuestStatus};
    use game::skill::SkillRecord;
    use game::Item;
    use packet::io::write::PktWrite;

    /// Basic, Intermediate and Advanced Diamonds, and the Diamond Ore.
    const BASIC: i32 = 4_250_000;
    const INTERMEDIATE: i32 = 4_250_001;
    const ADVANCED: i32 = 4_250_002;
    const ORE: i32 = 4_021_007;
    /// A level-35 drop, and the crystal for levels 31 to 50.
    const DROP: i32 = 4_000_013;
    const CRYSTAL: i32 = 4_260_000;

    fn maker(level: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 50;
        player.meso = 1_000_000;
        if level > 0 {
            player.skills.set(SkillRecord {
                skill_id: 1007,
                level,
                master_level: 0,
                expiration: None,
            });
        }
        player
            .inventory
            .insert(InventoryType::Etc, 1, Item::stack(ORE, 3));
        player
    }

    fn request(player: &mut Player, body: &[i32], flag: Option<u8>) -> Vec<Vec<u8>> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::ItemMake as i16).unwrap();
        for (i, value) in body.iter().enumerate() {
            p.write_int(*value).unwrap();
            if i == 1 {
                if let Some(flag) = flag {
                    p.write_byte(flag).unwrap();
                }
            }
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        ItemMakeHandler
            .handle(&mut p, &mut ctx)
            .unwrap()
            .actions
            .into_iter()
            .map(|action| match action {
                WorldAction::Reply(p) => p.bytes,
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    fn refusal(code: i32) -> Vec<u8> {
        let mut bytes = (SendOpcode::MakerResult as i16).to_le_bytes().to_vec();
        bytes.extend(code.to_le_bytes());
        bytes
    }

    fn opcode(bytes: &[u8]) -> i16 {
        i16::from_le_bytes([bytes[0], bytes[1]])
    }

    #[test]
    fn the_fee_is_what_the_window_shows() {
        assert_eq!(fee(100_000, false, &[]), 110_000);
        assert_eq!(fee(55_000, false, &[]), 60_000);
        // One tenth more, five for a stimulator, and three for each grade
        // of each gem: a Basic Garnet 3 and an Advanced Diamond 9.
        assert_eq!(
            fee(60_000, true, &[4_250_100, 4_250_002]),
            60_000 + 18 * 6_000
        );
        assert_eq!(fee(0, false, &[]), 0);
    }

    #[test]
    fn a_gem_recipe_makes_one_of_its_rewards_by_weight() {
        let recipe = items::maker().unwrap().recipe(BASIC).unwrap();
        let at = |n| {
            outcome(recipe, &mut |total| {
                assert_eq!(total, 20);
                n
            })
        };
        assert_eq!(
            [at(0), at(13), at(14), at(18), at(19)],
            [
                (BASIC, 1),
                (BASIC, 1),
                (INTERMEDIATE, 1),
                (INTERMEDIATE, 1),
                (ADVANCED, 1)
            ]
        );
    }

    #[test]
    fn making_a_gem_takes_the_ore_and_the_fee() {
        let mut player = maker(1);
        let made = make(&mut player, BASIC, false, &[], &mut |_| 15).unwrap();
        assert_eq!(made.meso, 110_000);
        assert_eq!(player.meso, 890_000);
        assert_eq!(player.inventory.count(ORE), 2);
        assert_eq!(player.inventory.count(INTERMEDIATE), 1);
        let mut expected = refusal(0);
        expected.extend(1i32.to_le_bytes());
        expected.push(0);
        for value in [INTERMEDIATE, 1, 1, ORE, 1, 0] {
            expected.extend(value.to_le_bytes());
        }
        expected.push(0);
        expected.extend(110_000i32.to_le_bytes());
        assert_eq!(made.result.bytes, expected);
    }

    #[test]
    fn a_made_gem_answers_with_the_meso_items_result_and_glow() {
        let mut player = maker(1);
        let replies = request(&mut player, &[1, BASIC, 0], Some(0));
        let opcodes: Vec<_> = replies.iter().map(|bytes| opcode(bytes)).collect();
        assert_eq!(
            opcodes,
            [
                SendOpcode::StatChange as i16,
                SendOpcode::InventoryOperation as i16,
                SendOpcode::MakerResult as i16,
                SendOpcode::ShowItemGainInChat as i16,
            ]
        );
        assert_eq!(replies[3][2..], [0x10, 0, 0, 0, 0]);
    }

    #[test]
    fn what_the_maker_may_not_make_still_gets_an_answer() {
        // No Maker skill, too low a level, too few mesos, no ore, a
        // stimulator, equipment, the random make and disassembly.
        let failed = vec![refusal(2)];
        assert_eq!(request(&mut maker(0), &[1, BASIC, 0], Some(0)), failed);
        let mut low = maker(1);
        low.stats.level = 30;
        assert_eq!(request(&mut low, &[1, BASIC, 0], Some(0)), failed);
        let mut poor = maker(1);
        poor.meso = 109_999;
        assert_eq!(request(&mut poor, &[1, BASIC, 0], Some(0)), failed);
        let mut player = maker(1);
        player.inventory.take(InventoryType::Etc, 1, 3).unwrap();
        assert_eq!(request(&mut player, &[1, BASIC, 0], Some(0)), failed);
        let mut player = maker(1);
        assert_eq!(request(&mut player, &[1, BASIC, 0], Some(1)), failed);
        assert_eq!(request(&mut player, &[1, 1_302_010, 0], Some(0)), failed);
        assert_eq!(request(&mut player, &[2, BASIC, 0], Some(0)), failed);
        assert_eq!(request(&mut player, &[4, 1_302_010, 1, 1], None), failed);
        assert_eq!((player.meso, player.inventory.count(ORE)), (1_000_000, 3));
    }

    #[test]
    fn a_full_tab_is_named() {
        let mut player = maker(1);
        let limit = player.inventory.slot_limit(InventoryType::Etc) as i16;
        for slot in 2..=limit {
            player.inventory.insert(
                InventoryType::Etc,
                slot,
                Item::stack(4_000_000 + i32::from(slot), 1),
            );
        }
        assert_eq!(request(&mut player, &[1, BASIC, 0], Some(0)), [refusal(7)]);
        assert_eq!(player.inventory.count(ORE), 3);
    }

    #[test]
    fn a_hundred_drops_make_a_crystal() {
        let mut player = maker(1);
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(DROP, 99));
        assert_eq!(request(&mut player, &[3, DROP], None), [refusal(2)]);
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(DROP, 150));
        let replies = request(&mut player, &[3, DROP], None);
        let mut crystal = refusal(0);
        for value in [3, CRYSTAL, DROP] {
            crystal.extend(value.to_le_bytes());
        }
        assert_eq!(replies[1], crystal);
        assert_eq!(player.inventory.count(DROP), 50);
        assert_eq!(player.inventory.count(CRYSTAL), 1);
        assert_eq!(player.meso, 1_000_000);
        // An ore has no level.
        assert_eq!(request(&mut player, &[3, ORE], None), [refusal(2)]);
        // Nor without the Maker.
        let mut player = maker(0);
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(DROP, 100));
        assert_eq!(request(&mut player, &[3, DROP], None), [refusal(2)]);
        assert_eq!(player.inventory.count(DROP), 100);
    }

    /// A level-45 sword whose recipe takes an ingot, 20 crystals and a
    /// stimulator, with one gem slot; and an attack gem and a jump gem.
    const SWORD: i32 = 1_302_010;
    const INGOT: i32 = 4_011_000;
    const STIMULATOR: i32 = 4_130_002;
    const PAD_GEM: i32 = 4_250_001;
    const JUMP_GEM: i32 = 4_250_502;

    fn smith() -> Player {
        let mut player = maker(1);
        for (slot, (id, count)) in [(INGOT, 1), (CRYSTAL, 20), (STIMULATOR, 1), (PAD_GEM, 1)]
            .into_iter()
            .enumerate()
        {
            player.inventory.insert(
                InventoryType::for_item(id).unwrap(),
                slot as i16 + 2,
                Item::stack(id, count),
            );
        }
        player
            .inventory
            .insert(InventoryType::Etc, 6, Item::stack(JUMP_GEM, 1));
        player
    }

    fn sword(player: &Player) -> Option<EquipStats> {
        player
            .inventory
            .items(InventoryType::Equip)
            .find(|(_, item)| item.item_id == SWORD)
            .and_then(|(_, item)| item.equip.clone())
    }

    fn base_sword() -> EquipStats {
        items::create(SWORD, 1).unwrap().equip.unwrap()
    }

    #[test]
    fn a_sword_takes_its_gem_and_stimulator() {
        let mut player = smith();
        // The stimulator holds, and raises each stat by all it may.
        let made = make(&mut player, SWORD, true, &[PAD_GEM], &mut |total| total - 1).unwrap();
        assert!(made.success);
        let meso = fee(60_000, true, &[PAD_GEM]);
        assert_eq!(made.meso, meso);
        assert_eq!(player.meso, 1_000_000 - meso);
        let base = base_sword();
        let stats = sword(&player).unwrap();
        assert_eq!(stats.watk, base.watk + 2 + 2);
        assert_eq!(stats.speed, base.speed);
        for id in [INGOT, CRYSTAL, STIMULATOR, PAD_GEM] {
            assert_eq!(player.inventory.count(id), 0, "{id}");
        }
        let mut expected = refusal(0);
        expected.extend(1i32.to_le_bytes());
        expected.push(0);
        for value in [SWORD, 1, 2, INGOT, 1, CRYSTAL, 20, 1, PAD_GEM] {
            expected.extend(value.to_le_bytes());
        }
        expected.push(1);
        expected.extend(STIMULATOR.to_le_bytes());
        expected.extend(meso.to_le_bytes());
        assert_eq!(made.result.bytes, expected);
    }

    #[test]
    fn without_a_stimulator_a_sword_has_its_own_stats() {
        let mut player = smith();
        let made = make(&mut player, SWORD, false, &[], &mut |_| unreachable!()).unwrap();
        assert_eq!(made.meso, 60_000 + 6_000);
        assert_eq!(sword(&player), Some(base_sword()));
        assert_eq!(player.inventory.count(STIMULATOR), 1);
    }

    #[test]
    fn a_stimulator_may_destroy_the_sword_and_all_it_took() {
        let mut player = smith();
        let made = make(&mut player, SWORD, true, &[PAD_GEM], &mut |_| 0).unwrap();
        assert!(!made.success);
        assert_eq!(sword(&player), None);
        for id in [INGOT, CRYSTAL, STIMULATOR, PAD_GEM] {
            assert_eq!(player.inventory.count(id), 0, "{id}");
        }
        let meso = fee(60_000, true, &[PAD_GEM]);
        assert_eq!(player.meso, 1_000_000 - meso);
        // Destroyed: no item, then what it cost.
        let mut expected = refusal(1);
        expected.extend(1i32.to_le_bytes());
        expected.push(1);
        for value in [2, INGOT, 1, CRYSTAL, 20, 1, PAD_GEM] {
            expected.extend(value.to_le_bytes());
        }
        expected.push(1);
        expected.extend(STIMULATOR.to_le_bytes());
        expected.extend(meso.to_le_bytes());
        assert_eq!(made.result.bytes, expected);
        let replies = request(&mut smith(), &[1, SWORD, 1, PAD_GEM], Some(1));
        assert_eq!(replies.len(), 4);
    }

    #[test]
    fn gems_go_in_as_the_window_lets_them() {
        let maker = items::maker().unwrap();
        let sword = maker.recipe(SWORD).unwrap();
        assert!(gems_fit(sword, &[PAD_GEM]));
        assert!(gems_fit(sword, &[JUMP_GEM]));
        assert!(!gems_fit(sword, &[PAD_GEM, JUMP_GEM]), "one slot");
        assert!(!gems_fit(sword, &[INGOT]), "not a gem");
        let mut two = sword.clone();
        two.gem_slots = 2;
        assert!(gems_fit(&two, &[PAD_GEM, JUMP_GEM]));
        assert!(!gems_fit(&two, &[PAD_GEM, 4_250_000]), "one of a kind");
        let mut hat = two.clone();
        hat.item_id = 1_002_022;
        assert!(!gems_fit(&hat, &[PAD_GEM]), "attack on a hat");
        assert!(gems_fit(&hat, &[JUMP_GEM]));
        // Nor a stimulator the recipe has none of, nor gems it has no
        // slots for, nor ones not held.
        let mut player = smith();
        assert!(make(&mut player, BASIC, true, &[], &mut |_| 1).is_err());
        assert!(make(&mut player, BASIC, false, &[JUMP_GEM], &mut |_| 1).is_err());
        player.inventory.take(InventoryType::Etc, 6, 1).unwrap();
        assert!(make(&mut player, SWORD, false, &[JUMP_GEM], &mut |_| 1).is_err());
        assert_eq!(player.inventory.count(INGOT), 1);
    }

    #[test]
    fn the_random_gems_shake_what_the_item_has() {
        let mut stats = EquipStats {
            str: 3,
            watk: 10,
            jump: 0,
            ..EquipStats::default()
        };
        put_gem(&mut stats, Gem::RandStat(5), &mut |total| total - 1);
        put_gem(&mut stats, Gem::RandOption(2), &mut |_| 0);
        assert_eq!((stats.str, stats.watk, stats.jump), (8, 8, 0));
        put_gem(&mut stats, Gem::Stat("Jump", 3), &mut |_| unreachable!());
        put_gem(&mut stats, Gem::ReqLevel(3), &mut |_| unreachable!());
        assert_eq!(stats.jump, 3);
    }

    #[test]
    fn a_sword_comes_apart_into_half_its_crystals() {
        let mut player = maker(1);
        player
            .inventory
            .insert(InventoryType::Equip, 3, items::create(SWORD, 1).unwrap());
        // Not what is in that slot, nor another tab.
        assert_eq!(request(&mut player, &[4, SWORD, 1, 2], None), [refusal(2)]);
        assert_eq!(request(&mut player, &[4, SWORD, 4, 3], None), [refusal(2)]);
        let replies = request(&mut player, &[4, SWORD, 1, 3], None);
        let meso = disassembly_fee(60_000, 0);
        assert_eq!(meso, 9_000);
        let mut expected = refusal(0);
        for value in [4, SWORD, 1, CRYSTAL, 10, meso] {
            expected.extend(value.to_le_bytes());
        }
        assert_eq!(replies[2], expected);
        assert_eq!(sword(&player), None);
        assert_eq!(player.inventory.count(CRYSTAL), 10);
        assert_eq!(player.meso, 1_000_000 - meso);
        // What has no recipe doesn't.
        player.inventory.insert(
            InventoryType::Equip,
            3,
            items::create(1_302_000, 1).unwrap(),
        );
        assert_eq!(
            request(&mut player, &[4, 1_302_000, 1, 3], None),
            [refusal(2)]
        );
    }

    #[test]
    fn the_better_the_sword_the_more_it_costs_to_take_apart() {
        let own = base_sword();
        let mut stats = own.clone();
        assert_eq!(stat_grade(&stats, &own), 0);
        stats.watk -= 1;
        assert_eq!(stat_grade(&stats, &own), -1);
        stats.watk += 7;
        stats.hp += 19;
        assert_eq!(stat_grade(&stats, &own), 1);
        stats.str += 17;
        assert_eq!(stat_grade(&stats, &own), 2);
        stats.dex += 17;
        assert_eq!(stat_grade(&stats, &own), 3);
        let fees: Vec<_> = (-1..=3)
            .map(|grade| disassembly_fee(60_000, grade))
            .collect();
        assert_eq!(fees, [6_000, 9_000, 12_000, 15_000, 18_000]);
        assert_eq!(disassembly_fee(66_500, 1), 13_000);

        let mut player = maker(1);
        let mut sword = items::create(SWORD, 1).unwrap();
        sword.equip.as_mut().unwrap().watk += 6;
        player.inventory.insert(InventoryType::Equip, 3, sword);
        request(&mut player, &[4, SWORD, 1, 3], None);
        assert_eq!(player.meso, 1_000_000 - 12_000);
    }

    #[test]
    fn a_cut_short_request_still_gets_an_answer() {
        let mut player = maker(1);
        assert_eq!(request(&mut player, &[1], None), [refusal(2)]);
        assert_eq!(request(&mut player, &[1, BASIC], Some(0)), [refusal(2)]);
        assert_eq!(player.inventory.count(ORE), 3);
    }

    #[test]
    fn a_recipe_wants_its_item_equipment_and_quests() {
        let mut player = maker(1);
        let recipe = |edit: fn(&mut Recipe)| {
            let mut recipe = Recipe {
                item_id: BASIC,
                req_skill_level: 1,
                ..Recipe::default()
            };
            edit(&mut recipe);
            recipe
        };
        assert!(may_make(&player, &recipe(|_| ())));

        let held = recipe(|r| r.req_item = 4_031_966);
        assert!(!may_make(&player, &held));
        player
            .inventory
            .insert(InventoryType::Etc, 2, Item::stack(4_031_966, 1));
        assert!(may_make(&player, &held));

        let worn = recipe(|r| r.req_equip = ORE);
        assert!(!may_make(&player, &worn), "held is not worn");

        let started = recipe(|r| r.req_quests = vec![(21_303, 1)]);
        let done = recipe(|r| r.req_quests = vec![(21_303, 2)]);
        let either = recipe(|r| r.req_quests = vec![(21_303, 3)]);
        assert!(![&started, &done, &either]
            .iter()
            .any(|r| may_make(&player, r)));
        player.quests.upsert(QuestRecord {
            quest_id: 21_303,
            status: QuestStatus::Started { kills: Vec::new() },
        });
        assert_eq!(
            [&started, &done, &either].map(|r| may_make(&player, r)),
            [true, false, true]
        );
        player.quests.upsert(QuestRecord {
            quest_id: 21_303,
            status: QuestStatus::Completed {
                at: std::time::SystemTime::UNIX_EPOCH,
            },
        });
        assert_eq!(
            [&started, &done, &either].map(|r| may_make(&player, r)),
            [false, true, true]
        );

        player.stats.hp = 0;
        assert!(!may_make(&player, &recipe(|_| ())));
    }

    #[test]
    fn each_branch_has_its_own_maker() {
        for (job, skill) in [
            (0, 1007),
            (412, 1007),
            (1100, 10_001_007),
            (2000, 20_001_007),
            (2112, 20_001_007),
            (2001, 20_011_007),
            (2218, 20_011_007),
        ] {
            let mut player = maker(0);
            player.stats.job = job;
            player.skills.set(SkillRecord {
                skill_id: skill,
                level: 2,
                master_level: 0,
                expiration: None,
            });
            assert_eq!(maker_level(&player), 2, "job {job}");
        }
    }
}
