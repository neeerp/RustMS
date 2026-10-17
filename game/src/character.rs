use crate::buff::{Buffs, ShownBuff};
use crate::inventory::{equip_slot, Inventory};
use crate::keymap::Keymap;
use crate::monster_book::MonsterBook;
use crate::quest::QuestLog;
use crate::skill::{self, SkillBook, SkillRecord};
use std::collections::BTreeMap;

/// The most STR, DEX, INT, or LUK AP can raise a stat to.
pub const MAX_BASE_STAT: i16 = 999;
/// The most max HP or max MP AP can raise.
pub const MAX_HP_MP: i16 = 30_000;
/// The most SP the three beginner skills can share (one per level from 2).
pub const MAX_BEGINNER_SP: i32 = 6;

/// A stat `SPEND_AP` can raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApStat {
    Str,
    Dex,
    Int,
    Luk,
    MaxHp,
    MaxMp,
}

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
    /// The portable chair (a 301xxxx setup item) the player sits on, or 0.
    pub chair: i32,
    /// The buffs others see, as `Buffs::shown`.
    pub buffs: Vec<ShownBuff>,
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
    pub monster_book: MonsterBook,
    /// Account GM level; 0 for regular players.
    pub gm_level: i16,
    /// Active buffs and cooldowns; not saved.
    pub buffs: Buffs,
    /// The portable chair the player sits on, or 0; not saved.
    pub chair: i32,
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
            monster_book: MonsterBook::default(),
            gm_level: 0,
            buffs: Buffs::default(),
            chair: 0,
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
            chair: 0,
            buffs: self.buffs.shown(),
        }
    }

    /// Record that the character entered `map_id` at `spawn_point`.
    pub fn move_to(&mut self, map_id: i32, spawn_point: u8) {
        self.stats.map_id = map_id;
        self.stats.spawn_point = spawn_point;
        self.dirty = true;
    }

    /// Record where the character should appear when they next enter
    /// their current map (after logout or a channel change).
    pub fn set_spawn_point(&mut self, spawn_point: u8) {
        if self.stats.spawn_point != spawn_point {
            self.stats.spawn_point = spawn_point;
            self.dirty = true;
        }
    }

    /// Mutable access to the keymap; marks the player as needing a save.
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        self.dirty = true;
        &mut self.keymap
    }

    /// Mutable access to the Monster Book; marks the player as needing a
    /// save.
    pub fn monster_book_mut(&mut self) -> &mut MonsterBook {
        self.dirty = true;
        &mut self.monster_book
    }

    /// Mutable access to the inventory; marks the player as needing a save.
    pub fn inventory_mut(&mut self) -> &mut Inventory {
        self.dirty = true;
        &mut self.inventory
    }

    /// Add EXP, levelling up as many times as it covers. Each level grants
    /// 5 AP, 3 SP past beginner, and the job family's max HP/MP growth;
    /// current HP and MP stay as they were, as in v83. Returns the number of
    /// levels gained.
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
            if self.stats.job % 1000 != 0 {
                self.stats.sp = self.stats.sp.saturating_add(3);
            }
            let (hp, mp) = level_up_growth(self.stats.job);
            self.stats.max_hp = self.stats.max_hp.saturating_add(hp);
            self.stats.max_mp = self.stats.max_mp.saturating_add(mp);
            gained += 1;
        }
        if crate::exp::to_next_level(self.stats.level).is_none() {
            exp = 0;
        }
        self.stats.exp = i32::try_from(exp).unwrap_or(i32::MAX);
        self.dirty = true;
        gained
    }

    /// Spend one AP on `stat`. Returns false, changing nothing, without AP
    /// or once the stat is at its cap.
    pub fn spend_ap(&mut self, stat: ApStat) -> bool {
        if self.stats.ap <= 0 {
            return false;
        }
        let (hp, mp) = ap_growth(self.stats.job);
        let stats = &mut self.stats;
        let (value, gain, cap) = match stat {
            ApStat::Str => (&mut stats.str, 1, MAX_BASE_STAT),
            ApStat::Dex => (&mut stats.dex, 1, MAX_BASE_STAT),
            ApStat::Int => (&mut stats.int, 1, MAX_BASE_STAT),
            ApStat::Luk => (&mut stats.luk, 1, MAX_BASE_STAT),
            ApStat::MaxHp => (&mut stats.max_hp, hp, MAX_HP_MP),
            ApStat::MaxMp => (&mut stats.max_mp, mp, MAX_HP_MP),
        };
        if *value >= cap {
            return false;
        }
        *value = value.saturating_add(gain).min(cap);
        stats.ap -= 1;
        self.dirty = true;
        true
    }

    /// SP left for the three beginner skills: one per level from 2 up to
    /// six, less what they already have.
    pub fn beginner_sp(&self) -> i32 {
        let base = i32::from(self.stats.job) / 1000 * 10_000_000;
        let used: i32 = (1000..=1002).map(|id| self.skills.level(base + id)).sum();
        (i32::from(self.stats.level) - 1).min(MAX_BEGINNER_SP) - used
    }

    /// Raise `skill_id` a level, up to `max_level`, paying one SP (or one
    /// beginner SP for the beginner skills). Fourth-job skills stop at the
    /// master level the character was granted instead. Returns the new
    /// record, or `None` if the skill can't go higher or there is no SP.
    /// Which skills the character may learn is the caller's to check.
    pub fn raise_skill(&mut self, skill_id: i32, max_level: i32) -> Option<SkillRecord> {
        let mut record = self.skills.get(skill_id).cloned().unwrap_or(SkillRecord {
            skill_id,
            level: 0,
            master_level: 0,
            expiration: None,
        });
        let cap = if skill::is_fourth_job(skill_id) {
            record.master_level.min(max_level)
        } else {
            max_level
        };
        if record.level >= cap {
            return None;
        }
        if skill::uses_beginner_sp(skill_id) {
            if self.beginner_sp() <= 0 {
                return None;
            }
        } else if self.stats.sp <= 0 {
            return None;
        } else {
            self.stats.sp -= 1;
        }
        record.level += 1;
        self.skills.set(record.clone());
        self.dirty = true;
        Some(record)
    }

    /// Change HP by `delta`, clamped to `0..=max_hp`. Returns the new HP.
    pub fn add_hp(&mut self, delta: i32) -> i16 {
        let hp = (i32::from(self.stats.hp) + delta).clamp(0, i32::from(self.stats.max_hp));
        self.stats.hp = hp as i16;
        self.dirty = true;
        self.stats.hp
    }

    /// Recover `hp` from a potion or a skill, halved while Undead holds, as
    /// its notice says (and Cosmic's ZOMBIFY does). Returns the new HP.
    pub fn recover_hp(&mut self, hp: i32) -> i16 {
        if self.buffs.has(crate::buff::BuffStat::Undead) {
            self.add_hp(hp / 2)
        } else {
            self.add_hp(hp)
        }
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

/// Max HP/MP gained per level: the middle of v83's random ranges
/// (`MapleCharacter.levelUp`), so levels are reproducible.
fn level_up_growth(job: i16) -> (i16, i16) {
    match job / 100 {
        0 | 10 | 20 => (14, 11),
        1 | 11 | 21 => (26, 5),
        2 | 12 => (12, 23),
        3 | 4 | 13 | 14 => (22, 15),
        5 | 15 => (25, 20),
        _ => (14, 11),
    }
}

/// Max HP/MP one AP adds: the middle of v83's random ranges, as with
/// `level_up_growth`.
fn ap_growth(job: i16) -> (i16, i16) {
    match job / 100 {
        1 | 11 | 21 => (22, 3),
        2 | 12 => (8, 19),
        3 | 4 | 13 | 14 => (18, 11),
        5 | 15 => (20, 15),
        _ => (10, 7),
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
        player.set_spawn_point(2);
        assert!(!player.is_dirty());
        player.set_spawn_point(4);
        assert!(player.is_dirty());
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
        player.stats.hp = 40;
        // Rain's seven quizzes pay 62 EXP: level 1 -> 3 with 13 left over.
        assert_eq!(player.gain_exp(62), 2);
        assert_eq!(
            (player.stats.level, player.stats.exp, player.stats.ap),
            (3, 13, 10)
        );
        // Two beginner levels: +28 max HP, +22 max MP, no refill.
        assert_eq!((player.stats.max_hp, player.stats.hp), (78, 40));
        assert_eq!((player.stats.max_mp, player.stats.mp), (27, 5));
        assert_eq!(player.gain_exp(0), 0);
        assert!(player.is_dirty());
    }

    #[test]
    fn levels_past_beginner_grant_sp() {
        let mut player = player();
        player.gain_exp(62);
        assert_eq!(player.stats.sp, 0);
        player.stats.job = 200;
        player.gain_exp(57 + 92 - 13);
        assert_eq!((player.stats.level, player.stats.sp), (5, 6));
    }

    #[test]
    fn ap_raises_one_stat_until_it_runs_out_or_caps() {
        let mut player = player();
        player.stats.ap = 3;
        assert!(player.spend_ap(ApStat::Str));
        assert_eq!((player.stats.str, player.stats.ap), (13, 2));
        assert!(player.is_dirty());
        player.stats.job = 200;
        assert!(player.spend_ap(ApStat::MaxMp));
        assert_eq!((player.stats.max_mp, player.stats.ap), (24, 1));
        player.stats.luk = MAX_BASE_STAT;
        assert!(!player.spend_ap(ApStat::Luk));
        player.stats.max_hp = MAX_HP_MP - 1;
        assert!(player.spend_ap(ApStat::MaxHp));
        assert_eq!((player.stats.max_hp, player.stats.ap), (MAX_HP_MP, 0));
        assert!(!player.spend_ap(ApStat::Dex));
        assert_eq!(player.stats.dex, 5);
    }

    #[test]
    fn beginner_skills_share_one_sp_per_level_up_to_six() {
        let mut player = player();
        assert_eq!(player.raise_skill(1000, 3), None);
        player.stats.level = 3;
        assert_eq!(player.raise_skill(1000, 3).unwrap().level, 1);
        assert_eq!(player.raise_skill(1001, 3).unwrap().level, 1);
        assert_eq!(player.raise_skill(1002, 3), None);
        assert_eq!(player.stats.sp, 0);
        player.stats.level = 30;
        assert_eq!(player.beginner_sp(), 4);
        player.raise_skill(1000, 3);
        player.raise_skill(1000, 3);
        assert_eq!(player.raise_skill(1000, 3), None);
        // A magician keeps the pool for the skills it never raised.
        player.stats.job = 200;
        player.stats.sp = 1;
        assert_eq!(player.raise_skill(1002, 3).unwrap().level, 1);
        assert_eq!((player.beginner_sp(), player.stats.sp), (1, 1));
    }

    #[test]
    fn skills_cost_sp_and_stop_at_their_cap() {
        let mut player = player();
        player.stats.job = 200;
        player.stats.sp = 2;
        assert_eq!(player.raise_skill(2001002, 1).unwrap().level, 1);
        assert_eq!(player.raise_skill(2001002, 1), None);
        assert_eq!(player.stats.sp, 1);
        assert_eq!(player.raise_skill(2000000, 10).unwrap().level, 1);
        assert_eq!(player.raise_skill(2000000, 10), None);
        assert_eq!(player.skills.level(2000000), 1);
    }

    #[test]
    fn fourth_job_skills_stop_at_their_master_level() {
        let mut player = player();
        player.stats.job = 212;
        player.stats.sp = 5;
        assert_eq!(player.raise_skill(2121000, 30), None);
        player.skills.set(SkillRecord {
            skill_id: 2121000,
            level: 0,
            master_level: 1,
            expiration: None,
        });
        assert_eq!(player.raise_skill(2121000, 30).unwrap().master_level, 1);
        assert_eq!(player.raise_skill(2121000, 30), None);
        assert_eq!(player.stats.sp, 4);
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
