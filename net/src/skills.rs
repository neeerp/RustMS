//! Skill templates (`Skill.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::skills::{SkillData, SkillInfo};
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
