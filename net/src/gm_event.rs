//! The events a GM opens to a channel's players, who come by an Event
//! Assistant (report b170).

use crate::load::{self, asset_path};
use game_data::ox_quiz::OxQuiz;
use game_data::server_content::keys;
use game_data::FieldTemplate;
use std::sync::OnceLock;
use std::time::Duration;

/// The portals of a Snowball field's balls, Story's on the lower lane and
/// Maple's on the upper (`snowBall/<team>/portal`).
pub const SNOWBALL_SPOTS: [&str; 2] = ["st00", "st01"];

/// The portal a player on `team` lands on as they come to the game on
/// `field`: their ball's on a Snowball field, and on a Coconut Season
/// field any of its spawn points (`sp`), by `roll`. The field's first
/// spawn point otherwise.
pub fn spot(field: &FieldTemplate, team: u8, roll: usize) -> u8 {
    let id = if field.snowball.is_some() {
        SNOWBALL_SPOTS
            .get(usize::from(team))
            .and_then(|name| field.portal_by_name(name))
            .map(|portal| portal.id)
    } else {
        let spawns: Vec<_> = field.portals().filter(|p| p.name == "sp").collect();
        (!spawns.is_empty()).then(|| spawns[roll % spawns.len()].id)
    };
    id.and_then(|id| u8::try_from(id).ok()).unwrap_or(0)
}

/// The Snowball fields, each with its entrance map ("Event Map Entrance",
/// fieldType 7) just after it.
const SNOWBALL_FIELDS: [i32; 3] = [109_060_000, 109_060_002, 109_060_004];

/// Where the Event Assistants take players to the event on `map_id`: a
/// Snowball field's entrance, where they wait for the GM's `/start`.
pub fn lobby(map_id: i32) -> Option<i32> {
    SNOWBALL_FIELDS.contains(&map_id).then_some(map_id + 1)
}

/// The game field a GM's `/start` in the lobby `map_id` sends its players
/// to.
pub fn lobby_field(map_id: i32) -> Option<i32> {
    SNOWBALL_FIELDS
        .iter()
        .copied()
        .find(|&field| field + 1 == map_id)
}

/// A GM event that runs on a clock across its maps once the GM starts it
/// (a FieldSet, report b172): the gate in its lobby opens, its notices go
/// out on time, and when the time is up everyone still inside goes out.
#[derive(Debug)]
pub struct TimedEvent {
    /// Its maps, the lobby first.
    pub maps: &'static [i32],
    /// The clock the players see.
    pub length: Duration,
    /// Each notice, and when it goes out after the start.
    pub notices: &'static [(Duration, &'static str)],
}

impl TimedEvent {
    /// When those still inside go out: a second after the clock runs out.
    pub fn end(&self) -> Duration {
        self.length + Duration::from_secs(1)
    }
}

const fn secs(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

/// The Maple Physical Fitness Test (FieldSet `Event6`): a lobby and four
/// stages, from 109040004 on to 109050000, in 900 s.
pub const FITNESS: TimedEvent = TimedEvent {
    maps: &[
        109_040_000,
        109_040_001,
        109_040_002,
        109_040_003,
        109_040_004,
    ],
    length: secs(900),
    notices: &[
        (secs(1), keys::FITNESS_NOTICE[0]),
        (secs(30), keys::FITNESS_NOTICE[1]),
        (secs(60), keys::FITNESS_NOTICE[2]),
        (secs(120), keys::FITNESS_NOTICE[3]),
        (secs(200), keys::FITNESS_NOTICE[4]),
        (secs(240), keys::FITNESS_NOTICE[5]),
        (secs(300), keys::FITNESS_NOTICE[6]),
        (secs(400), keys::FITNESS_NOTICE[7]),
        (secs(540), keys::FITNESS_NOTICE[8]),
        (secs(600), keys::FITNESS_NOTICE[9]),
        (secs(660), keys::FITNESS_NOTICE[10]),
        (secs(800), keys::FITNESS_NOTICE[11]),
        (secs(890), keys::FITNESS_NOTICE[12]),
    ],
};

/// Ola Ola's notices, the same in each of its FieldSets.
const OLA_NOTICES: &[(Duration, &str)] = &[
    (secs(1), keys::OLA_NOTICE[0]),
    (secs(30), keys::OLA_NOTICE[1]),
    (secs(60), keys::OLA_NOTICE[2]),
    (secs(90), keys::OLA_NOTICE[3]),
    (secs(120), keys::OLA_NOTICE[4]),
    (secs(180), keys::OLA_NOTICE[5]),
    (secs(240), keys::OLA_NOTICE[6]),
    (secs(300), keys::OLA_NOTICE[7]),
    (secs(350), keys::OLA_NOTICE[8]),
];

const fn ola(maps: &'static [i32]) -> TimedEvent {
    TimedEvent {
        maps,
        length: secs(360),
        notices: OLA_NOTICES,
    }
}

/// Ola Ola's five variants (FieldSets `Event1` to `Event5`): three levels
/// each, the first also where players wait, on to [`WON_MAP`] in 360 s.
/// Variant 1's levels end in a row of `rand_ola` portals
/// (`scripts::rand_ola`); the others' lead on by Map.nx.
pub const OLA: [TimedEvent; 5] = [
    ola(&[109_030_001, 109_030_002, 109_030_003]),
    ola(&[109_030_101, 109_030_102, 109_030_103]),
    ola(&[109_030_201, 109_030_202, 109_030_203]),
    ola(&[109_030_301, 109_030_302, 109_030_303]),
    ola(&[109_030_401, 109_030_402, 109_030_403]),
];

/// Find the Jewel (FieldSet `Event8`, the Treasure Hunt): the lobby
/// 109010000, where players wait, its halls 109010100 and 109010200, and
/// the rooms off them, 109010101-110 and 109010201-206, in 600 s. The
/// halls' boxes (`net::reactors`) send players to the rooms, bring out
/// monsters, or drop loot, the Treasure Scroll among it.
pub const JEWEL: TimedEvent = TimedEvent {
    maps: &[
        109_010_000,
        109_010_100,
        109_010_101,
        109_010_102,
        109_010_103,
        109_010_104,
        109_010_105,
        109_010_106,
        109_010_107,
        109_010_108,
        109_010_109,
        109_010_110,
        109_010_200,
        109_010_201,
        109_010_202,
        109_010_203,
        109_010_204,
        109_010_205,
        109_010_206,
    ],
    length: secs(600),
    notices: &[
        (secs(1), keys::JEWEL_NOTICE[0]),
        (secs(600), keys::JEWEL_NOTICE[1]),
    ],
};

/// The timed event `map_id` is one of the maps of.
pub fn timed_event(map_id: i32) -> Option<&'static TimedEvent> {
    [&FITNESS, &JEWEL]
        .into_iter()
        .chain(&OLA)
        .find(|event| event.maps.contains(&map_id))
}

