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
    /// Send whoever broke it to a map's portal.
    Warp(Warp),
    /// Drop its loot `percent` times in a hundred, else send whoever broke
    /// it away.
    DropsOrWarp { percent: u32, warp: Warp },
}

/// A map, and the name of the portal there to arrive at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warp(pub i32, pub &'static str);

impl Warp {
    /// The map, and its portal's id.
    pub fn to(self) -> (i32, u8) {
        (self.0, crate::scripts::named(self.0, self.1))
    }
}

const HITMAN_HIDEOUT: i32 = 910200000;
const BERSERK_ROOM: i32 = 910500200;

pub fn action(reactor_id: i32) -> Action {
    match reactor_id {
        // The boxes in the Hitman's hideout: those that lead up the floors,
        // and those he hides in.
        1020000 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt00")),
        1020001 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt01")),
        1020002 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt02")),
        1021000 | 1021001 => Action::BringOut(9300091),
        // The Berserk room's four false statues: most throw the player out
        // to the Sleepy Dungeon below.
        1050000 => Action::DropsOrWarp {
            percent: 30,
            warp: Warp(105090200, "sp"),
        },
        // The Fire Hawk's and Ice Eagle's eggs (924000100).
        2401001 => Action::BringOut(9300089),
        2401002 => Action::BringOut(9300090),
        _ => Action::Drops,
    }
}

/// Whether a map's reactors change places each time it starts afresh: the
/// Berserk room's statues, so the true one is somewhere new.
pub fn shuffled(map_id: i32) -> bool {
    map_id == BERSERK_ROOM
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

    #[test]
    fn the_hitmans_boxes_lead_up_and_the_false_statues_mostly_throw_out() {
        assert_eq!(action(1020001), Action::Warp(Warp(HITMAN_HIDEOUT, "pt01")));
        let Action::DropsOrWarp { percent: 30, warp } = action(1050000) else {
            panic!("{:?}", action(1050000));
        };
        assert_eq!(action(1052000), Action::Drops);
        if crate::game_data::get().is_err() {
            return;
        }
        assert_eq!(Warp(HITMAN_HIDEOUT, "pt02").to(), (HITMAN_HIDEOUT, 3));
        assert_eq!(warp.to(), (105090200, 0));
        assert!(shuffled(BERSERK_ROOM) && !shuffled(HITMAN_HIDEOUT));
    }
}
