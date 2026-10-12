use crate::inventory::{equip_slot, Inventory};
use crate::keymap::Keymap;
use crate::quest::QuestLog;
use crate::skill::SkillBook;
use std::collections::BTreeMap;

/// The core stat block that v83 sends in character lists, SET_FIELD, and
/// stat updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterStats {
    pub id: i32,
    pub name: String,
    pub gender: u8,
    pub skin: u8,
    pub face: i32,
    /// Hair style including its color digit, as the client expects it.
    pub hair: i32,
    pub level: u8,
    pub job: i16,
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub hp: i16,
    pub max_hp: i16,
    pub mp: i16,
    pub max_mp: i16,
    pub ap: i16,
    pub sp: i16,
    pub exp: i32,
    pub fame: i16,
    pub map_id: i32,
    /// Portal id the character respawns at when entering `map_id`.
    pub spawn_point: u8,
}

/// What other players see: body, face, hair, and visible equipment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvatarLook {
    pub gender: u8,
    pub skin: u8,
    pub face: i32,
    pub hair: i32,
    /// Visible equipment keyed by positive body slot (1 = cap, 5 = top, ...).
    /// Cash items override regular items in the same slot.
    pub visible: BTreeMap<u8, i32>,
    /// Regular items hidden underneath a cash item, keyed the same way.
    pub masked: BTreeMap<u8, i32>,
    /// The cash weapon shown over the regular weapon, if any.
    pub cash_weapon: Option<i32>,
}

impl AvatarLook {
    /// Build a look from equipped item ids keyed by their (negative) slots.
    pub fn from_equipped(
        stats: &CharacterStats,
        equipped: impl IntoIterator<Item = (i16, i32)>,
    ) -> Self {
        let mut regular = BTreeMap::new();
        let mut cash = BTreeMap::new();
        let mut cash_weapon = None;
        for (slot, item_id) in equipped {
            if slot == equip_slot::CASH_WEAPON {
                cash_weapon = Some(item_id);
            } else if let Some(body) = equip_slot::cash_body_slot(slot) {
                cash.insert(body, item_id);
            } else if let Some(body) = equip_slot::body_slot(slot) {
                regular.insert(body, item_id);
            }
        }

        let mut visible = regular.clone();
        let mut masked = BTreeMap::new();
        for (body, item_id) in cash {
            if let Some(hidden) = visible.insert(body, item_id) {
                masked.insert(body, hidden);
            }
        }

        Self {
            gender: stats.gender,
            skin: stats.skin,
            face: stats.face,
            hair: stats.hair,
            visible,
            masked,
            cash_weapon,
        }
    }
}

/// What a field needs in order to show a player to everyone else on the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldAvatar {
    pub id: i32,
    pub name: String,
    pub level: u8,
    pub job: i16,
    pub look: AvatarLook,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
}

/// A character that is logged in. The owning client actor holds the only
/// copy; everything else (fields, the world directory) receives projections.
#[derive(Clone, Debug)]
pub struct Player {
    pub account_id: i32,
    pub world_id: u8,
    pub stats: CharacterStats,
    pub meso: i32,
    pub buddy_capacity: u8,
    pub inventory: Inventory,
    pub skills: SkillBook,
    pub keymap: Keymap,
    pub quests: QuestLog,
    /// Account GM level; 0 for regular players.
    pub gm_level: i16,
    dirty: bool,
}

impl Player {
    pub fn new(
        account_id: i32,
        world_id: u8,
        stats: CharacterStats,
        meso: i32,
        inventory: Inventory,
        skills: SkillBook,
        keymap: Keymap,
        quests: QuestLog,
    ) -> Self {
        Self {
            account_id,
            world_id,
            stats,
            meso,
            buddy_capacity: 20,
            inventory,
            skills,
            keymap,
            quests,
            gm_level: 0,
            dirty: false,
        }
    }

    pub fn id(&self) -> i32 {
        self.stats.id
    }

    pub fn name(&self) -> &str {
        &self.stats.name
    }

    pub fn look(&self) -> AvatarLook {
        AvatarLook::from_equipped(&self.stats, self.inventory.equipped_ids())
    }

    /// The projection a field broadcasts for this player at a position.
    pub fn field_avatar(&self, x: i16, y: i16, stance: u8) -> FieldAvatar {
        FieldAvatar {
            id: self.stats.id,
            name: self.stats.name.clone(),
            level: self.stats.level,
            job: self.stats.job,
            look: self.look(),
            x,
            y,
            stance,
        }
    }

    /// Record that the character entered `map_id` at `spawn_point`.
    pub fn move_to(&mut self, map_id: i32, spawn_point: u8) {
        self.stats.map_id = map_id;
        self.stats.spawn_point = spawn_point;
        self.dirty = true;
    }

