//! The v83 quest flow. The client renders the conversation from `Say.img`
//! and sends `QUEST_ACTION`; the server checks the request against the
//! catalog and the player's state before anything is persisted.

use crate::load;
use game::{Player, QuestState};
pub use game_data::quests::{QuestCatalog, QuestPrerequisite, QuestTemplate, StartRequirements};
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
    Ok(())
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
            start_script: None,
            end_script: None,
            exp: 7,
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
            QuestState::Started => QuestStatus::Started,
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
}
