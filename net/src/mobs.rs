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
/// Zakum's last body: its death on its altar map wins the fight.
pub const ZAKUM_LAST: i32 = 8800002;
pub const ZAKUM_ALTAR_MAP: i32 = 280030000;

/// The Time Sphere the Piece of Cracked Dimension brings out on the Origin
/// of Clocktower (Mob.nx `revive`s it into the Papulatus Clock, 8500001,
/// and that into Papulatus).
pub const TIME_SPHERE: i32 = 8500000;
/// Papulatus himself: his death on the Origin of Clocktower wins the
/// fight.
pub const PAPULATUS_LAST: i32 = 8500002;
/// The Darkstars the Papulatus Clock calls up (skill 200 levels 39 and
/// 40), bombs that blow up when touched.
pub const HIGH_DARKSTAR: i32 = 8500003;
pub const LOW_DARKSTAR: i32 = 8500004;

/// What brings Horntail out on its cave (Cosmic `spawnHorntailOnGroundBelow`):
/// the intro, which leaves at once (`removeAfter`), its fall turning into
/// Horntail; then Horntail itself, `HORNTAIL`, whose HP is the boss bar
/// and takes no hits of its own; then its eight parts, `HORNTAIL_PARTS`,
/// heads, hands, legs, wings and tail, each hit on which comes off
/// Horntail's HP as well (`MobSet::link`). Each part's death lets out its
/// dead part (Mob.nx `revive`), 8810010 to 8810017.
pub const HORNTAIL_INTRO: i32 = 8810026;
pub const HORNTAIL: i32 = 8810018;
pub const HORNTAIL_PARTS: RangeInclusive<i32> = 8810002..=8810009;
/// What the parts leave: once all eight are out, Horntail dies.
pub const DEAD_HORNTAIL_PARTS: RangeInclusive<i32> = 8810010..=8810017;
/// The ground below which Horntail and its parts come out.
pub const HORNTAIL_AT: (i16, i16) = (71, 260);

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

    #[test]
    fn the_clock_calls_up_the_darkstars() {
        let Ok(_) = data() else { return };
        let high = skill_level(200, 39).unwrap();
        let low = skill_level(200, 40).unwrap();
        // Four at a time each, the High ones 150 to 500 px above the
        // clock, the Low ones up to 150 px.
        assert_eq!(high.summons, [HIGH_DARKSTAR; 4]);
        assert_eq!((high.lt, high.rb), ((-500, -500), (500, -150)));
        assert_eq!(low.summons, [LOW_DARKSTAR; 4]);
        assert_eq!((low.lt, low.rb), ((-500, -150), (500, 0)));
    }
}
