//! Mob templates (`Mob.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::mobs::{MobData, MobInfo};
use game_data::skills::MobSkillLevel;
use std::sync::OnceLock;

pub fn data() -> Result<&'static MobData, &'static str> {
    static MOBS: OnceLock<Result<MobData, String>> = OnceLock::new();
    load::once(&MOBS, || {
        MobData::load(asset_path("RUSTMS_MOB_NX_PATH", "assets/game-data/Mob.nx"))
    })
}

pub fn info(mob_id: i32) -> Option<MobInfo> {
    data().ok()?.info(mob_id).ok().flatten()
}

/// A mob skill's level (`Skill.nx` `MobSkill.img/<skill>/level/<level>`).
pub fn skill_level(skill: i32, level: i32) -> Option<MobSkillLevel> {
    crate::skills::data()
        .ok()?
        .mob_skill(skill, level)
        .ok()
        .flatten()
}
