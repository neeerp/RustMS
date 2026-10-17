//! Skill templates (`Skill.nx`), loaded once.

use crate::load::{self, asset_path};
use game::Player;
use game_data::skills::{SkillData, SkillInfo, SkillLevel};
use std::sync::OnceLock;

pub fn data() -> Result<&'static SkillData, &'static str> {
    static SKILLS: OnceLock<Result<SkillData, String>> = OnceLock::new();
    load::once(&SKILLS, || {
        SkillData::load(asset_path(
            "RUSTMS_SKILL_NX_PATH",
            "assets/game-data/Skill.nx",
        ))
    })
}

pub fn info(skill_id: i32) -> Option<SkillInfo> {
    data().ok()?.info(skill_id).ok().flatten()
}

/// A skill the player has, at the level they have it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Learned {
    pub skill_id: i32,
    pub level: i32,
    pub effect: SkillLevel,
}

pub fn learned(player: &Player, skill_id: i32) -> Option<Learned> {
    let level = player.skills.level(skill_id);
    let effect = info(skill_id)?.level(level)?.clone();
    Some(Learned {
        skill_id,
        level,
        effect,
    })
}

/// What the v83 client heals on its own at rest (FUN_0083007f), before the
/// map's `info/recovery` rate and a chair's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalRecovery {
    /// HP per heal: 10, plus a warrior's Improved HP Recovery `hp` or a
    /// thief's Endure `hp`.
    pub hp: i32,
    /// MP per heal: 3, plus a magician's skill level in Improved MP Recovery
    /// times their level over 10, a thief's Endure `mp`, or a Crusader's or
    /// White Knight's Improved MP Recovery `mp`.
    pub mp: i32,
    /// Milliseconds between HP heals: 10000, or 5000 for a warrior with
    /// Improved HP Recovery. MP always heals every 10000.
    pub hp_interval: u64,
}

pub const IMPROVED_HP_RECOVERY: i32 = 1000000;
pub const IMPROVED_MP_RECOVERY: i32 = 2000000;
pub const ASSASSIN_ENDURE: i32 = 4100002;
pub const BANDIT_ENDURE: i32 = 4200001;
pub const FIGHTER_IMPROVED_MP_RECOVERY: i32 = 1110000;
pub const PAGE_IMPROVED_MP_RECOVERY: i32 = 1210000;

pub fn natural_recovery(player: &Player) -> NaturalRecovery {
    let job = i32::from(player.stats.job);
    let effect = |skill_id| learned(player, skill_id).map(|skill| skill.effect);
    let hp_skill = if job / 100 == 1 {
        effect(IMPROVED_HP_RECOVERY)
    } else if in_line(job, 410) {
        effect(ASSASSIN_ENDURE)
    } else if in_line(job, 420) {
        effect(BANDIT_ENDURE)
    } else {
        None
    };
    let mp_bonus = if job / 100 == 2 {
        player.skills.level(IMPROVED_MP_RECOVERY) * i32::from(player.stats.level) / 10
    } else {
        let skill_id = [
            (410, ASSASSIN_ENDURE),
            (420, BANDIT_ENDURE),
            (111, FIGHTER_IMPROVED_MP_RECOVERY),
            (121, PAGE_IMPROVED_MP_RECOVERY),
        ]
        .into_iter()
        .find(|&(line, _)| in_line(job, line))
        .map(|(_, skill_id)| skill_id);
        skill_id.and_then(effect).map_or(0, |effect| effect.mp)
    };
    let hp_interval = if job / 100 == 1 && player.skills.level(IMPROVED_HP_RECOVERY) > 0 {
        5000
    } else {
        10000
    };
    NaturalRecovery {
        hp: 10 + hp_skill.map_or(0, |effect| effect.hp),
        mp: 3 + mp_bonus,
        hp_interval,
    }
}

/// The percent Element Amplification scales the player's spells by (its
/// `y`), or 100 without it (v83 FUN_00765771).
pub fn amplification(player: &Player) -> i32 {
    let job = i32::from(player.stats.job);
    [(211, 2110001), (221, 2210001), (1211, 12110001)]
        .into_iter()
        .find(|&(line, _)| in_line(job, line))
        .and_then(|(_, skill_id)| learned(player, skill_id))
        .map_or(100, |skill| skill.effect.y)
}

/// Whether `job` is `base` or advanced from it, as the v83 client checks:
/// 410 covers 410 to 412, and 100 every warrior.
fn in_line(job: i32, base: i32) -> bool {
    if base % 100 == 0 {
        base / 100 == job / 100
    } else {
        base / 10 == job / 10 && base % 10 <= job % 10
    }
}

/// Skills held down to charge or keep firing (the v83 client's
/// FUN_004fb08f).
pub const KEYDOWN: [i32; 16] = [
    1121001,  // Monster Magnet (Hero)
    1221001,  // Monster Magnet (Paladin)
    1321001,  // Monster Magnet (Dark Knight)
    2121001,  // Big Bang (Fire/Poison)
    2221001,  // Big Bang (Ice/Lightning)
    2321001,  // Big Bang (Bishop)
    3121004,  // Hurricane
    3221001,  // Piercing Arrow
    5101004,  // Corkscrew Blow
    5201002,  // Grenade
    5221004,  // Rapid Fire
    13111002, // Hurricane (Wind Archer)
    14111006, // Poison Bomb
    15101003, // Corkscrew Blow (Thunder Breaker)
    22121000, // Evan
    22151001, // Evan
];

/// The held spells, whose MAGIC_ATTACK says how long they were charged.
pub const CHARGED_SPELLS: [i32; 5] = [2121001, 2221001, 2321001, 22121000, 22151001];

pub fn keydown(skill_id: i32) -> bool {
    KEYDOWN.contains(&skill_id)
}

/// Monster Magnet, for each warrior branch: held, then let go with the mobs
/// it caught.
pub const MONSTER_MAGNETS: [i32; 3] = [1121001, 1221001, 1321001];

pub fn is_magnet(skill_id: i32) -> bool {
    MONSTER_MAGNETS.contains(&skill_id)
}

/// Explosion and Chakra prepare too, but by a timer rather than a held key.
pub const TIMED: [i32; 2] = [2111002, 4211001];

/// Whether the client prepares the skill first, with SKILL_EFFECT.
pub fn prepares(skill_id: i32) -> bool {
    keydown(skill_id) || TIMED.contains(&skill_id)
}
