//! What a killed mob or broken reactor drops for one player, from the
//! content pack's drop table (`crate::content::rewards`) and the committed
//! v83 overrides.

use game::{Item, Player, QuestState};
use game_data::server_content::RewardEntry;
use rand::Rng;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::SystemTime;

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

    /// Whether nobody may pick it up (`pickUpBlock`). The v83 client leaves
    /// such items out of the pickup key and pets' looting (FUN_005d5224).
    pub fn blocked(&self) -> bool {
        let item_id = match self {
            Loot::Item { item_id, .. } => *item_id,
            Loot::Dropped(item) => item.item_id,
            Loot::Mesos(_) => return false,
        };
        crate::items::info(item_id).is_some_and(|info| info.pickup_block)
    }
}

/// Roll one mob's drops for `player` at `now`, each `showdown` percent more
/// likely (Taunt's) and scaled by the world's and their cards' rates
/// (`crate::rates`), then its Monster Book card.
pub fn roll(
    mob_id: i32,
    showdown: i32,
    player: &Player,
    now: SystemTime,
    rng: &mut impl Rng,
) -> Vec<Loot> {
    let rate = f64::from(100 + showdown) / 100.0 * crate::rates::drop(player, now);
    let mesos = crate::rates::meso(player, now);
    let entries = crate::content::rewards().drops(mob_id);
    let mut loot = roll_at(entries, (rate, mesos), player, rng);
    if let Some(card) = crate::monster_book::card_for(mob_id) {
        if rng.gen_bool(crate::monster_book::drop_rate()) {
            loot.push(Loot::Item {
                item_id: card,
                quantity: 1,
                quest: None,
            });
        }
    }
    loot
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
    roll_at(entries, (1.0, 1.0), player, rng)
}

/// `roll_entries`, with every chance times `rate` and every meso amount
/// times `mesos`.
fn roll_at(
    entries: &[RewardEntry],
    (rate, mesos): (f64, f64),
    player: &Player,
    rng: &mut impl Rng,
) -> Vec<Loot> {
    let mut loot = Vec::new();
    for entry in entries {
        // Quest drops only while the quest is started and the player holds
        // fewer than it needs.
        let quest = entry.quest.or_else(|| entry.item.and_then(quest_for));
        if let Some(quest) = quest {
            if player.quests.state(quest) != QuestState::Started {
                continue;
            }
            if entry
                .item
                .is_some_and(|item| player.inventory.count(item) >= needed(quest, item))
            {
                continue;
            }
        }
        if !rng.gen_bool((entry.prob * rate).clamp(0.0, 1.0)) {
            continue;
        }
        loot.extend(loot_of(entry, quest, mesos, rng));
    }
    loot
}

/// What one entry drops: its mesos times `mesos`, or its item.
fn loot_of(
    entry: &RewardEntry,
    quest: Option<i16>,
    mesos: f64,
    rng: &mut impl Rng,
) -> Option<Loot> {
    if let Some(amount) = entry.money {
        // 4/5 of the amount plus up to 2/5 more, as the Nexon server
        // OrionAlpha decompiles has it (`Reward.create`): about ±20%.
        let money = i64::from(amount.max(0));
        let (base, width) = (4 * money / 5, 2 * money / 5);
        let amount = (base + rng.gen_range(0, width + 1)).max(1);
        let amount = (amount as f64 * mesos).round().min(f64::from(i32::MAX)) as i32;
        return Some(Loot::Mesos(amount.max(1)));
    }
    let item_id = entry.item?;
    let (min, max) = (
        entry.min.unwrap_or(1),
        entry.max.unwrap_or(entry.min.unwrap_or(1)),
    );
    let quantity = rng
        .gen_range(min.min(max), max.max(min) + 1)
        .clamp(1, i32::from(i16::MAX)) as i16;
    Some(Loot::Item {
        item_id,
        quantity,
        quest,
    })
}

/// What Steal takes from a mob for `player`: one of its drops, quest items
/// aside, picked by their chances (Cosmic's rule), its mesos at the
/// player's rates.
pub fn steal(mob_id: i32, player: &Player, now: SystemTime, rng: &mut impl Rng) -> Option<Loot> {
    let mesos = crate::rates::meso(player, now);
    steal_from(crate::content::rewards().drops(mob_id), mesos, rng)
}

fn steal_from(entries: &[RewardEntry], mesos: f64, rng: &mut impl Rng) -> Option<Loot> {
    let pool: Vec<&RewardEntry> = entries
        .iter()
        .filter(|entry| {
            entry
                .quest
                .or_else(|| entry.item.and_then(quest_for))
                .is_none()
        })
        .filter(|entry| entry.prob > 0.0)
        .collect();
    let total: f64 = pool.iter().map(|entry| entry.prob).sum();
    if pool.is_empty() {
        return None;
    }
    let mut pick = rng.gen::<f64>() * total;
    let entry = pool
        .iter()
        .find(|entry| {
            pick -= entry.prob;
            pick < 0.0
        })
        .unwrap_or(&pool[pool.len() - 1]);
    loot_of(entry, None, mesos, rng)
}

