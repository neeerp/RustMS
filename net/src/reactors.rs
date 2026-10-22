//! Reactor templates (`Reactor.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::reactors::{ReactorData, ReactorTemplate};
use std::sync::OnceLock;

pub fn data() -> Result<&'static ReactorData, &'static str> {
    static REACTORS: OnceLock<Result<ReactorData, String>> = OnceLock::new();
    load::once(&REACTORS, || {
        ReactorData::load(asset_path(
            "RUSTMS_REACTOR_NX_PATH",
            "assets/game-data/Reactor.nx",
        ))
    })
}

pub fn template(reactor_id: i32) -> Option<ReactorTemplate> {
    data().ok()?.template(reactor_id).ok().flatten()
}

/// What a reactor does when it breaks: Cosmic's reactor scripts, keyed by
/// reactor id as there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Drop its loot (`reactor_drops`), as every reactor not listed does.
    Drops,
    /// Bring out one `mob_id` on the ground below it instead.
    BringOut(i32),
}

pub fn action(reactor_id: i32) -> Action {
    match reactor_id {
        // The boxes in the Hitman's hideout (910200000).
        1021000 | 1021001 => Action::BringOut(9300091),
        // The Fire Hawk's and Ice Eagle's eggs (924000100).
        2401001 => Action::BringOut(9300089),
        2401002 => Action::BringOut(9300090),
        _ => Action::Drops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eggs_hatch_and_other_reactors_drop_their_loot() {
        assert_eq!(action(2401001), Action::BringOut(9300089));
        assert_eq!(action(2401002), Action::BringOut(9300090));
        assert_eq!(action(2302003), Action::Drops);
        assert_eq!(action(2000), Action::Drops);
    }
}
