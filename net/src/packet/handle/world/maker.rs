//! ITEM_MAKE (0x71, v83 FUN_00827096): the Item Maker's Start, sent once
//! the window's gauge is full (reports c006 and c006b). Its mode says what
//! to make: 1 a recipe, 2 the "???" random make, 3 a monster crystal, 4 a
//! disassembly. The window checks everything first but sends no price, so
//! the recipe and the fee come from the data here.
//!
//! So far the Maker makes what has no stimulator or gem slots (gems,
//! scrolls, order forms) and monster crystals; equipment, the random make
//! and disassembly fail. The window waits for an answer to every request.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use crate::items;
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::maker::{
    build_maker_crystal, build_maker_effect, build_maker_made, build_maker_refusal, MakerRefusal,
};
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use game::quest::QuestState;
use game::{InventoryChange, InventoryError, InventoryType, Player};
use game_data::maker::Recipe;
use packet::{io::read::PktRead, Packet};
use rand::Rng;
use std::io::Cursor;

/// How many drops one monster crystal takes.
const CRYSTAL_DROPS: i16 = 100;

pub struct ItemMakeHandler;

impl WorldHandler for ItemMakeHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let made = request(packet).and_then(|request| match request {
            Request::Make(item_id) => make(ctx.player, item_id, &mut |total| {
                rand::thread_rng().gen_range(0, total)
            }),
            Request::Crystal(item_id) => crystal(ctx.player, item_id),
        });
        let made = match made {
            Ok(made) => made,
            Err(why) => return Ok(HandlerResult::reply(build_maker_refusal(why)?)),
        };
        ctx.player.mark_dirty();
        let mut result = HandlerResult::empty();
        if made.meso > 0 {
            result = result.with_reply(build_stat_update(
                true,
                &[StatUpdate::Meso(ctx.player.meso)],
            )?);
        }
        Ok(result
            .with_reply(build_inventory_operation(true, &made.changes)?)
            .with_reply(made.result)
            .with_reply(build_maker_effect(true)?))
    }
}

/// What the Maker can be asked so far.
enum Request {
    /// A recipe with no stimulator and no gems.
    Make(i32),
    Crystal(i32),
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
            1 if r.read_byte()? == 0 && r.read_int()? == 0 => Request::Make(item_id),
            3 => Request::Crystal(item_id),
            _ => return Err(std::io::ErrorKind::Unsupported.into()),
        })
    };
    read().map_err(|_| MakerRefusal::Failed)
}

/// What a request made: the inventory changes, the mesos it cost, and the
/// MAKER_RESULT that tells the window.
struct Made {
    changes: Vec<InventoryChange>,
    meso: i32,
    result: Packet,
}

/// Make `item_id` from its recipe; `roll(total)` picks a number below
/// `total` for a gem recipe's outcome. Nothing changes unless it works.
fn make(
    player: &mut Player,
    item_id: i32,
    roll: &mut impl FnMut(i32) -> i32,
) -> Result<Made, MakerRefusal> {
    use MakerRefusal::Failed;
    let recipe = items::maker().and_then(|maker| maker.recipe(item_id));
    let recipe = recipe.ok_or(Failed)?;
    // Equipment takes a stimulator and gems.
    if recipe.catalyst != 0 || recipe.gem_slots != 0 || !may_make(player, recipe) {
        return Err(Failed);
    }
    let meso = fee(recipe.meso, false, &[]);
    if player.meso < meso {
        return Err(Failed);
    }
    let (made, count) = outcome(recipe, roll);
    let count = i16::try_from(count).map_err(|_| Failed)?;
    if items::refuses_only(&player.inventory, made, i32::from(count)) {
        return Err(Failed);
    }
    let mut next = player.inventory.clone();
    let mut changes = Vec::new();
    for &(ingredient, needed) in &recipe.ingredients {
        let needed = i16::try_from(needed).map_err(|_| Failed)?;
        changes.extend(next.remove(ingredient, needed).map_err(|_| Failed)?);
    }
    changes.extend(items::give(&mut next, made, count).map_err(|why| no_room(why, made))?);
    let result =
        build_maker_made(made, i32::from(count), &recipe.ingredients, meso).map_err(|_| Failed)?;
    *player.inventory_mut() = next;
    player.meso -= meso;
    Ok(Made {
        changes,
        meso,
        result,
    })
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
        let made = make(&mut player, BASIC, &mut |_| 15).unwrap();
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
