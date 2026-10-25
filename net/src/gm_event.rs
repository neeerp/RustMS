//! The events a GM opens to a channel's players, who come by an Event
//! Assistant (report b170).

use crate::load::{self, asset_path};
use game_data::ox_quiz::OxQuiz;
use game_data::FieldTemplate;
use std::sync::OnceLock;

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
}
