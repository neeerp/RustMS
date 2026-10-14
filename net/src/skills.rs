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
