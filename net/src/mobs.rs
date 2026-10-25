//! Mob templates (`Mob.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::footholds::Footholds;
use game_data::mobs::{MobData, MobInfo};
use game_data::skills::MobSkillLevel;
use game_data::MapMobTemplate;
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

/// A boss that keeps coming back to its spot on one map: an area boss.
/// The world's maps hold none of them, so each gets a spawn point of its
/// own, which `mobTime` brings back as any boss's (Cosmic's `AreaBoss*`
/// events give the maps, mobs and spots; a spot Cosmic picks at random
/// along a line is the line's middle here, and Snack Bar's the first of
/// its two). Its name is String.nx's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AreaBoss {
    pub map: i32,
    pub mob: i32,
    pub name: &'static str,
    /// It comes out on the ground below this point.
    pub at: (i16, i16),
}

/// An area boss's spawn point's `mobTime`, in seconds: that of Mushmom's
/// on Henesys Hunting Ground I, the like boss the world's maps do place.
/// A placeholder (QUESTIONS a117).
pub const AREA_BOSS_TIME: i32 = 1800;

const fn area_boss(map: i32, mob: i32, name: &'static str, at: (i16, i16)) -> AreaBoss {
    AreaBoss { map, mob, name, at }
}

pub const AREA_BOSSES: [AreaBoss; 20] = [
    area_boss(104000400, 2220000, "Mano", (279, -496)),
    area_boss(101030404, 3220000, "Stumpy", (800, 1280)),
    area_boss(260010201, 3220001, "Deo", (645, 275)),
    area_boss(230020100, 4220001, "Seruf", (-350, 520)),
    area_boss(110040000, 5220001, "King Clang", (-400, 140)),
    area_boss(100040105, 5220002, "Faust", (456, 278)),
    area_boss(100040106, 5220002, "Faust", (474, 278)),
    area_boss(220050100, 5220003, "Timer", (-385, 1030)),
    area_boss(220050000, 5220003, "Timer", (-300, 1030)),
    area_boss(220050200, 5220003, "Timer", (0, 1030)),
    area_boss(251010102, 5220004, "Giant Centipede", (560, 50)),
    area_boss(107000300, 6220000, "Dyle", (90, 119)),
    area_boss(221040301, 6220001, "Zeno", (-4224, 776)),
    area_boss(250010304, 7220000, "Tae Roon", (-450, 390)),
    area_boss(222010310, 7220001, "Nine-Tailed Fox", (-150, 33)),
    area_boss(250010504, 7220002, "King Sage Cat", (150, 540)),
    area_boss(200010300, 8220000, "Eliza", (208, 83)),
    area_boss(261030000, 8220002, "Chimera", (-450, 180)),
    area_boss(240040401, 8220003, "Leviathan", (0, 1125)),
    area_boss(105090310, 8220008, "Unknow Snack Bar", (-626, -604)),
];

/// The area boss `mob_id` on `map`, if it is one there.
pub fn area_boss_on(map: i32, mob_id: i32) -> Option<&'static AreaBoss> {
    AREA_BOSSES
        .iter()
        .find(|boss| boss.map == map && boss.mob == mob_id)
}

/// The spawn points of `map`'s area bosses, on the ground of `footholds`.
pub fn area_boss_spawns(map: i32, footholds: &Footholds) -> Vec<MapMobTemplate> {
    AREA_BOSSES
        .iter()
        .filter(|boss| boss.map == map)
        .filter_map(|boss| {
            let (x, y) = boss.at;
            let (ground, foothold) = footholds.below(x, y)?;
            Some(MapMobTemplate {
                mob_id: boss.mob,
                x,
                y: ground,
                foothold,
                flip: false,
                mob_time: AREA_BOSS_TIME,
                hide: false,
                team: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_bosses_come_out_on_their_maps_ground() {
        let Ok(data) = crate::get_game_data() else {
            return;
        };
        for boss in AREA_BOSSES {
            let field = data.field(boss.map).unwrap();
            let spawns = area_boss_spawns(boss.map, &field.footholds);
            let spawn = spawns.iter().find(|spawn| spawn.mob_id == boss.mob);
            let spawn = spawn.unwrap_or_else(|| panic!("{boss:?} has no ground"));
            // The ground right below the spot, not a floor further down.
            assert!((0..150).contains(&(spawn.y - boss.at.1)), "{boss:?}");
            assert!(info(boss.mob).is_some(), "{boss:?}");
            // The world's maps place none of them already.
            assert!(
                field.mob_spawns.iter().all(|s| s.mob_id != boss.mob),
                "{boss:?}"
            );
        }
        assert_eq!(area_boss_on(104000400, 2220000).unwrap().name, "Mano");
        assert_eq!(area_boss_on(104000400, 2220001), None);
        assert_eq!(area_boss_on(100000000, 2220000), None);
    }

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
