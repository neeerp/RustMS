//! The Item Maker's recipes, from `Etc.nx` (`ItemMake.img`), and what the
//! monster crystals are made from, from `Item.nx` (reports c006 and
//! c006b). Each recipe is named for what it makes; the groups it sits in
//! are the jobs that see it in the window (0 for everyone).

use crate::items::path_index;
use crate::{GameDataError, NxDataFile};
use std::collections::HashMap;
use std::path::Path;

/// The monster crystals, 4260000 to 4260008.
pub const CRYSTALS: std::ops::RangeInclusive<i32> = 4_260_000..=4_260_008;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recipe {
    pub item_id: i32,
    /// Its `ItemMake.img` group.
    pub group: i32,
    /// How many it makes (`itemNum`).
    pub count: i32,
    /// The fee before the window adds to it (`meso`).
    pub meso: i32,
    /// Each ingredient and how many it takes.
    pub ingredients: Vec<(i32, i32)>,
    /// The stimulator equipment takes, 0 for none (`catalyst`).
    pub catalyst: i32,
    /// What a gem recipe may make instead of the gem (`randomReward`).
    pub rewards: Vec<Reward>,
    pub req_level: i32,
    /// The Maker skill's level it needs.
    pub req_skill_level: i32,
    /// An item it needs held, 0 for none.
    pub req_item: i32,
    /// An item it needs worn, 0 for none.
    pub req_equip: i32,
    /// Quests and the state each must be in: 1 started, 2 completed, 3
    /// either.
    pub req_quests: Vec<(i32, i32)>,
    /// The gem slots the window gives it (`tuc`).
    pub gem_slots: i32,
}

/// One of a gem recipe's outcomes, with its weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reward {
    pub item_id: i32,
    pub count: i32,
    pub weight: i32,
}

#[derive(Debug, Default)]
pub struct MakerData {
    recipes: HashMap<i32, Recipe>,
    /// Each etc item a crystal is made from, and its level (`info/lv`).
    levels: HashMap<i32, i32>,
    /// Each crystal and the levels it is made from (`lvMin`, `lvMax`).
    crystals: Vec<(i32, i32, i32)>,
}

impl MakerData {
    pub fn load(etc: impl AsRef<Path>, items: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let mut data = Self::default();
        let nx = NxDataFile::open(etc.as_ref())?;
        if let Some(root) = nx.child_by_name(0, "ItemMake.img")? {
            for group in nx.child_indices(root)? {
                let Ok(group_key) = nx.node_name(group)?.parse() else {
                    continue;
                };
                for node in nx.child_indices(group)? {
                    if let Some(recipe) = recipe(&nx, group_key, node)? {
                        data.recipes.insert(recipe.item_id, recipe);
                    }
                }
            }
        }
        let nx = NxDataFile::open(items.as_ref())?;
        if let Some(etc) = path_index(&nx, "Etc/0400.img")? {
            for node in nx.child_indices(etc)? {
                let Ok(item_id) = nx.node_name(node)?.parse() else {
                    continue;
                };
                let info = nx.child_by_name(node, "info")?;
                if let Some(lv) = info
                    .map(|info| nx.int_child(info, "lv"))
                    .transpose()?
                    .flatten()
                {
                    data.levels.insert(item_id, lv);
                }
            }
        }
        for crystal in CRYSTALS {
            let Some(info) = path_index(&nx, &format!("Etc/0426.img/{crystal:08}/info"))? else {
                continue;
            };
            let low = nx.int_child(info, "lvMin")?;
            let high = nx.int_child(info, "lvMax")?;
            if let (Some(low), Some(high)) = (low, high) {
                data.crystals.push((crystal, low, high));
            }
        }
        Ok(data)
    }

    pub fn recipe(&self, item_id: i32) -> Option<&Recipe> {
        self.recipes.get(&item_id)
    }

    /// The crystal 100 of `item_id` make, if it makes one.
    pub fn crystal(&self, item_id: i32) -> Option<i32> {
        let lv = *self.levels.get(&item_id)?;
        self.crystals
            .iter()
            .find(|(_, low, high)| (*low..=*high).contains(&lv))
            .map(|(crystal, _, _)| *crystal)
    }
}

