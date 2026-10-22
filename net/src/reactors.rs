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
    /// Bring out an NPC on the ground below it, with a pink line (the text
    /// key `notice`) to everyone on its map.
    Appear { npc_id: i32, notice: &'static str },
    /// A moonflower on Henesys's Primrose Hill, which blooms on its seed:
    /// it stays in bloom, the map's full moon (`FULL_MOON`) waxes a phase,
    /// and once every moonflower there blooms the Moon Bunny comes out below
    /// the moon (Cosmic's 9108000-5).
    Bloom,
    /// One of the scars on the Tower of Goddess's hub (`2008000`-`2008005`),
    /// where a piece of Minerva's statue goes: it keeps the piece set in it.
    Hold,
    /// One of the fourteen spots on the Tower of Goddess's Salion stage
    /// (`2001002`-`2001015`), which breaks on the Cellion's drop: the
    /// field brings the next Cellion out at another spot (`salion_next`),
    /// until enough have gone that this one drops its loot and clears the
    /// stage (Cosmic `2001002`).
    Salion,
    /// A pot in the Tower of Goddess's garden (`2001000`, `2001001`), which
    /// breaks on a seed: a Nependeath or a Royal Nependeath grows out of
    /// it (`pot_mob`), unless Papa Pixie is already out.
    Pot,
    /// A spot under a garden pot (`2001016`), which breaks on the Royal
    /// Nependeath's seed: every mob in the garden goes, the pots grow no
    /// more, and Papa Pixie comes out at `PAPA_PIXIE_AT`, with a pink line.
    PapaPixie,
    /// Drop its loot, and clear the event stage on its map for whoever
    /// broke it: the garden's Grass of Life pot (Cosmic `2002003`).
    DropsAndClears,
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
/// The light on Orbis's cloud stage, which Eak comes out of.
pub const OPQ_LIGHT: i32 = 2006000;
/// The first of the Salion stage's spots, `2001002` to `2001015`.
pub const SALION_SPOTS: i32 = 2001002;
/// The Cellion that comes out at the Salion stage's spots.
pub const CELLION: i32 = 9300040;

/// The spot the `count`th Cellion comes out at, after the first, which the
/// map has at the first spot: Cosmic steps 11 spots on each time, so none
/// comes back to a spot before all fourteen are used.
pub fn salion_next(count: u32) -> i32 {
    SALION_SPOTS + (11 * count % 14) as i32
}

/// How many Cellions go before a spot drops its loot: 4 to 13, at random
/// as Cosmic draws it (`max(floor(random * 14), 4)`).
pub fn salion_limit(rng: &mut impl rand::Rng) -> u32 {
    rng.gen_range(0, 14).max(4)
}

/// The Nependeath and the Royal Nependeath, which grow out of the
/// garden's pots; Papa Pixie, and where he comes out (Cosmic `2001016`).
pub const NEPENDEATH: i32 = 9300048;
pub const ROYAL_NEPENDEATH: i32 = 9300049;
pub const PAPA_PIXIE: i32 = 9300039;
pub const PAPA_PIXIE_AT: (i16, i16) = (260, 490);

/// What grows out of the garden's `nth` pot to break (from 1): a Royal
/// Nependeath 40 times in a hundred for the first seven, then always
/// (Cosmic `2001000`, `statusStg7_c`). For testing, `RUSTMS_OPQ_POTS`
/// (a digit, 0-7) pins how many Nependeaths grow before the Royals.
pub fn pot_mob(nth: u32, rng: &mut impl rand::Rng) -> i32 {
    static PINNED: OnceLock<Option<u32>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        std::env::var("RUSTMS_OPQ_POTS")
            .ok()
            .and_then(|pots| pots.parse::<u32>().ok())
            .filter(|pots| *pots <= 7)
    });
    let royal = match pinned {
        Some(plain) => nth > plain,
        None => nth > 7 || rng.gen_range(0, 100) < 40,
    };
    if royal {
        ROYAL_NEPENDEATH
    } else {
        NEPENDEATH
    }
}

/// The music box in the Tower of Goddess's music room (Cosmic `2008006`).
/// It plays one record a day: the event set aside for its day is the only
/// one it reacts to (`OrbisPQ.js` sets its event state to the weekday as
/// the event starts), so another day's record stays on the floor.
pub const MUSIC: i32 = 2008006;
/// The music box's name on its map, by which Eak hears whether it plays.
pub const MUSIC_BOX: &str = "music";
/// Sunday's record; the other days' follow it in order.
pub const SUNDAY_RECORD: i32 = 4001056;

/// The day (Sunday is 0, as Cosmic counts) whose record the music box
/// plays at `now`: `RUSTMS_OPQ_DAY` for testing, else the weekday on the
/// rate cards' clock (`rates::weekday`).
pub fn music_day(now: std::time::SystemTime) -> u8 {
    static PINNED: OnceLock<Option<u8>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        std::env::var("RUSTMS_OPQ_DAY")
            .ok()
            .and_then(|day| day.parse::<u8>().ok())
            .filter(|day| *day < 7)
    });
    pinned.unwrap_or_else(|| (crate::rates::weekday(now) + 1) % 7)
}

