//! The v83 quest flow. The client renders the conversation from `Say.img`
//! and sends `QUEST_ACTION`; the server checks the request against the
//! catalog and the player's state before anything is persisted.

use crate::load;
use game::{InventoryChange, InventoryError, Player, QuestState};
pub use game_data::quests::{
    QuestCatalog, QuestItem, QuestPrerequisite, QuestTemplate, StartRequirements,
};
use rand::Rng;
use std::sync::OnceLock;

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
    Start { npc_id: i32 },
    Complete { npc_id: i32 },
    Forfeit,
    ScriptStart { npc_id: i32 },
    ScriptEnd { npc_id: i32 },
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
    /// Fewer mesos than the quest takes.
    Mesos,
    /// The item rewards don't fit.
    InventoryFull,
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
    if player.quests.state(quest.id) != QuestState::NotStarted {
        return Err(QuestRejection::WrongState);
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
    Ok(())
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
/// among those with one, weighted by it. An entry without a count takes
/// all the player holds, and is left out if that's none.
pub fn pick_items(items: &[QuestItem], player: &Player, rng: &mut impl Rng) -> Vec<(i32, i32)> {
    let eligible: Vec<&QuestItem> = items
        .iter()
        .filter(|item| item.applies_to(player.stats.job, player.stats.gender))
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
    picked
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

#[cfg(test)]
mod tests {
    use super::*;
    use game::{QuestRecord, QuestStatus};
    use std::time::SystemTime;

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
            },
            end_npc: 2102,
            end_items: vec![],
            end_mobs: vec![],
            end_quests: vec![],
            start_items: vec![],
            end_rewards: vec![],
            start_script: None,
            end_script: None,
            exp: 7,
            money: 0,
            next_quest: None,
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
            let picked = pick_items(&items, &beginner, &mut rng);
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
}
