//! The events a GM opens to a channel's players, who come by an Event
//! Assistant (report b170).

use game_data::FieldTemplate;

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