/// Whether reactor `reactor_id` takes `item_id` on `day` (Sunday 0): the
/// music box only that day's record, every other reactor whatever its
/// events take.
pub fn takes(reactor_id: i32, item_id: i32, day: u8) -> bool {
    reactor_id != MUSIC || item_id == SUNDAY_RECORD + i32::from(day)
}

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
        // The boxes of a premium wedding's box room 680000401 (`6802000`,
        // `6802001`: `sprayItems(true, 1, 100, 400, 15)`).
        6802000 | 6802001 => Action::Spray {
            at_least: 15,
            mesos: (100, 400),
        },
        // Ludibrium's stage 2 trap box (`go922010201`).
        2200002 => Action::WarpMap {
            warp: Warp(LPQ_TRAP_ROOM, "sp"),
            notice: keys::LPQ_NOTICE_TRAP,
        },
        // Orbis's cloud stage (`2006000`): the light takes the cloud pieces
        // and Eak steps out of it.
        OPQ_LIGHT => Action::Appear {
            npc_id: crate::npc_scripts::EAK,
            notice: keys::OPQ_NOTICE_LIGHT,
        },
        2008000..=2008005 => Action::Hold,
        2001002..=2001015 => Action::Salion,
        // The garden's pots, the spots under them, and the Grass of Life.
        2001000 | 2001001 => Action::Pot,
        2001016 => Action::PapaPixie,
        2002003 => Action::DropsAndClears,
        _ => Action::Drops,
    }
}

/// The state a reactor starts in, from 0 for most. Orbis's light waits at
/// 20, the only state of v83's whose event takes an item: the 20 cloud
/// pieces dropped on it (Reactor.nx `2006000.img/20`).
pub fn first_state(reactor_id: i32) -> u8 {
    match reactor_id {
        OPQ_LIGHT => 20,
        _ => 0,
    }
}

/// Whether a reactor stays when it reaches its last state, rather than
/// going away: a moonflower stays in bloom, a scar keeps its piece.
pub fn stays(reactor_id: i32) -> bool {
    matches!(action(reactor_id), Action::Bloom | Action::Hold)
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
        assert!(matches!(
            action(6802001),
            Action::Spray {
                at_least: 15,
                mesos: (100, 400)
            }
        ));
        let Action::SpawnAt { mob_id, at, .. } = action(2201003) else {
            panic!("{:?}", action(2201003));
        };
        assert_eq!((mob_id, at), (LPQ_ALISHAR, (941, 184)));
        assert_eq!(action(2302003), Action::Drops);
        assert_eq!(action(2000), Action::Drops);
    }

    #[test]
    fn orbis_light_waits_for_its_cloud_pieces_then_brings_out_eak() {
        let Action::Appear { npc_id, .. } = action(OPQ_LIGHT) else {
            panic!("{:?}", action(OPQ_LIGHT));
        };
        assert_eq!(npc_id, crate::npc_scripts::EAK);
        assert_eq!((first_state(OPQ_LIGHT), first_state(2000)), (20, 0));
        let Some(light) = template(OPQ_LIGHT) else {
            return;
        };
        assert!(light.takes_items(20) && !light.takes_items(0));
        assert_eq!(light.item_dropped(20, 4001063, 20, (0, 0)), Some((0, 21)));
        assert!(light.is_last(21));
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
    fn the_tower_of_goddess_s_scars_keep_the_pieces_set_in_them() {
        for reactor_id in 2008000..=2008005 {
            assert_eq!(action(reactor_id), Action::Hold);
            assert!(stays(reactor_id));
        }
        assert_eq!(action(2008006), Action::Drops);
        let Some(scar) = template(2008003) else {
            return;
        };
        assert_eq!(scar.item_dropped(0, 4001044, 1, (0, 0)), Some((0, 1)));
        assert_eq!(scar.item_dropped(0, 4001048, 1, (0, 0)), None);
        assert!(scar.is_last(1));
    }

    #[test]
    fn the_gardens_pots_grow_royals_now_and_then_and_always_after_the_seventh() {
        assert_eq!(action(2001000), Action::Pot);
        assert_eq!(action(2001001), Action::Pot);
        assert_eq!(action(2001016), Action::PapaPixie);
        assert_eq!(action(2002003), Action::DropsAndClears);
        let rng = &mut rand::thread_rng();
        let first: Vec<i32> = (0..400).map(|_| pot_mob(1, rng)).collect();
        assert!(first.contains(&NEPENDEATH) && first.contains(&ROYAL_NEPENDEATH));
        assert!((0..50).all(|_| pot_mob(8, rng) == ROYAL_NEPENDEATH));
        let Some(pot) = template(2001000) else {
            return;
        };
        assert_eq!(pot.item_dropped(0, 4001053, 1, (0, 0)), Some((0, 1)));
        assert!(pot.is_last(1));
    }

    #[test]
    fn the_music_box_plays_only_the_days_record_counting_from_sunday() {
        // 1 January 1970 was a Thursday.
        assert_eq!(music_day(std::time::SystemTime::UNIX_EPOCH), 4);
        for day in 0..7 {
            let records = (0..7).filter(|d| takes(MUSIC, SUNDAY_RECORD + d, day));
            assert_eq!(records.collect::<Vec<_>>(), [i32::from(day)]);
        }
        assert!(takes(2001002, 4001074, 3));
    }

    #[test]
    fn the_cellions_come_out_at_every_salion_spot_but_the_first_in_turn() {
        for reactor_id in 2001002..=2001015 {
            assert_eq!(action(reactor_id), Action::Salion);
        }
        assert_eq!(action(2001017), Action::Drops);
        let spots: Vec<i32> = (1..14).map(salion_next).collect();
        assert_eq!(spots[..3], [2001013, 2001010, 2001007]);
        let mut seen = spots.clone();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 13);
        assert!(!spots.contains(&SALION_SPOTS));
        let rng = &mut rand::thread_rng();
        assert!((0..200).all(|_| (4..=13).contains(&salion_limit(rng))));
        let Some(spot) = template(2001002) else {
            return;
        };
        assert_eq!(spot.item_dropped(0, 4001074, 1, (0, 0)), Some((0, 1)));
        assert!(spot.is_last(1));
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
