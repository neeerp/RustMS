//! The v83 quest flow. The client renders the conversation from `Say.img`
//! and sends `QUEST_ACTION`; the server checks the request against the
//! catalog and the player's state before anything is persisted.

use crate::load;
use game::{InventoryChange, InventoryError, Player, QuestState, QuestStatus, SkillRecord};
pub use game_data::quests::{
    QuestCatalog, QuestItem, QuestPrerequisite, QuestSkill, QuestTemplate, StartRequirements,
};
use rand::Rng;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

pub fn catalog() -> Result<&'static QuestCatalog, &'static str> {
    static CATALOG: OnceLock<Result<QuestCatalog, String>> = OnceLock::new();
    load::once(&CATALOG, || {
        QuestCatalog::load_from_nx(load::asset_path(
            "RUSTMS_QUEST_NX_PATH",
            "assets/game-data/Quest.nx",
        ))
    })
}

/// A parsed `QUEST_ACTION`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestRequest {
    pub quest_id: i16,
    pub kind: QuestActionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestActionKind {
    Start {
        npc_id: i32,
    },
    /// `choice` is the picked reward's place among those offered, or -1.
    Complete {
        npc_id: i32,
        choice: i32,
    },
    Forfeit,
    ScriptStart {
        npc_id: i32,
    },
    ScriptEnd {
        npc_id: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestRejection {
    WrongNpc,
    WrongState,
    Job,
    Level,
    Prerequisite(i16),
    /// Missing (or, for a count of 0, still holding) a required item.
    Items(i32),
    /// Fewer kills of this mob than required.
    Mobs(i32),
    /// Lacking, or for a "must not have" entry having, this skill.
    Skill(i32),
    /// Less fame than the quest asks.
    Fame,
    /// Fewer mesos than the quest takes.
    Mesos,
    /// The item rewards don't fit.
    InventoryFull,
    /// The picked reward isn't one of those offered.
    Choice,
}

/// Whether `player` may start `quest` with `npc_id`.
pub fn check_start(
    quest: &QuestTemplate,
    npc_id: i32,
    player: &Player,
) -> Result<(), QuestRejection> {
    let start = &quest.start;
    if start.npc != npc_id {
        return Err(QuestRejection::WrongNpc);
    }
    match player.quests.status(quest.id) {
        None => {}
        // A repeatable quest starts again `interval` minutes after it was
        // last completed (v83 FUN_00721163).
        Some(QuestStatus::Completed { at })
            if start.repeat_interval.is_some_and(|minutes| {
                *at + Duration::from_secs(u64::from(minutes) * 60) <= SystemTime::now()
            }) => {}
        Some(_) => return Err(QuestRejection::WrongState),
    }
    if !start.jobs.is_empty() && !start.jobs.contains(&player.stats.job) {
        return Err(QuestRejection::Job);
    }
    let level = i16::from(player.stats.level);
    if start.min_level.is_some_and(|min| level < min)
        || start.max_level.is_some_and(|max| level > max)
    {
        return Err(QuestRejection::Level);
    }
    if let Some(unmet) = start
        .quests
        .iter()
        .find(|p| player.quests.state(p.quest_id) != p.state)
    {
        return Err(QuestRejection::Prerequisite(unmet.quest_id));
    }
    // v83 compares fame unsigned, so negative fame passes.
    if start.min_fame != 0 && (player.stats.fame as u16) < (start.min_fame as u16) {
        return Err(QuestRejection::Fame);
    }
    for &(item_id, count) in &start.items {
        let held = held_for_start(player, item_id);
        let met = match count {
            1.. => held >= count,
            0 => held == 0,
            _ => held <= count.saturating_neg(),
        };
        if !met {
            return Err(QuestRejection::Items(item_id));
        }
    }
    if let Some(&(skill_id, _)) = start
        .skills
        .iter()
        .find(|&&(id, acquire)| has_skill(player, id) != acquire)
    {
        return Err(QuestRejection::Skill(skill_id));
    }
    Ok(())
}

/// How many of `item_id` v83's start check sees `player` hold: those in the
/// bag, plus one if a copy is worn.
fn held_for_start(player: &Player, item_id: i32) -> i32 {
    let worn = player.inventory.equipped_ids().any(|(_, id)| id == item_id);
    player.inventory.count(item_id) + i32::from(worn)
}

/// Whether v83's start check counts `player` as having `skill_id`: a skill
/// they've learned, or any skill that isn't hidden from the skill window.
/// It looks at neither the level nor the job.
fn has_skill(player: &Player, skill_id: i32) -> bool {
    crate::skills::info(skill_id)
        .is_some_and(|info| !info.invisible || player.skills.get(skill_id).is_some())
}

/// Whether `player` may complete `quest` with `npc_id`. The start
/// requirements, including the level range, only apply to starting.
pub fn check_complete(
    quest: &QuestTemplate,
    npc_id: i32,
    player: &Player,
) -> Result<(), QuestRejection> {
    if quest.end_npc != npc_id {
        return Err(QuestRejection::WrongNpc);
    }
    if player.quests.state(quest.id) != QuestState::Started {
        return Err(QuestRejection::WrongState);
    }
    for &(item_id, count) in &quest.end_items {
        let held = player.inventory.count(item_id);
        let met = if count > 0 { held >= count } else { held == 0 };
        if !met {
            return Err(QuestRejection::Items(item_id));
        }
    }
    let kills = player.quests.kills(quest.id);
    for (index, &(mob_id, count)) in quest.end_mobs.iter().enumerate() {
        if kills.get(index).copied().unwrap_or(0) < count {
            return Err(QuestRejection::Mobs(mob_id));
        }
    }
    if let Some(unmet) = quest
        .end_quests
        .iter()
        .find(|p| player.quests.state(p.quest_id) != p.state)
    {
        return Err(QuestRejection::Prerequisite(unmet.quest_id));
    }
    if quest.money < 0 && player.meso < quest.money.saturating_neg() {
        return Err(QuestRejection::Mesos);
    }
    Ok(())
}

/// Count a kill of `mob_id` toward every started quest that needs it.
/// Returns the quests whose progress changed.
pub fn count_kill<'a>(
    quest: impl Fn(i16) -> Option<&'a QuestTemplate>,
    player: &mut Player,
    mob_id: i32,
) -> Vec<i16> {
    let started: Vec<i16> = player.quests.started().map(|r| r.quest_id).collect();
    let mut changed = Vec::new();
    for quest_id in started {
        let Some(quest) = quest(quest_id) else {
            continue;
        };
        if !quest
            .end_mobs
            .iter()
            .any(|&(required, _)| required == mob_id)
        {
            continue;
        }
        let Some(kills) = player.quests.kills_mut(quest_id) else {
            continue;
        };
        kills.resize(quest.end_mobs.len(), 0);
        let mut counted = false;
        for (index, &(required, count)) in quest.end_mobs.iter().enumerate() {
            // Kills past the required count are not recorded.
            if required == mob_id && kills[index] < count {
                kills[index] += 1;
                counted = true;
            }
        }
        if counted {
            changed.push(quest_id);
        }
    }
    changed
}

/// The item changes a quest step makes for `player`, as `(item id, count)`:
/// every entry for their job and gender without a `prop`, plus one pick
/// among those with a positive one, weighted by it, plus the `choice`th of
/// those with -1 that give something, if there are any (v83 numbers the
/// choices it shows the same way, FUN_0071808a). An entry without a count
/// takes all the player holds, and is left out if that's none.
pub fn pick_items(
    items: &[QuestItem],
    player: &Player,
    choice: i32,
    rng: &mut impl Rng,
) -> Result<Vec<(i32, i32)>, QuestRejection> {
    let eligible: Vec<&QuestItem> = items
        .iter()
        .filter(|item| item.applies_to(player.stats.job, player.stats.gender))
        .collect();
    let choices: Vec<&&QuestItem> = eligible
        .iter()
        .filter(|item| item.prop.is_some_and(|p| p < 0) && item.count > 0)
        .collect();
    let mut picked: Vec<(i32, i32)> = eligible
        .iter()
        .filter(|item| item.prop.is_none())
        .map(|item| match item.count {
            0 => (item.item_id, -player.inventory.count(item.item_id)),
            count => (item.item_id, count),
        })
        .filter(|&(_, count)| count != 0)
        .collect();
    let weighted: Vec<&&QuestItem> = eligible
        .iter()
        .filter(|item| item.prop.is_some_and(|w| w > 0))
        .collect();
    let total: i32 = weighted.iter().filter_map(|item| item.prop).sum();
    if total > 0 {
        let mut roll = rng.gen_range(0, total);
        for item in weighted {
            let weight = item.prop.unwrap_or(0);
            if roll < weight {
                picked.push((item.item_id, item.count));
                break;
            }
            roll -= weight;
        }
    }
    if !choices.is_empty() {
        let chosen = usize::try_from(choice)
            .ok()
            .and_then(|index| choices.get(index))
            .ok_or(QuestRejection::Choice)?;
        picked.push((chosen.item_id, chosen.count));
    }
    Ok(picked)
}

/// Take, then give, `items` (negative counts take). Takes are capped at
/// what the player holds. Fails without a partial result if a gift doesn't
/// fit; the caller applies this to a copy of the player.
pub fn apply_items(
    player: &mut Player,
    items: &[(i32, i32)],
) -> Result<Vec<InventoryChange>, QuestRejection> {
    let mut changes = Vec::new();
    for &(item_id, count) in items.iter().filter(|(_, count)| *count < 0) {
        let held = player.inventory.count(item_id);
        let taken = i16::try_from(held.min(-count)).unwrap_or(i16::MAX);
        if taken > 0 {
            changes.extend(
                player
                    .inventory
                    .remove(item_id, taken)
                    .map_err(|_| QuestRejection::Items(item_id))?,
            );
        }
    }
    for &(item_id, count) in items.iter().filter(|(_, count)| *count > 0) {
        let quantity = i16::try_from(count).unwrap_or(i16::MAX);
        let given = crate::items::give(&mut player.inventory, item_id, quantity);
        changes.extend(given.map_err(|error| match error {
            InventoryError::UnknownItem => QuestRejection::Items(item_id),
            _ => QuestRejection::InventoryFull,
        })?);
    }
    Ok(changes)
}

/// Teach `player` the skills for their exact job, raising the level and the
/// master level to the entry's where they are lower (as Cosmic's
/// SkillAction does). The records that changed, for UPDATE_SKILLS.
pub fn apply_skills(skills: &[QuestSkill], player: &mut Player) -> Vec<SkillRecord> {
    let mut taught = Vec::new();
    for skill in skills
        .iter()
        .filter(|skill| skill.jobs.contains(&player.stats.job))
    {
        let known = player.skills.get(skill.skill_id).cloned();
        let record = SkillRecord {
            skill_id: skill.skill_id,
            level: skill.level.max(known.as_ref().map_or(0, |k| k.level)),
            master_level: skill
                .master_level
                .max(known.as_ref().map_or(0, |k| k.master_level)),
            expiration: known.as_ref().and_then(|k| k.expiration),
        };
        if known.as_ref() != Some(&record) {
            player.skills.set(record.clone());
            taught.push(record);
        }
    }
    taught
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::QuestRecord;

    fn quest() -> QuestTemplate {
        QuestTemplate {
            id: 1033,
            name: "What Sen Wants".into(),
            start: StartRequirements {
                npc: 2001,
                jobs: vec![0],
                min_level: None,
                max_level: Some(10),
                quests: vec![QuestPrerequisite {
                    quest_id: 1032,
                    state: QuestState::Completed,
                }],
                ..StartRequirements::default()
            },
            end_npc: 2102,
            end_items: vec![],
            end_mobs: vec![],
            end_quests: vec![],
            start_items: vec![],
            end_rewards: vec![],
            end_skills: vec![],
            start_script: None,
            end_script: None,
            exp: 7,
            money: 0,
            fame: 0,
            next_quest: None,
            auto_start: false,
            auto_complete: false,
        }
    }

    fn player(level: u8, job: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        player.stats.job = job;
        player
    }

    fn set(player: &mut Player, quest_id: i16, state: QuestState) {
        let status = match state {
            QuestState::NotStarted => return player.quests.remove(quest_id),
            QuestState::Started => QuestStatus::Started { kills: Vec::new() },
            QuestState::Completed => QuestStatus::Completed {
                at: SystemTime::UNIX_EPOCH,
            },
        };
        player.quests.upsert(QuestRecord { quest_id, status });
    }

    #[test]
    fn start_checks_npc_state_job_level_and_prerequisites() {
        let q = quest();
        let mut p = player(1, 0);
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Prerequisite(1032))
        );
        set(&mut p, 1032, QuestState::Completed);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        assert_eq!(check_start(&q, 2102, &p), Err(QuestRejection::WrongNpc));
        assert_eq!(
            check_start(&q, 2001, &player(11, 0)),
            Err(QuestRejection::Level)
        );
        let mut warrior = player(1, 100);
        set(&mut warrior, 1032, QuestState::Completed);
        assert_eq!(check_start(&q, 2001, &warrior), Err(QuestRejection::Job));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
    }

    #[test]
    fn complete_needs_a_started_quest_at_its_end_npc_and_ignores_level() {
        let q = quest();
        let mut p = player(30, 0);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::WrongState)
        );
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2001, &p), Err(QuestRejection::WrongNpc));
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        set(&mut p, 1033, QuestState::Completed);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::WrongState)
        );
    }

    #[test]
    fn a_quest_that_takes_mesos_needs_them_held() {
        let mut q = quest();
        q.money = -200;
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        p.meso = 199;
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Mesos));
        p.meso = 200;
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        q.money = 200;
        p.meso = 0;
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn completion_checks_item_counts_and_zero_means_none_left() {
        let mut q = quest();
        q.end_items = vec![(4031792, 2), (2010007, 0)];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Items(4031792))
        );
        p.inventory.add(game::Item::stack(4031792, 2), 100).unwrap();
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        p.inventory.add(game::Item::stack(2010007, 1), 100).unwrap();
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Items(2010007))
        );
    }

    #[test]
    fn completion_needs_the_listed_quests_in_their_states() {
        let mut q = quest();
        q.end_quests = vec![
            QuestPrerequisite {
                quest_id: 1041,
                state: QuestState::Completed,
            },
            QuestPrerequisite {
                quest_id: 1042,
                state: QuestState::Completed,
            },
        ];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        set(&mut p, 1041, QuestState::Completed);
        set(&mut p, 1042, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Prerequisite(1042))
        );
        set(&mut p, 1042, QuestState::Completed);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn mutually_exclusive_quests_require_the_others_unstarted() {
        let mut q = quest();
        q.start.quests = vec![QuestPrerequisite {
            quest_id: 2078,
            state: QuestState::NotStarted,
        }];
        let mut p = player(1, 0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 2078, QuestState::Started);
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Prerequisite(2078))
        );
    }

    #[test]
    fn start_items_count_worn_copies_and_bound_by_sign() {
        let mut q = quest();
        q.start.quests.clear();
        q.start.items = vec![(1002067, 2), (2010007, 0), (4000381, -1)];
        let mut p = player(1, 0);
        p.inventory.insert(
            game::InventoryType::Equipped,
            game::inventory::equip_slot::CAP,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(1002067))
        );
        p.inventory.insert(
            game::InventoryType::Equip,
            1,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.inventory.add(game::Item::stack(4000381, 1), 100).unwrap();
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.inventory.add(game::Item::stack(4000381, 1), 100).unwrap();
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(4000381))
        );
        q.start.items.truncate(2);
        p.inventory.add(game::Item::stack(2010007, 1), 100).unwrap();
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(2010007))
        );
    }

    #[test]
    fn repeatable_quests_restart_once_their_interval_has_passed() {
        let mut q = quest();
        q.start.quests.clear();
        let mut p = player(1, 0);
        let id = q.id;
        let completed = move |p: &mut Player, ago: u64| {
            p.quests.upsert(QuestRecord {
                quest_id: id,
                status: QuestStatus::Completed {
                    at: SystemTime::now() - Duration::from_secs(ago),
                },
            })
        };
        completed(&mut p, 0);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
        q.start.repeat_interval = Some(0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        q.start.repeat_interval = Some(60);
        completed(&mut p, 59 * 60);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
        completed(&mut p, 61 * 60);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, q.id, QuestState::Started);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
    }

    #[test]
    fn start_fame_is_compared_unsigned() {
        let mut q = quest();
        q.start.quests.clear();
        q.start.min_fame = 1;
        let mut p = player(1, 0);
        p.stats.fame = 0;
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Fame));
        p.stats.fame = 1;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.stats.fame = -3;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
    }

    #[test]
    fn start_skills_count_any_visible_skill_and_learned_hidden_ones() {
        if crate::skills::data().is_err() {
            return;
        }
        let mut q = quest();
        q.start.quests.clear();
        // Power Strike (visible) needed; Maple Warrior (hidden) must be lacked.
        q.start.skills = vec![(1001004, true), (1121000, false)];
        let mut p = player(1, 0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.skills.set(SkillRecord {
            skill_id: 1121000,
            level: 0,
            master_level: 10,
            expiration: None,
        });
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Skill(1121000))
        );
    }

    #[test]
    fn kills_count_toward_started_quests_up_to_the_required_number() {
        let mut q = quest();
        q.end_mobs = vec![(100100, 2), (100101, 1)];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Mobs(100100))
        );
        let catalog = |id| (id == 1033).then_some(&q);
        assert_eq!(count_kill(catalog, &mut p, 100100), [1033]);
        assert_eq!(count_kill(catalog, &mut p, 100100), [1033]);
        assert!(count_kill(catalog, &mut p, 100100).is_empty());
        assert!(count_kill(catalog, &mut p, 9300018).is_empty());
        assert_eq!(p.quests.kills(1033), [2, 0]);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Mobs(100101))
        );
        assert_eq!(count_kill(catalog, &mut p, 100101), [1033]);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn rewards_filter_by_job_and_gender_and_pick_one_weighted_entry() {
        use rand::SeedableRng;
        let item = |item_id, job, gender, prop| QuestItem {
            item_id,
            count: 1,
            job,
            gender,
            prop,
        };
        let items = [
            item(1002053, 1, 2, Some(1)),
            item(1002008, 1, 2, Some(1)),
            item(1302000, 2, 2, None),
            item(1041002, 0, 1, None),
            QuestItem {
                count: -1,
                ..item(4031802, 0, 2, None)
            },
            // Without a count: all held, or nothing.
            QuestItem {
                count: 0,
                ..item(4031792, 0, 2, None)
            },
            QuestItem {
                count: 0,
                ..item(4031856, 0, 2, None)
            },
        ];
        let mut beginner = player(1, 0);
        beginner.stats.gender = 0;
        beginner
            .inventory
            .add(game::Item::stack(4031792, 17), 100)
            .unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut hats = std::collections::BTreeSet::new();
        for _ in 0..50 {
            let picked = pick_items(&items, &beginner, -1, &mut rng).unwrap();
            // The shellpiece and every held 4031792 are always taken;
            // exactly one hat; no sword (warriors only) and no skirt
            // (female only).
            assert_eq!(picked.len(), 3);
            assert!(picked.contains(&(4031802, -1)));
            assert!(picked.contains(&(4031792, -17)));
            hats.insert(picked.iter().find(|(_, c)| *c > 0).unwrap().0);
        }
        assert_eq!(hats.len(), 2);
    }

    #[test]
    fn the_picked_reward_counts_only_choices_offered() {
        let choice = |item_id, count, job| QuestItem {
            item_id,
            count,
            job,
            gender: 2,
            prop: Some(-1),
        };
        let items = [
            QuestItem {
                prop: None,
                ..choice(4031223, -1, 0)
            },
            choice(1072133, 1, 0),
            // Not offered: nothing given, or for warriors only.
            choice(1072122, 0, 0),
            choice(1072303, 1, 2),
            choice(1072129, 1, 0),
        ];
        let beginner = player(37, 0);
        let mut rng = rand::thread_rng();
        let mut pick = |choice| pick_items(&items, &beginner, choice, &mut rng);
        assert_eq!(pick(1), Ok(vec![(4031223, -1), (1072129, 1)]));
        assert_eq!(pick(0), Ok(vec![(4031223, -1), (1072133, 1)]));
        assert_eq!(pick(2), Err(QuestRejection::Choice));
        assert_eq!(pick(-1), Err(QuestRejection::Choice));
        let warrior = player(37, 100);
        assert_eq!(
            pick_items(&items, &warrior, 1, &mut rand::thread_rng()),
            Ok(vec![(4031223, -1), (1072303, 1)])
        );
    }

    #[test]
    fn skill_rewards_match_the_exact_job_and_never_lower_a_skill() {
        let ambush = |skill_id, job| QuestSkill {
            skill_id,
            level: 0,
            master_level: 10,
            jobs: vec![job, 0],
        };
        let skills = [ambush(4221004, 422), ambush(4121004, 412)];
        let mut p = player(130, 412);
        let taught = apply_skills(&skills, &mut p);
        let expected = SkillRecord {
            skill_id: 4121004,
            level: 0,
            master_level: 10,
            expiration: None,
        };
        assert_eq!(taught, [expected.clone()]);
        assert_eq!(p.skills.get(4121004), Some(&expected));
        assert!(p.skills.get(4221004).is_none());
        // A known skill keeps its higher level and master level.
        p.skills.set(SkillRecord {
            level: 12,
            master_level: 20,
            ..expected
        });
        assert!(apply_skills(&skills, &mut p).is_empty());
        assert_eq!(p.skills.level(4121004), 12);
        // Job 410 is not 412, though both are thieves.
        assert!(apply_skills(&skills, &mut player(70, 410)).is_empty());
    }
}
