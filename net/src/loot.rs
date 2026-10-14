//! What a killed mob or broken reactor drops for one player, from the
//! content pack's drop table (`crate::content::rewards`) and the committed
//! v83 overrides.

use game::{Item, Player, QuestState};
use game_data::server_content::RewardEntry;
use rand::Rng;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loot {
    Item {
        item_id: i32,
        quantity: i16,
        /// Only for its owner, while they have this quest started.
        quest: Option<i16>,
    },
    Mesos(i32),
    /// An item a player dropped, exactly as it was (equipment keeps its
    /// stats).
    Dropped(Item),
}

impl Loot {
    pub fn quest(&self) -> Option<i16> {
        match self {
            Loot::Item { quest, .. } => *quest,
            Loot::Mesos(_) | Loot::Dropped(_) => None,
        }
    }
}

/// Roll one mob's drops for `player`.
pub fn roll(mob_id: i32, player: &Player, rng: &mut impl Rng) -> Vec<Loot> {
    roll_entries(crate::content::rewards().drops(mob_id), player, rng)
}

/// Roll one broken reactor's drops for `player`.
pub fn roll_reactor(reactor_id: i32, player: &Player, rng: &mut impl Rng) -> Vec<Loot> {
    roll_entries(
        crate::content::rewards().reactor_drops(reactor_id),
        player,
        rng,
    )
}

pub fn roll_entries(entries: &[RewardEntry], player: &Player, rng: &mut impl Rng) -> Vec<Loot> {
    let mut loot = Vec::new();
    for entry in entries {
        // Quest drops only while the quest is started and the item is still
        // needed (one of each is enough for the beginner quests).
        if let Some(quest) = entry.quest {
            if player.quests.state(quest) != QuestState::Started {
                continue;
            }
            if entry
                .item
                .is_some_and(|item| player.inventory.count(item) > 0)
            {
                continue;
            }
        }
        if !rng.gen_bool(entry.prob.clamp(0.0, 1.0)) {
            continue;
        }
        if let Some(amount) = entry.money {
            // v83 varies meso drops by about ±25%.
            let spread = (amount / 4).max(0);
            let amount = rng.gen_range(amount - spread, amount + spread + 1).max(1);
            loot.push(Loot::Mesos(amount));
        } else if let Some(item_id) = entry.item {
            let (min, max) = (
                entry.min.unwrap_or(1),
                entry.max.unwrap_or(entry.min.unwrap_or(1)),
            );
            let quantity = rng
                .gen_range(min.min(max), max.max(min) + 1)
                .clamp(1, i32::from(i16::MAX)) as i16;
            loot.push(Loot::Item {
                item_id,
                quantity,
                quest: entry.quest,
            });
        }
    }
    loot
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{QuestRecord, QuestStatus};
    use rand::SeedableRng;

    fn entry(item: Option<i32>, money: Option<i32>, prob: f64, quest: Option<i16>) -> RewardEntry {
        RewardEntry {
            item,
            money,
            prob,
            min: None,
            max: None,
            quest,
            premium: None,
        }
    }

    #[test]
    fn quest_drops_need_the_quest_and_stop_once_held() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let shellpiece = [entry(Some(4031802), None, 1.0, Some(1035))];
        let mut player = crate::codec::stats::sample_player();
        assert!(roll_entries(&shellpiece, &player, &mut rng).is_empty());
        player.quests.upsert(QuestRecord {
            quest_id: 1035,
            status: QuestStatus::Started { kills: Vec::new() },
        });
        assert_eq!(
            roll_entries(&shellpiece, &player, &mut rng),
            [Loot::Item {
                item_id: 4031802,
                quantity: 1,
                quest: Some(1035)
            }]
        );
        player
            .inventory
            .add(game::Item::stack(4031802, 1), 100)
            .unwrap();
        assert!(roll_entries(&shellpiece, &player, &mut rng).is_empty());
    }

    #[test]
    fn mesos_vary_around_the_table_amount() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let player = crate::codec::stats::sample_player();
        for _ in 0..50 {
            let [Loot::Mesos(amount)] =
                roll_entries(&[entry(None, Some(8), 1.0, None)], &player, &mut rng)[..]
            else {
                panic!("one meso drop");
            };
            assert!((6..=10).contains(&amount));
        }
        assert!(
            roll_entries(&[entry(Some(4000019), None, 0.0, None)], &player, &mut rng).is_empty()
        );
    }
}
