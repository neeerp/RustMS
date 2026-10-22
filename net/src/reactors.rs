//! Reactor templates (`Reactor.nx`), loaded once.

use crate::load::{self, asset_path};
use game_data::reactors::{ReactorData, ReactorTemplate};
use game_data::server_content::keys;
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
    /// Drop its loot with a pile of mesos (`mesos`, from and up to but not
    /// including to, as Cosmic rolls them): one pile first, and more after
    /// it until there are `at_least` drops.
    Spray { at_least: usize, mesos: (i32, i32) },
    /// Bring out `count` of `mob_id` on the ground below it instead.
    BringOut(i32, usize),
    /// Send whoever broke it to a map's portal.
    Warp(Warp),
    /// Drop its loot `percent` times in a hundred, else send whoever broke
    /// it away.
    DropsOrWarp { percent: u32, warp: Warp },
    /// Send everyone on its map away, with a pink line (the text key
    /// `notice`) first.
    WarpMap { warp: Warp, notice: &'static str },
    /// Bring out one `mob_id` on the ground below `at`, a fixed point on
    /// its map, with a pink line (the text key `notice`) to everyone there.
    SpawnAt {
        mob_id: i32,
        at: (i16, i16),
        notice: &'static str,
    },
    /// A moonflower on Henesys's Primrose Hill, which blooms on its seed:
    /// it stays in bloom, the map's full moon (`FULL_MOON`) waxes a phase,
    /// and once every moonflower there blooms the Moon Bunny comes out below
    /// the moon (Cosmic's 9108000-5).
    Bloom,
}

/// The name of the reactor the moonflowers wax.
pub const FULL_MOON: &str = "fullmoon";
/// The Moon Bunny, who makes rice cakes.
pub const MOON_BUNNY: i32 = 9300061;

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
const LPQ_TRAP_ROOM: i32 = 922010201;
const LPQ_BLOCTOPUS: i32 = 9300007;
const LPQ_ROMBAD: i32 = 9300010;
const LPQ_ALISHAR: i32 = 9300012;

pub fn action(reactor_id: i32) -> Action {
    match reactor_id {
        // The boxes in the Hitman's hideout: those that lead up the floors,
        // and those he hides in.
        1020000 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt00")),
        1020001 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt01")),
        1020002 => Action::Warp(Warp(HITMAN_HIDEOUT, "pt02")),
        1021000 | 1021001 => Action::BringOut(9300091, 1),
        // The Berserk room's four false statues: most throw the player out
        // to the Sleepy Dungeon below.
        1050000 => Action::DropsOrWarp {
            percent: 30,
            warp: Warp(105090200, "sp"),
        },
        // The Fire Hawk's and Ice Eagle's eggs (924000100).
        2401001 => Action::BringOut(9300089, 1),
        2401002 => Action::BringOut(9300090, 1),
        9108000..=9108005 => Action::Bloom,
        // Ludibrium's stage 3 boxes (`ludiMob1`): three Bloctopuses come out.
        2201001 => Action::BringOut(LPQ_BLOCTOPUS, 3),
        // Stage 7's (`ludiMob2`), which take the other pass, and the
        // boss's (`ludiBoss0`), which takes a pass.
        2201002 => Action::SpawnAt {
            mob_id: LPQ_ROMBAD,
            at: (1, -211),
            notice: keys::LPQ_NOTICE_ROMBAD,
        },
        2201003 => Action::SpawnAt {
            mob_id: LPQ_ALISHAR,
            at: (941, 184),
            notice: keys::LPQ_NOTICE_ALISHAR,
        },
        // The boxes on Ludibrium's bonus stage (`ludiquest4`:
        // `sprayItems(true, 1, 30, 60, 15)`).
        2202004 => Action::Spray {
            at_least: 15,
            mesos: (30, 60),
        },
        // Ludibrium's stage 2 trap box (`go922010201`).
        2200002 => Action::WarpMap {
            warp: Warp(LPQ_TRAP_ROOM, "sp"),
            notice: keys::LPQ_NOTICE_TRAP,
        },
        _ => Action::Drops,
    }
}

/// Whether a reactor stays when it reaches its last state, rather than
/// going away: a moonflower stays in bloom.
pub fn stays(reactor_id: i32) -> bool {
    action(reactor_id) == Action::Bloom
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
        assert_eq!(action(2401001), Action::BringOut(9300089, 1));
        assert_eq!(action(2401002), Action::BringOut(9300090, 1));
        assert_eq!(action(2201001), Action::BringOut(LPQ_BLOCTOPUS, 3));
        assert!(matches!(
            action(2202004),
            Action::Spray { at_least: 15, .. }
        ));
        let Action::SpawnAt { mob_id, at, .. } = action(2201003) else {
            panic!("{:?}", action(2201003));
        };
        assert_eq!((mob_id, at), (LPQ_ALISHAR, (941, 184)));
        assert_eq!(action(2302003), Action::Drops);
        assert_eq!(action(2000), Action::Drops);
    }

    #[test]
    fn primrose_hills_six_moonflowers_bloom_and_stay() {
        for reactor_id in 9108000..=9108005 {
            assert_eq!(action(reactor_id), Action::Bloom);
            assert!(stays(reactor_id));
        }
        assert!(!stays(9108006) && !stays(9101000) && !stays(9102002));
    }

    #[test]
    fn the_hitmans_boxes_lead_up_and_the_false_statues_mostly_throw_out() {
        assert_eq!(action(1020001), Action::Warp(Warp(HITMAN_HIDEOUT, "pt01")));
        let Action::DropsOrWarp { percent: 30, warp } = action(1050000) else {
            panic!("{:?}", action(1050000));
        };
        assert_eq!(action(1052000), Action::Drops);
        let Action::WarpMap { warp: trap, .. } = action(2200002) else {
            panic!("{:?}", action(2200002));
        };
        assert_eq!(action(2202003), Action::Drops);
        if crate::game_data::get().is_err() {
            return;
        }
        assert_eq!(Warp(HITMAN_HIDEOUT, "pt02").to(), (HITMAN_HIDEOUT, 3));
        assert_eq!(warp.to(), (105090200, 0));
        assert_eq!(trap.to(), (LPQ_TRAP_ROOM, 0));
        assert!(shuffled(BERSERK_ROOM) && !shuffled(HITMAN_HIDEOUT));
    }
}