/// The quest a quest item (`info/quest`) the drop table doesn't tag drops
/// for: the one quest whose completion check asks for it.
fn quest_for(item: i32) -> Option<i16> {
    static ASKED_BY: OnceLock<HashMap<i32, Option<i16>>> = OnceLock::new();
    let asked_by = ASKED_BY.get_or_init(|| {
        let mut asked_by = HashMap::new();
        let Ok(catalog) = crate::quests::catalog() else {
            return asked_by;
        };
        for quest in catalog.supported() {
            for &(item, count) in &quest.end_items {
                if count > 0 {
                    asked_by
                        .entry(item)
                        .and_modify(|only: &mut Option<i16>| *only = None)
                        .or_insert(Some(quest.id));
                }
            }
        }
        asked_by
    });
    if !crate::items::info(item).is_some_and(|info| info.quest) {
        return None;
    }
    asked_by.get(&item).copied().flatten()
}

/// How many of `item` completing `quest` takes: any number when the quest
/// doesn't ask for it.
fn needed(quest: i16, item: i32) -> i32 {
    crate::quests::catalog()
        .ok()
        .and_then(|catalog| catalog.quest(quest))
        .and_then(|quest| quest.end_items.iter().find(|(id, _)| *id == item))
        .map_or(i32::MAX, |&(_, count)| count)
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
    fn steal_picks_one_drop_by_its_chance_and_never_a_quest_item() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        assert_eq!(steal_from(&[], 1.0, &mut rng), None);
        let table = [
            entry(Some(4000019), None, 0.3, None),
            entry(Some(4031802), None, 1.0, Some(1035)),
            entry(None, Some(10), 0.1, None),
        ];
        let mut counts = (0, 0);
        for _ in 0..400 {
            match steal_from(&table, 2.0, &mut rng) {
                Some(Loot::Item {
                    item_id: 4000019,
                    quantity: 1,
                    quest: None,
                }) => counts.0 += 1,
                Some(Loot::Mesos(amount)) => {
                    assert!((16..=26).contains(&amount), "{amount}");
                    counts.1 += 1;
                }
                other => panic!("stole {other:?}"),
            }
        }
        // Three items for each meso bag.
        assert!((250..350).contains(&counts.0), "{counts:?}");
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
    fn quest_drops_continue_until_the_quest_has_enough() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        // Quest 1034 needs ten of item 4031792.
        let quest = crate::quests::catalog().unwrap().quest(1034).unwrap();
        assert!(quest.end_items.contains(&(4031792, 10)));
        let drop = [entry(Some(4031792), None, 1.0, Some(1034))];
        let mut player = crate::codec::stats::sample_player();
        player.quests.upsert(QuestRecord {
            quest_id: 1034,
            status: QuestStatus::Started { kills: Vec::new() },
        });
        player
            .inventory
            .add(game::Item::stack(4031792, 9), 100)
            .unwrap();
        assert_eq!(roll_entries(&drop, &player, &mut rng).len(), 1);
        player
            .inventory
            .add(game::Item::stack(4031792, 1), 100)
            .unwrap();
        assert!(roll_entries(&drop, &player, &mut rng).is_empty());
    }

    #[test]
    fn untagged_quest_items_drop_for_their_quest() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        // 4031792 is a quest item that only quest 1034 asks for.
        let drop = [entry(Some(4031792), None, 1.0, None)];
        let mut player = crate::codec::stats::sample_player();
        assert!(roll_entries(&drop, &player, &mut rng).is_empty());
        player.quests.upsert(QuestRecord {
            quest_id: 1034,
            status: QuestStatus::Started { kills: Vec::new() },
        });
        assert_eq!(
            roll_entries(&drop, &player, &mut rng),
            [Loot::Item {
                item_id: 4031792,
                quantity: 1,
                quest: Some(1034)
            }]
        );
        // An ordinary item a quest asks for (Blue Snail Shell, quest 1007)
        // drops for everyone.
        assert_eq!(
            roll_entries(&[entry(Some(4000000), None, 1.0, None)], &player, &mut rng).len(),
            1
        );
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
            assert!((6..=9).contains(&amount), "{amount}");
        }
        assert!(
            roll_entries(&[entry(Some(4000019), None, 0.0, None)], &player, &mut rng).is_empty()
        );
    }

    #[test]
    fn taunt_makes_drops_likelier() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let player = crate::codec::stats::sample_player();
        let half = [entry(Some(4000019), None, 0.5, None)];
        // Twice as likely: always.
        for _ in 0..50 {
            assert_eq!(roll_at(&half, (2.0, 1.0), &player, &mut rng).len(), 1);
        }
    }

    #[test]
    fn the_meso_rate_multiplies_each_amount() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let player = crate::codec::stats::sample_player();
        let mesos = [entry(None, Some(100), 1.0, None)];
        for _ in 0..50 {
            let [Loot::Mesos(amount)] = roll_at(&mesos, (1.0, 2.0), &player, &mut rng)[..] else {
                panic!("one meso drop");
            };
            assert!((160..=240).contains(&amount) && amount % 2 == 0);
        }
    }

    #[test]
    fn nobody_picks_up_a_pickup_block_item() {
        let item = |item_id| Loot::Item {
            item_id,
            quantity: 1,
            quest: None,
        };
        // A Transparent Item that a party quest drops, and a Snail Shell.
        assert!(item(4001074).blocked());
        assert!(!item(4000019).blocked());
        assert!(!Loot::Mesos(10).blocked());
    }
}