/// The portal of a timed event's lobby that stays closed until the GM
/// starts it. It leads within the lobby, but v83 asks the server to go
/// through a portal of its type (4) all the same (report b173).
pub const GATE: &str = "join00";

/// Whether `portal` on `map_id` is a timed event's gate.
pub fn is_gate(map_id: i32, portal: &str) -> bool {
    portal == GATE && timed_event(map_id).is_some_and(|event| event.maps[0] == map_id)
}

/// Where those still inside a timed event go when its time is up: its
/// maps' return map.
pub const EXIT_MAP: i32 = 109_050_001;

/// Where a timed event's winners come out, as Ola Ola's last portals
/// lead.
pub const WON_MAP: i32 = 109_050_000;

/// The OX Quiz's questions (`Etc.nx`), loaded on first use.
pub fn ox_quiz() -> Option<&'static OxQuiz> {
    static QUIZ: OnceLock<Result<OxQuiz, String>> = OnceLock::new();
    load::once(&QUIZ, || {
        OxQuiz::load(asset_path("RUSTMS_ETC_NX_PATH", "assets/game-data/Etc.nx"))
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_snowball_field_has_its_lobby_and_coconut_season_none() {
        for (field, entrance) in [
            (109_060_000, 109_060_001),
            (109_060_002, 109_060_003),
            (109_060_004, 109_060_005),
        ] {
            assert_eq!(lobby(field), Some(entrance));
            assert_eq!(lobby_field(entrance), Some(field));
        }
        assert_eq!(lobby(109_080_000), None);
        assert_eq!(lobby_field(109_060_000), None);
        assert_eq!(lobby_field(109_080_001), None);
    }

    #[test]
    fn the_fitness_test_runs_on_its_five_maps_and_its_gate_is_in_the_lobby() {
        for map_id in 109_040_000..=109_040_004 {
            assert!(timed_event(map_id).is_some_and(|event| event.maps.contains(&map_id)));
        }
        assert!(timed_event(109_050_000).is_none());
        assert!(timed_event(109_020_001).is_none());
        assert!(is_gate(109_040_000, "join00"));
        assert!(!is_gate(109_040_000, "in00"));
        assert!(!is_gate(109_040_001, "join00"));
        assert!(!is_gate(109_020_001, "join00"));
        assert_eq!(FITNESS.end(), Duration::from_secs(901));
        assert!(FITNESS.notices.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(FITNESS.notices.iter().all(|(at, _)| *at < FITNESS.length));
    }

    #[test]
    fn find_the_jewel_runs_on_its_lobby_halls_and_rooms() {
        for map_id in (109_010_000..=109_010_000)
            .chain(109_010_100..=109_010_110)
            .chain(109_010_200..=109_010_206)
        {
            assert_eq!(
                timed_event(map_id).map(|event| event.maps),
                Some(JEWEL.maps)
            );
        }
        assert!(timed_event(109_010_111).is_none());
        assert!(is_gate(109_010_000, "join00"));
        assert!(!is_gate(109_010_100, "join00"));
        assert_eq!(JEWEL.end(), Duration::from_secs(601));
        assert!(JEWEL.notices.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn each_ola_variant_runs_on_its_three_levels_and_its_gate_is_on_the_first() {
        for first in [
            109_030_001,
            109_030_101,
            109_030_201,
            109_030_301,
            109_030_401,
        ] {
            for level in first..first + 3 {
                let event = timed_event(level).unwrap();
                assert_eq!(event.maps, [first, first + 1, first + 2]);
                assert_eq!(event.length, Duration::from_secs(360));
            }
            assert!(is_gate(first, "join00"));
            assert!(!is_gate(first + 1, "join00"));
        }
        assert_eq!(OLA[0].end(), Duration::from_secs(361));
        assert!(OLA_NOTICES.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(OLA_NOTICES.iter().all(|(at, _)| *at < OLA[0].length));
    }
}
