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
        /// Seen and taken only by those who still need it for this quest
        /// (`QuestNeeds`).
        quest: Option<i16>,
    },
    Mesos(i32),
    /// An item a player dropped, exactly as it was (equipment keeps its
    /// stats).
    Dropped(Item),
}

impl Loot {
    pub fn quest(&self) -> Option<i16> {
        self.quest_item().map(|(quest, _)| quest)
    }

    /// The quest a quest drop is for, and its item.
    pub fn quest_item(&self) -> Option<(i16, i32)> {
        match self {
            Loot::Item {
                item_id,
                quest: Some(quest),
                ..
            } => Some((*quest, *item_id)),
            Loot::Item { .. } | Loot::Mesos(_) | Loot::Dropped(_) => None,
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
/// (`crate::rates`), then its Monster Book card. `in_party` rolls quest
/// drops the player doesn't need too, for their party.
pub fn roll(
    mob_id: i32,
    showdown: i32,
    player: &Player,
    in_party: bool,
    now: SystemTime,
    rng: &mut impl Rng,
) -> Vec<Loot> {
    let rate = f64::from(100 + showdown) / 100.0 * crate::rates::drop(player, now);
    let mesos = crate::rates::meso(player, now);
    let entries = crate::content::rewards().drops(mob_id);
    let mut loot = roll_at(entries, (rate, mesos), player, in_party, rng);
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

/// Roll one broken reactor's drops for `player` (`in_party` as for
/// `roll`).
pub fn roll_reactor(
    reactor_id: i32,
    player: &Player,
    in_party: bool,
    rng: &mut impl Rng,
) -> Vec<Loot> {
    roll_at(
        crate::content::rewards().reactor_drops(reactor_id),
        (1.0, 1.0),
        player,
        in_party,
        rng,
    )
}

/// `loot` sprayed with mesos (Cosmic `sprayItems`): a pile of `from` up to
/// but not including `to` first, then more such piles after it until there are `at_least`
/// drops.
pub fn spray(
    loot: Vec<Loot>,
    at_least: usize,
    (from, to): (i32, i32),
    rng: &mut impl Rng,
) -> Vec<Loot> {
    let mut pile = || Loot::Mesos(rng.gen_range(from, to));
    let mut sprayed = vec![pile()];
    sprayed.extend(loot);
    while sprayed.len() < at_least {
        sprayed.push(pile());
    }
    sprayed
}

/// Roll what a friendly mob (`dropItemPeriod`) makes from its drops, for
/// no one in particular: the Moon Bunny's rice cakes, Tylus's Trust. A
/// quest drop is marked so, for only those here who need it to see it.
pub fn roll_friendly(mob_id: i32, rng: &mut impl Rng) -> Vec<Loot> {
    friendly_from(crate::content::rewards().drops(mob_id), rng)
}

fn friendly_from(entries: &[RewardEntry], rng: &mut impl Rng) -> Vec<Loot> {
    let mut loot = Vec::new();
    for entry in entries {
        if rng.gen_bool(entry.prob.clamp(0.0, 1.0)) {
            let quest = quest_of(entry);
            loot.extend(loot_of(entry, quest, 1.0, rng));
        }
    }
    loot
}

pub fn roll_entries(entries: &[RewardEntry], player: &Player, rng: &mut impl Rng) -> Vec<Loot> {
    roll_at(entries, (1.0, 1.0), player, false, rng)
}

/// `roll_entries`, with every chance times `rate` and every meso amount
/// times `mesos`.
fn roll_at(
    entries: &[RewardEntry],
    (rate, mesos): (f64, f64),
    player: &Player,
    in_party: bool,
    rng: &mut impl Rng,
) -> Vec<Loot> {
    let mut loot = Vec::new();
    for entry in entries {
        // Quest drops only while the quest is started and the player holds
        // fewer than it needs. In a party quest items roll anyway, as
        // Cosmic's do, for the members who need them: only those who do see
        // them (`QuestNeeds`).
        let quest = quest_of(entry);
        if let Some(quest) = quest {
            let needed = entry
                .item
                .map_or(player.quests.state(quest) == QuestState::Started, |item| {
                    wants(player, quest, item)
                });
            if !needed && !(in_party && entry.item.is_some()) {
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
        .filter(|entry| quest_of(entry).is_none())
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

/// The quest `entry` drops for: its own, or that of a quest item it holds.
/// An override's `quest = 0` makes it an ordinary drop anyway.
fn quest_of(entry: &RewardEntry) -> Option<i16> {
    match entry.quest {
        Some(0) => None,
        Some(quest) => Some(quest),
        None => entry.item.and_then(quest_for),
    }
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

/// Whether `player` still needs `item` for `quest`: it is started and they
/// hold fewer than its completion asks for (Cosmic's `needQuestItem`).
pub fn wants(player: &Player, quest: i16, item: i32) -> bool {
    player.quests.state(quest) == QuestState::Started
        && player.inventory.count(item) < needed(quest, item)
}

/// Which quest drops a player still needs, as `wants` has it, taken when
/// they reach for one: the field decides with it who may take a quest
/// drop.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuestNeeds {
    /// The quests the player has started.
    pub started: Vec<i16>,
    /// Items of those quests the player holds enough of.
    pub enough: Vec<(i16, i32)>,
}

impl QuestNeeds {
    pub fn of(player: &Player) -> Self {
        let started: Vec<i16> = player
            .quests
            .started()
            .map(|record| record.quest_id)
            .collect();
        let enough = started
            .iter()
            .filter_map(|&quest| {
                let catalog = crate::quests::catalog().ok()?;
                let items = &catalog.quest(quest)?.end_items;
                Some(
                    items
                        .iter()
                        .filter(|&&(item, count)| player.inventory.count(item) >= count)
                        .map(move |&(item, _)| (quest, item))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();
        QuestNeeds { started, enough }
    }

    pub fn wants(&self, quest: i16, item: i32) -> bool {
        self.started.contains(&quest) && !self.enough.contains(&(quest, item))
    }
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
    fn a_spray_leads_with_mesos_and_tops_up_with_more() {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(1);
        let item = Loot::Item {
            item_id: 2000002,
            quantity: 1,
            quest: None,
        };
        let sprayed = spray(vec![item.clone()], 15, (30, 60), rng);
        assert_eq!(sprayed.len(), 15);
        assert_eq!(sprayed[1], item);
        let mesos = sprayed.iter().filter_map(|loot| match loot {
            Loot::Mesos(amount) => Some(*amount),
            _ => None,
        });
        assert!(mesos.clone().all(|amount| (30..60).contains(&amount)));
        assert_eq!(mesos.count(), 14);
        let many = vec![item; 20];
        assert_eq!(spray(many, 15, (30, 60), rng).len(), 21);
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
    fn a_friendly_mob_makes_its_items_and_marks_its_quest_items() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let table = [
            entry(Some(4001101), None, 1.0, None),
            entry(Some(4031802), None, 1.0, Some(1035)),
            entry(Some(4000019), None, 0.0, None),
        ];
        let cake = Loot::Item {
            item_id: 4001101,
            quantity: 1,
            quest: None,
        };
        let shellpiece = Loot::Item {
            item_id: 4031802,
            quantity: 1,
            quest: Some(1035),
        };
        assert_eq!(friendly_from(&table, &mut rng), [cake, shellpiece]);
    }

    #[test]
    fn quest_drops_need_the_quest_and_stop_once_held() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let shellpiece = [entry(Some(4031802), None, 1.0, Some(1035))];
        let mut player = crate::codec::stats::sample_player();
        assert!(roll_entries(&shellpiece, &player, &mut rng).is_empty());
        player.quests.upsert(QuestRecord {
            quest_id: 1035,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
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
    fn quest_zero_makes_a_quest_item_an_ordinary_drop() {
        // Persimmons are a quest item that only quest 9330 asks for.
        if quest_for(2022055).is_none() {
            return;
        }
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let player = crate::codec::stats::sample_player();
        let inferred = [entry(Some(2022055), None, 1.0, None)];
        assert!(roll_entries(&inferred, &player, &mut rng).is_empty());
        let ordinary = [entry(Some(2022055), None, 1.0, Some(0))];
        let persimmon = Loot::Item {
            item_id: 2022055,
            quantity: 1,
            quest: None,
        };
        assert_eq!(
            roll_entries(&ordinary, &player, &mut rng),
            [persimmon.clone()]
        );
        assert_eq!(friendly_from(&ordinary, &mut rng), [persimmon]);
        assert!(steal_from(&ordinary, 1.0, &mut rng).is_some());
        assert!(steal_from(&inferred, 1.0, &mut rng).is_none());
    }

    #[test]
    fn a_party_rolls_quest_items_and_their_takers_need_them() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let table = [
            entry(Some(4031802), None, 1.0, Some(1035)),
            entry(None, Some(10), 1.0, Some(1035)),
        ];
        let mut player = crate::codec::stats::sample_player();
        let rolled = roll_at(&table, (1.0, 1.0), &player, true, &mut rng);
        assert_eq!(
            rolled,
            [Loot::Item {
                item_id: 4031802,
                quantity: 1,
                quest: Some(1035)
            }]
        );
        let agree = |player: &Player| {
            let needs = QuestNeeds::of(player);
            assert_eq!(needs.wants(1035, 4031802), wants(player, 1035, 4031802));
            needs.wants(1035, 4031802)
        };
        assert!(!agree(&player));
        player.quests.upsert(QuestRecord {
            quest_id: 1035,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        assert!(agree(&player));
        player
            .inventory
            .add(game::Item::stack(4031802, 1), 100)
            .unwrap();
        assert!(!agree(&player));
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
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
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
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
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
            assert_eq!(
                roll_at(&half, (2.0, 1.0), &player, false, &mut rng).len(),
                1
            );
        }
    }

    #[test]
    fn the_meso_rate_multiplies_each_amount() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let player = crate::codec::stats::sample_player();
        let mesos = [entry(None, Some(100), 1.0, None)];
        for _ in 0..50 {
            let [Loot::Mesos(amount)] = roll_at(&mesos, (1.0, 2.0), &player, false, &mut rng)[..]
            else {
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
