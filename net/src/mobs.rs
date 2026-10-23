//! Mob templates (`Mob.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::mobs::{MobData, MobInfo};
use game_data::skills::MobSkillLevel;
use std::ops::RangeInclusive;
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

/// Zakum's body as it first comes out (Mob.nx `revive`s it into 8800001,
/// and that into 8800002), and its eight arms, which come out with it.
pub const ZAKUM_BODY: i32 = 8800000;
pub const ZAKUM_ARMS: RangeInclusive<i32> = 8800003..=8800010;

/// The mobs that keep `mob_id` from harm while any of them is out: Zakum's
/// arms its body (Cosmic `MapleMap.damageMonster`). Until then it has no
/// controller either; the last arm to die makes it real.
pub fn shielded_by(mob_id: i32) -> Option<RangeInclusive<i32>> {
    (mob_id == ZAKUM_BODY).then_some(ZAKUM_ARMS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zakums_arms_shield_only_its_first_body() {
        assert_eq!(shielded_by(ZAKUM_BODY), Some(8800003..=8800010));
        assert_eq!(shielded_by(8800001), None);
        assert_eq!(shielded_by(8800003), None);
    }
}