fn recipe(nx: &NxDataFile, group: i32, node: u32) -> Result<Option<Recipe>, GameDataError> {
    let Ok(item_id) = nx.node_name(node)?.parse() else {
        return Ok(None);
    };
    let int =
        |key: &str| -> Result<i32, GameDataError> { Ok(nx.int_child(node, key)?.unwrap_or(0)) };
    let mut recipe = Recipe {
        item_id,
        group,
        count: int("itemNum")?.max(1),
        meso: int("meso")?,
        catalyst: int("catalyst")?,
        req_level: int("reqLevel")?,
        req_skill_level: int("reqSkillLevel")?,
        req_item: int("reqItem")?,
        req_equip: int("reqEquip")?,
        gem_slots: int("tuc")?,
        ..Recipe::default()
    };
    if let Some(list) = nx.child_by_name(node, "recipe")? {
        for entry in nx.child_indices(list)? {
            let item = nx.int_child(entry, "item")?;
            let count = nx.int_child(entry, "count")?;
            if let (Some(item), Some(count)) = (item, count) {
                recipe.ingredients.push((item, count));
            }
        }
    }
    if let Some(list) = nx.child_by_name(node, "randomReward")? {
        for entry in nx.child_indices(list)? {
            let Some(item_id) = nx.int_child(entry, "item")? else {
                continue;
            };
            recipe.rewards.push(Reward {
                item_id,
                count: nx.int_child(entry, "itemNum")?.unwrap_or(1).max(1),
                weight: nx.int_child(entry, "prob")?.unwrap_or(0).max(0),
            });
        }
    }
    if let Some(list) = nx.child_by_name(node, "reqQuest")? {
        for entry in nx.child_indices(list)? {
            let name = nx.node_name(entry)?;
            let Ok(quest) = name.parse() else {
                continue;
            };
            if let Some(state) = nx.int_child(list, &name)? {
                recipe.req_quests.push((quest, state));
            }
        }
    }
    Ok(Some(recipe))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<MakerData> {
        let etc = crate::test_assets::path("Etc.nx")?;
        let items = crate::test_assets::path("Item.nx")?;
        Some(MakerData::load(etc, items).unwrap())
    }

    #[test]
    fn the_recipes_hold_what_item_make_says() {
        let Some(data) = data() else {
            return;
        };
        assert_eq!(data.recipes.len(), 832);
        let diamond = data.recipe(4_250_000).unwrap();
        assert_eq!(
            (
                diamond.group,
                diamond.count,
                diamond.meso,
                diamond.req_level
            ),
            (0, 1, 100_000, 45)
        );
        assert_eq!(diamond.ingredients, [(4_021_007, 1)]);
        let weights: Vec<_> = diamond
            .rewards
            .iter()
            .map(|r| (r.item_id, r.count, r.weight))
            .collect();
        assert_eq!(
            weights,
            [(4_250_000, 1, 14), (4_250_001, 1, 5), (4_250_002, 1, 1)]
        );
        // Ten Basic make an Intermediate, or nine Basic come back.
        let rewards = &data.recipe(4_250_001).unwrap().rewards;
        assert_eq!(
            rewards[1],
            Reward {
                item_id: 4_250_000,
                count: 9,
                weight: 2
            }
        );

        let sword = data.recipe(1_302_010).unwrap();
        assert_eq!(
            (sword.catalyst, sword.meso, sword.gem_slots),
            (4_130_002, 60_000, 1)
        );
        assert_eq!(sword.ingredients, [(4_011_000, 1), (4_260_000, 20)]);
        assert_eq!(data.recipe(2_040_727).unwrap().req_equip, 1_122_013);
        assert_eq!(data.recipe(4_032_312).unwrap().req_quests, [(21_303, 2)]);
        assert_eq!(data.recipe(4_001_174).unwrap().req_item, 4_031_966);
    }

    #[test]
    fn a_hundred_drops_make_the_crystal_of_their_level() {
        let Some(data) = data() else {
            return;
        };
        assert_eq!(data.crystals.len(), 9);
        // 4000013 is level 35 and 4000022 level 55.
        assert_eq!(data.crystal(4_000_013), Some(4_260_000));
        assert_eq!(data.crystal(4_000_022), Some(4_260_001));
        // A Snail Shell has no level.
        assert_eq!(data.crystal(4_000_019), None);
    }
}