    /// Mutable access to the keymap; marks the player as needing a save.
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        self.dirty = true;
        &mut self.keymap
    }

    /// Mutable access to the inventory; marks the player as needing a save.
    pub fn inventory_mut(&mut self) -> &mut Inventory {
        self.dirty = true;
        &mut self.inventory
    }

    /// Add EXP, levelling up as many times as it covers. Each level grants
    /// 5 AP. Returns the number of levels gained. (HP/MP and SP growth come
    /// with combat.)
    pub fn gain_exp(&mut self, amount: i32) -> u8 {
        if amount <= 0 {
            return 0;
        }
        let mut exp = i64::from(self.stats.exp) + i64::from(amount);
        let mut gained = 0;
        while let Some(needed) = crate::exp::to_next_level(self.stats.level) {
            if exp < needed {
                break;
            }
            exp -= needed;
            self.stats.level += 1;
            self.stats.ap = self.stats.ap.saturating_add(5);
            gained += 1;
        }
        if crate::exp::to_next_level(self.stats.level).is_none() {
            exp = 0;
        }
        self.stats.exp = i32::try_from(exp).unwrap_or(i32::MAX);
        self.dirty = true;
        gained
    }

    /// Change HP by `delta`, clamped to `0..=max_hp`. Returns the new HP.
    pub fn add_hp(&mut self, delta: i32) -> i16 {
        let hp = (i32::from(self.stats.hp) + delta).clamp(0, i32::from(self.stats.max_hp));
        self.stats.hp = hp as i16;
        self.dirty = true;
        self.stats.hp
    }

    /// Change MP by `delta`, clamped to `0..=max_mp`. Returns the new MP.
    pub fn add_mp(&mut self, delta: i32) -> i16 {
        let mp = (i32::from(self.stats.mp) + delta).clamp(0, i32::from(self.stats.max_mp));
        self.stats.mp = mp as i16;
        self.dirty = true;
        self.stats.mp
    }

    /// Flag in-memory changes that have not been persisted yet. Mutators on
    /// this type call this themselves; use it after editing public fields.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Called after a successful save of a snapshot taken from this player.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
pub(crate) fn test_stats() -> CharacterStats {
    CharacterStats {
        id: 7,
        name: "tester".to_string(),
        gender: 0,
        skin: 0,
        face: 20000,
        hair: 30030,
        level: 1,
        job: 0,
        str: 12,
        dex: 5,
        int: 4,
        luk: 4,
        hp: 50,
        max_hp: 50,
        mp: 5,
        max_mp: 5,
        ap: 0,
        sp: 0,
        exp: 0,
        fame: 0,
        map_id: 10000,
        spawn_point: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::equip_slot;

    #[test]
    fn cash_items_cover_regular_items_in_the_same_slot() {
        let look = AvatarLook::from_equipped(
            &test_stats(),
            [
                (equip_slot::TOP, 1040002),
                (equip_slot::SHOES, 1072001),
                (equip_slot::TOP - 100, 1042003),
                (equip_slot::CASH_WEAPON, 1702000),
            ],
        );
        assert_eq!(look.visible.get(&5), Some(&1042003));
        assert_eq!(look.visible.get(&7), Some(&1072001));
        assert_eq!(look.masked.get(&5), Some(&1040002));
        assert_eq!(look.cash_weapon, Some(1702000));
    }

    #[test]
    fn mutators_mark_the_player_dirty() {
        let mut player = Player::new(
            1,
            0,
            test_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        );
        assert!(!player.is_dirty());
        player.move_to(100000000, 2);
        assert!(player.is_dirty());
        assert_eq!(player.stats.spawn_point, 2);
        player.mark_saved();
        assert!(!player.is_dirty());
    }

    fn player() -> Player {
        Player::new(
            1,
            0,
            test_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        )
    }

    #[test]
    fn exp_levels_up_through_several_levels() {
        let mut player = player();
        player.stats.max_hp = 50;
        // Rain's seven quizzes pay 62 EXP: level 1 -> 3 with 13 left over.
        assert_eq!(player.gain_exp(62), 2);
        assert_eq!(
            (player.stats.level, player.stats.exp, player.stats.ap),
            (3, 13, 10)
        );
        assert_eq!(player.gain_exp(0), 0);
        assert!(player.is_dirty());
    }

    #[test]
    fn hp_and_mp_stay_within_bounds() {
        let mut player = player();
        player.stats.max_hp = 50;
        player.stats.hp = 25;
        assert_eq!(player.add_hp(30), 50);
        assert_eq!(player.add_hp(-80), 0);
        player.stats.max_mp = 5;
        assert_eq!(player.add_mp(10), 5);
    }
}
