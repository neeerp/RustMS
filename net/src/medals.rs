//! The explorer medals (quests 29004 to 29011 and 29014). Entering a map of
//! a region (`explorationPoint`), or touching a `highposition` portal on one
//! of the highest places, counts the map toward that region's medal: the
//! first one starts the quest, each new one raises the count in the medal's
//! info record, and Dalair gives the medal once the count reaches what
//! `Check/1/infoex` asks.
//!
//! Choices with no source either way (v83ref lanes/QUESTIONS.md, 2026-10-22):
//! a map counts only from the quest's `lvmin`, 29012 and 29013 (which also
//! need a record nothing sets) are left out, and a forfeit keeps the maps.

use crate::content;
use crate::error::NetworkError;
use crate::packet::build::world::quest::{
    build_info_text, build_quest_clear, build_script_progress,
};
use crate::quests::{catalog, QuestTemplate};
use crate::scripts::Effect;
use game::{Player, QuestState};
use game_data::server_content::keys;
use packet::Packet;

/// One Who's Touched the Sky: the `highposition` portals' medal.
pub const TOUCHED_THE_SKY: i16 = 29004;

/// The medal whose region holds `map_id`.
pub fn region(map_id: i32) -> Option<i16> {
    Some(match map_id {
        110000000 | 100000000..=105040299 => 29005,
        105040300..=105090900 => 29014,
        200000000..=211041800 => 29006,
        220000000..=222020000 => 29007,
        230000000..=230040401 => 29008,
        240000000..=240050000 => 29011,
        250000000..=251010500 => 29009,
        260000000..=261030000 => 29010,
        _ => return None,
    })
}

/// A visit to `map_id` toward `medal`, unless the medal is done, the player
/// is below its level, or the map already counted. A map already counted
/// still starts a forfeited medal again, whose maps were kept.
pub fn explore(medal: i16, map_id: i32, player: &Player) -> Option<Effect> {
    let quest = catalog().ok()?.quest(medal)?;
    let explored = quest.explored?;
    let level = i16::from(player.stats.level);
    let state = player.quests.state(medal);
    if state == QuestState::Completed
        || quest.start.min_level.is_some_and(|min| level < min)
        || (state == QuestState::Started && player.quests.has_explored(explored.record, map_id))
    {
        return None;
    }
    Some(Effect::Explore { medal, map_id })
}

/// What the player is told when `quest`'s count reaches `count`: how far
/// they are, or, at the count needed, that the title is earned. Past it
/// (Minar Forest has two maps to spare) nothing more is said.
pub fn messages(quest: &QuestTemplate, count: usize) -> Result<Vec<Packet>, NetworkError> {
    let Some(explored) = quest.explored else {
        return Ok(Vec::new());
    };
    let needed = explored.needed as usize;
    let text = |key| {
        content::text().get(
            key,
            &[
                ("name", &quest.name),
                ("count", &count.to_string()),
                ("needed", &needed.to_string()),
            ],
        )
    };
    Ok(match count.cmp(&needed) {
        std::cmp::Ordering::Less => vec![
            build_script_progress(&text(keys::MEDAL_PROGRESS))?,
            build_info_text(&text(keys::MEDAL_PROGRESS_INFO))?,
        ],
        std::cmp::Ordering::Equal => vec![
            build_quest_clear(quest.id)?,
            build_script_progress(&text(keys::MEDAL_EARNED))?,
            build_info_text(&text(keys::MEDAL_EARNED_INFO))?,
        ],
        std::cmp::Ordering::Greater => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{QuestRecord, QuestStatus};
    use std::collections::BTreeMap;
    use std::time::SystemTime;

    fn player(level: u8) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        player
    }

    #[test]
    fn each_region_has_the_maps_its_medal_needs() {
        let Ok(quests) = catalog() else {
            return;
        };
        let data = crate::get_game_data().unwrap();
        let mut maps: BTreeMap<i16, usize> = BTreeMap::new();
        for map_id in data.field_ids() {
            let field = data.field(map_id).unwrap();
            if field.on_user_enter.as_deref() == Some("explorationPoint") {
                if let Some(medal) = region(map_id) {
                    *maps.entry(medal).or_default() += 1;
                }
            }
        }
        for &medal in game_data::quests::EXPLORER_QUESTS {
            if medal == TOUCHED_THE_SKY {
                continue;
            }
            let needed = quests.quest(medal).unwrap().explored.unwrap().needed;
            assert!(maps[&medal] >= needed as usize, "{medal}: {maps:?}");
        }
        // 97 maps run explorationPoint; 910310000-910310004 are in no
        // region.
        assert_eq!(maps.values().sum::<usize>(), 92);
    }

    #[test]
    fn a_map_counts_once_from_the_medals_level_until_it_is_done() {
        if catalog().is_err() {
            return;
        }
        let henesys = 100000000;
        // Beginner Explorer counts from level 15.
        assert_eq!(explore(29005, henesys, &player(14)), None);
        let mut player = player(15);
        let visit = Some(Effect::Explore {
            medal: 29005,
            map_id: henesys,
        });
        assert_eq!(explore(29005, henesys, &player), visit);
        player.quests.explore(27010, henesys);
        // Seen, but the quest isn't started (it was forfeited): the visit
        // starts it again.
        assert_eq!(explore(29005, henesys, &player), visit);
        player.quests.upsert(QuestRecord {
            quest_id: 29005,
            status: QuestStatus::Started { kills: Vec::new() },
        });
        assert_eq!(explore(29005, henesys, &player), None);
        assert!(explore(29005, 101000000, &player).is_some());
        player.quests.upsert(QuestRecord {
            quest_id: 29005,
            status: QuestStatus::Completed {
                at: SystemTime::UNIX_EPOCH,
            },
        });
        assert_eq!(explore(29005, 101000000, &player), None);
        // Not a medal.
        assert_eq!(explore(1000, henesys, &player), None);
    }

    #[test]
    fn the_title_is_told_once_at_its_count() {
        let Ok(quests) = catalog() else {
            return;
        };
        let minar = quests.quest(29011).unwrap();
        assert_eq!(messages(minar, 9).unwrap().len(), 2);
        assert_eq!(messages(minar, 10).unwrap().len(), 3);
        assert_eq!(messages(minar, 11).unwrap().len(), 0);
    }

    #[test]
    fn regions_follow_the_map_ids() {
        assert_eq!(region(110000000), Some(29005));
        assert_eq!(region(104000000), Some(29005));
        assert_eq!(region(105040300), Some(29014));
        assert_eq!(region(240050000), Some(29011));
        assert_eq!(region(910310000), None);
    }
}
