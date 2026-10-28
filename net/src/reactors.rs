//! Reactor templates (`Reactor.nx`), loaded once.

use crate::events::{EOS_TOWER_100, EOS_TOWER_100_RETURN};
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
    /// A Monster Carnival guardian: its side's mobs lose what it gave them
    /// (Cosmic `mcGuardian0`'s `dispelAllMonsters`).
    Guardian,
    /// Drop its loot with a pile of mesos (`mesos`, from and up to but not
    /// including to, as Cosmic rolls them): one pile first, and more after
    /// it until there are `at_least` drops.
    Spray { at_least: usize, mesos: (i32, i32) },
    /// Bring out `count` of `mob_id` on the ground below it instead.
    BringOut(i32, usize),
    /// Bring out `count` of `mob_id` on the ground below `at`, a fixed point
    /// on its map, instead.
    BringOutAt {
        mob_id: i32,
        count: usize,
        at: (i16, i16),
    },
    /// Bring out two lots of mobs, each `count` of `mob_id`, on the ground
    /// below it instead: the Pirate Ship's mob boxes (Cosmic `2511000`).
    BringOutLots([(i32, usize); 2]),
    /// On each hit, drop the trap of its number: a lever `leverN` of the
    /// Orbis party quest's jails sets off `trapN`, its spears (Cosmic
    /// `2008007`'s `moveEnvironment`). The tall room's levers, named
    /// `1`..`5`, set off nothing.
    Lever,
    /// Send whoever broke it to a map's portal.
    Warp(Warp),
    /// Send whoever broke it to one of these maps, at random: Find the
    /// Jewel's boxes (`eventMap0` to `eventMap2`, whose `info` names them).
    WarpAnyOf(&'static [i32]),
    /// Drop its loot `percent` times in a hundred, else send whoever broke
    /// it away.
    DropsOrWarp { percent: u32, warp: Warp },
    /// Send everyone on its map away, with a pink line (the text key
    /// `notice`) first.
    WarpMap { warp: Warp, notice: &'static str },
    /// Bring out one `mob_id` on the ground below `at`, a fixed point on
    /// its map, with a pink line (the text key `notice`) to everyone there,
    /// and `music`, if any, playing.
    SpawnAt {
        mob_id: i32,
        at: (i16, i16),
        notice: &'static str,
        music: Option<&'static str>,
    },
    /// Bring out an NPC on the ground below it, with a pink line (the text
    /// key `notice`) to everyone on its map.
    Appear { npc_id: i32, notice: &'static str },
    /// A moonflower on Henesys's Primrose Hill, which blooms on its seed:
    /// it stays in bloom, the map's full moon (`FULL_MOON`) waxes a phase,
    /// and once every moonflower there blooms the Moon Bunny comes out below
    /// the moon (Cosmic's 9108000-5).
    Bloom,
    /// A statue in the Guild Quest's hall of statues: a strike counts
    /// toward the guild's guess of the statue keeper's combo
    /// (`guild_quest::Statues`).
    Statue,
    /// One of the scars on the Tower of Goddess's hub (`2008000`-`2008005`),
    /// where a piece of Minerva's statue goes: it keeps the piece set in it.
    /// The Guild Quest's stone and metal gates (`9208006`, `9208005`) and
    /// the waterway's doors (`9208010`-`9208012`) keep theirs too, and stay
    /// open.
    Hold,
    /// One of the Guild Quest's spears (`9208007`), which takes a spear and
    /// stays: the knights' hall's gate moves to the number set, and the
    /// last opens it and clears the stage (`guild_quest::SPEAR_GATE`).
    Spear,
    /// The King's Corridor's door (`guild_quest::KING_GATE_REACTOR`), which
    /// takes Sharen III's earring and stays open: the dead of the guild's
    /// quest revive in it from then on (Cosmic `9208009`'s `canRevive`).
    KingGate,
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
    /// Bring out an NPC on the ground below it, and clear the event stage
    /// on its map for whoever broke it: Minerva's statue on the Tower of
    /// Goddess's hub (Cosmic `2006001`), whose stage wins the event.
    AppearAndClears(i32),
    /// Drop its loot as `Spray` does, and bring out an NPC on the ground
    /// below `at`, a fixed point on its map, unless it is already out: the
    /// boxes of Orbis's treasure room (Cosmic `2002014`), the first of which
    /// brings out Minerva.
    SprayAndAppear {
        at_least: usize,
        mesos: (i32, i32),
        npc_id: i32,
        at: (i16, i16),
    },
    /// Zakum's altar (Cosmic `2111001`), which breaks on the Eye of Fire:
    /// Zakum's music plays, its body comes out on the ground below, not yet
    /// real, then its eight arms, with a pink line to everyone there.
    Zakum,
    /// The stone on Horntail's Cave (Cosmic `2401000`), which breaks on its
    /// sixth hit: Horntail's music plays, Horntail comes out unless it is
    /// already coming (`crate::mobs::HORNTAIL_INTRO`), and the expedition's
    /// clock starts over; then a line to everyone there.
    Horntail,
    /// The seal of one of the Trial Grounds' rooms (Cosmic `2408002`),
    /// which breaks on the room's key: a pink line to everyone in the room,
    /// the room's stage clears for whoever threw it or someone still
    /// there, and the event sends the same key to the first map (the event
    /// actor's `send_room_key`).
    PassKey,
    /// Set a record (`game::quest::RECORDS`) to `value` for whoever broke
    /// it while `quest` is started, with a pink line (the text key
    /// `notice`): the real Bart of Lord Jonathan's test (Cosmic `1209000`).
    Record {
        quest: i16,
        record: i16,
        value: &'static str,
        notice: &'static str,
    },
    /// One of the Ludibrium Maze's boxes (Cosmic `8091000` to `8091004`
    /// and `8098000`): two lots of mobs (`maze_box`) come out where it was,
    /// with a pink line to everyone there.
    MazeBox,
    /// One of the seals on the Pirate Ship's doors (Cosmic `2519000` to
    /// `2519003`), which takes an Old Metal Key and stays: the spawn points
    /// within `half` of it each way spawn no more, and if the other seals
    /// are set and no monster is left, the stage clears.
    Seal { half: (i16, i16) },
    /// Give everyone on its map this much EXP as it takes an item, and
    /// stay: the Forest of Poison Haze's fountain (Cosmic `3008000`, whose
    /// `hit` gives 52,000), which its states' timeouts empty again.
    ExpToAll(i32),
    /// One of the three beakers of a Magatia beaker room (Cosmic
    /// `2618000`), which takes a Magic Stone a state at a time and drains
    /// a state back each 9.5 seconds: the first time each reaches
    /// `BEAKER_FULL` counts, and the third counted clears the stage
    /// (`crate::events::MAGATIA_BEAKERS`).
    Beaker,
    /// One of a Magatia hub's two doors (Cosmic `2618001`, `2618002`),
    /// which opens for two seconds as it takes a Card Key, and opens the
    /// way back from the room it lets in to for as long
    /// (`crate::events::MagatiaSide::side_doors`).
    SideDoor,
}

/// The maze's boxes that bring out the same mobs in every room, from
/// `8091000` on: two each of the two mobs.
const MAZE_BOXES: [[i32; 2]; 5] = [
    [9400210, 9400209],
    [9400211, 9400212],
    [9400213, 9400214],
    [9400215, 9400216],
    [9400217, 9400218],
];
/// The maze's box that brings out mobs by the room it is in.
const MAZE_ROOM_BOX: i32 = 8098000;

/// The mobs one of the maze's boxes (`Action::MazeBox`) brings out on map
/// `map_id`, and how many of each. The room box brings out six Miner
/// Zombies and nine Coolie Zombies on the first, eleventh and fifteenth
/// rooms, and elsewhere two and three of the pair of mobs as far down from
/// the Flyeye and Tauromacis as the room is from the sixth (Cosmic
/// `8098000`).
pub fn maze_box(reactor_id: i32, map_id: i32) -> [(i32, usize); 2] {
    use crate::events::MAZE_ROOM;
    if reactor_id != MAZE_ROOM_BOX {
        let [a, b] = MAZE_BOXES[(reactor_id - 8091000).clamp(0, 4) as usize];
        return [(a, 2), (b, 2)];
    }
    match map_id - MAZE_ROOM {
        0 | 10 | 14 => [(9400209, 6), (9400210, 9)],
        room => {
            let b = (room - 5).abs();
            [(9400217 - b, 2), (9400218 - b, 3)]
        }
    }
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
/// Find the Jewel's monster, which its `eventMob0` boxes bring out.
const JEWEL_MOB: i32 = 9100001;
const BERSERK_ROOM: i32 = 910500200;
const LPQ_TRAP_ROOM: i32 = 922010201;
const LPQ_BLOCTOPUS: i32 = 9300007;
const LPQ_ROMBAD: i32 = 9300010;
const LPQ_ALISHAR: i32 = 9300012;
/// The light on Orbis's cloud stage, which Eak comes out of.
pub const OPQ_LIGHT: i32 = 2006000;
/// Minerva's statue on the Tower of Goddess's hub, which takes the Grass
/// of Life.
pub const OPQ_STATUE: i32 = 2006001;
/// The levers of the tall room and the jails under the sealed room.
pub const OPQ_LEVER: i32 = 2008007;
/// The crystal boxes of the jails' box rooms: the plain one of the rooms
/// no way leads to, and the critical one of those the levers open.
pub const OPQ_JAIL_BOX: i32 = 2002017;
pub const OPQ_CRITICAL_BOX: i32 = 2002018;
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

/// Whether reactor `reactor_id`, named `name` on its map, takes `item_id`
/// on `day` (Sunday 0): the music box only that day's record, a Trial
/// Grounds room's seal (`passKey1` to `passKey4`) only that room's key, as
/// Cosmic pins its event, and every other reactor whatever its events take.
pub fn takes(reactor_id: i32, name: &str, item_id: i32, day: u8) -> bool {
    match reactor_id {
        MUSIC => item_id == SUNDAY_RECORD + i32::from(day),
        HTPQ_PASS_KEY => {
            name.strip_prefix("passKey")
                .and_then(|room| room.parse::<u8>().ok())
                .and_then(crate::events::htpq_room_key)
                == Some(item_id)
        }
        _ => true,
    }
}

pub fn action(reactor_id: i32) -> Action {
    match reactor_id {
        // Find the Jewel's boxes, by their `action` in Reactor.nx; its
        // `eventItem` boxes drop their loot.
        9000000 => Action::WarpAnyOf(&[109010104, 109010105, 109010106, 109010107, 109010108]),
        9000001 => Action::WarpAnyOf(&[109010109, 109010110]),
        9000002 => Action::WarpAnyOf(&[
            109010201, 109010202, 109010203, 109010204, 109010205, 109010206,
        ]),
        9001000 => Action::BringOut(JEWEL_MOB, 1),
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
        9208000..=9208002 => Action::Statue,
        // The Guild Quest's metal and stone gates and the waterway's doors
        // (their `9208005`, `9208006`, `9208010`-`9208012` scripts do
        // nothing), its spears, and the gargoyles' altar (`9201000`).
        9208005 | 9208006 | 9208010..=9208012 => Action::Hold,
        crate::guild_quest::SPEAR => Action::Spear,
        crate::guild_quest::KING_GATE_REACTOR => Action::KingGate,
        // The grave's altar, once dressed (Cosmic `9201001`).
        crate::guild_quest::CLOTHES_ALTAR => Action::Appear {
            npc_id: crate::guild_quest::SHAREN_SOUL,
            notice: keys::GUILD_QUEST_SOUL_APPEARS,
        },
        crate::guild_quest::GARGOYLE_ALTAR => Action::BringOutAt {
            mob_id: crate::guild_quest::JR_GARGOYLE,
            count: crate::guild_quest::JR_GARGOYLES,
            at: crate::guild_quest::JR_GARGOYLES_AT,
        },
        // Ludibrium's stage 3 boxes (`ludiMob1`): three Bloctopuses come out.
        2201001 => Action::BringOut(LPQ_BLOCTOPUS, 3),
        // Stage 7's (`ludiMob2`), which take the other pass, and the
        // boss's (`ludiBoss0`), which takes a pass.
        2201002 => Action::SpawnAt {
            mob_id: LPQ_ROMBAD,
            at: (1, -211),
            notice: keys::LPQ_NOTICE_ROMBAD,
            music: None,
        },
        2201003 => Action::SpawnAt {
            mob_id: LPQ_ALISHAR,
            at: (941, 184),
            notice: keys::LPQ_NOTICE_ALISHAR,
            music: None,
        },
        // The cracked dimension on the Origin of Clocktower (`boss`, Cosmic
        // `2201004`), which takes a Piece of Cracked Dimension (4031179).
        PAPULATUS_CRACK => Action::SpawnAt {
            mob_id: crate::mobs::TIME_SPHERE,
            at: (-410, -400),
            notice: keys::PAPULATUS_SUMMONED,
            music: Some("Bgm09/TimeAttack"),
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
        // Eos Tower's dollhouse room: every dollhouse but the odd one
        // (`go221024400`, whose `info` names the map) throws the player
        // back out by Olson, which ends their run.
        DOLLHOUSE => Action::WarpMap {
            warp: Warp(EOS_TOWER_100, EOS_TOWER_100_RETURN),
            notice: keys::DOLLHOUSE_WRONG,
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
        8091000..=8091004 | MAZE_ROOM_BOX => Action::MazeBox,
        // The Pirate Ship's boxes on its first stage: three of the Lord
        // Pirate's Ginseng Jars (9300109) and five of his Bellflowers
        // (9300110) out of a mob box, and
        // items out of an item box (`dropItems(true, 1, 30, 60, 15)`).
        PIRATE_MOB_BOX => Action::BringOutLots([(9300109, 3), (9300110, 5)]),
        // Six of his Krus and six of his Captains out of a mob box on a
        // deck (Cosmic `2511001`).
        PIRATE_DECK_BOX => Action::BringOutLots([(9300124, 6), (9300125, 6)]),
        // A bellflower room's chest, opened with the Lord Pirate's key: its
        // loot topped up with piles of 50 to 99 mesos (Cosmic `2512001`,
        // `sprayItems(true, 1, 50, 100, 15)`).
        PIRATE_CHEST => Action::Spray {
            at_least: 15,
            mesos: (50, 100),
        },
        PIRATE_ITEM_BOX => Action::Spray {
            at_least: 15,
            mesos: (30, 60),
        },
        OPQ_LEVER => Action::Lever,
        // The jails' boxes (`2002017`, `2002018`: `sprayItems(true, 1, 100,
        // 400, 15)`).
        OPQ_JAIL_BOX | OPQ_CRITICAL_BOX => Action::Spray {
            at_least: 15,
            mesos: (100, 400),
        },
        // A door's seal: `setAllowSpawnPointInBox(false, ..)` over
        // `Rectangle(x - 160, y - 75, 320, 150)`.
        PIRATE_SEAL..=PIRATE_LAST_SEAL => Action::Seal { half: (160, 75) },
        2008000..=2008005 => Action::Hold,
        // The statue (`2006001`): the Grass of Life brings Minerva out, and
        // wins the event.
        OPQ_STATUE => Action::AppearAndClears(crate::npc_scripts::MINERVA),
        2001002..=2001015 => Action::Salion,
        // The garden's pots, the spots under them, and the Grass of Life.
        2001000 | 2001001 => Action::Pot,
        2001016 => Action::PapaPixie,
        // The Forest of Poison Haze's spine, and the fifth stage's box that
        // holds the Purple Stone of Magic (Cosmic `3002001`); its eight
        // empty ones (3002002) drop nothing.
        2002003 | ELLIN_SPINE | ELLIN_STONE_BOX => Action::DropsAndClears,
        // The sixth stage's altar (Cosmic `3001000`), which takes the stone.
        ELLIN_ALTAR => Action::SpawnAt {
            mob_id: crate::events::ELLIN_GOLEM,
            at: crate::events::ELLIN_ALTAR_AT,
            notice: keys::ELLIN_ALTAR_GOLEM,
            music: None,
        },
        ELLIN_FOUNTAIN => Action::ExpToAll(crate::events::ELLIN_FOUNTAIN_EXP),
        MAGATIA_BEAKER => Action::Beaker,
        MAGATIA_SIDE_DOOR..=MAGATIA_LAST_SIDE_DOOR => Action::SideDoor,
        // The treasure room's boxes (`2002014`: `dropItems(true, 1, 100,
        // 400, 15)`, then Minerva, once).
        2002014 => Action::SprayAndAppear {
            at_least: 15,
            mesos: (100, 400),
            npc_id: crate::npc_scripts::MINERVA,
            at: crate::events::OPQ_BONUS_MINERVA_AT,
        },
        // The Dead Mine's (Cosmic `2110000`, `2111000`): back to the hub,
        // or three Mimics out of the box.
        ZPQ_TRAP => Action::Warp(Warp(crate::events::ZPQ_HUB, "sp")),
        ZPQ_MIMIC_BOX => Action::BringOut(ZPQ_MIMIC, 3),
        ZAKUM_ALTAR => Action::Zakum,
        HORNTAIL_STONE => Action::Horntail,
        HTPQ_PASS_KEY => Action::PassKey,
        // The real Bart among Lord Jonathan's gulls; the false ones
        // (1200000) drop nothing.
        9_980_000 | 9_980_001 => Action::Guardian,
        REAL_BART => Action::Record {
            quest: JONATHANS_COURT,
            record: BART_RECORD,
            value: BART_FOUND,
            notice: keys::JONATHAN_BART_FOUND,
        },
        _ => Action::Drops,
    }
}

/// The dollhouses of Eos Tower's dollhouse room that are only dollhouses.
pub const DOLLHOUSE: i32 = 2200000;

/// The Dead Mine's boxes that send whoever breaks one back to the hub, and
/// those that hold Mimics.
pub const ZPQ_TRAP: i32 = 2110000;
pub const ZPQ_MIMIC_BOX: i32 = 2111000;
pub const ZPQ_MIMIC: i32 = 9300004;

/// The cracked dimension where Papulatus comes out.
pub const PAPULATUS_CRACK: i32 = 2201004;

/// Zakum's altar, and the music it plays as it brings Zakum out.
pub const ZAKUM_ALTAR: i32 = 2111001;
pub const ZAKUM_MUSIC: &str = "Bgm06/FinalFight";

/// The stone on Horntail's Cave (`boss`), and the music it plays.
pub const HORNTAIL_STONE: i32 = 2401000;
pub const HORNTAIL_MUSIC: &str = "Bgm14/HonTale";

/// The seals of the Trial Grounds' rooms 1 to 4 (`passKey1` to `passKey4`).
pub const HTPQ_PASS_KEY: i32 = 2408002;
/// The lamp of the Trial Grounds' lamp room: each hit moves it on, from
/// dark through lit and back (0, 1, 2, 3, then 0), and it never breaks; the
/// room's way on reads it (`scripts::htpq_lamp_door`).
pub const HTPQ_LAMP: i32 = 2408001;

/// Lord Jonathan's test, a room of ten gulls, one of them the real Bart,
/// for "Jonathan's Court" (6400), which reads the record he sets.
pub const BART_ROOM: i32 = 912020000;
pub const REAL_BART: i32 = 1209000;
pub const JONATHANS_COURT: i16 = 6400;
/// The record the real Bart sets (`game::quest::RECORDS`), and what it
/// reads once he is found (6400's `Check/1 infoex`).
pub const BART_RECORD: i16 = 6401;
pub const BART_FOUND: &str = "q3";

/// The Forest of Poison Haze's second stage: its pond (Cosmic `3002000`,
/// `dropItems`) takes a Concentrated Poison, drops a Diluted Poison, and
/// fills again; its spine takes four Diluted Poisons and, staying, clears
/// the stage (`3009000`, `showClearEffect` in state 4).
pub const ELLIN_POND: i32 = 3002000;
pub const ELLIN_SPINE: i32 = 3009000;
/// Its fifth stage's box with the Purple Stone of Magic, among eight empty
/// ones, and its sixth's altar, which brings out the Poison Golem.
pub const ELLIN_STONE_BOX: i32 = 3002001;
pub const ELLIN_ALTAR: i32 = 3001000;
/// The prize map's fountain, which takes a Golem's Marble; its map names
/// it "", and the way home asks its state.
pub const ELLIN_FOUNTAIN: i32 = 3008000;
pub const ELLIN_FOUNTAIN_NAME: &str = "";

/// A Magatia beaker, and the state at which it counts as full (Cosmic
/// `2618000`'s `hit`); one Magic Stone more fills it to its last.
pub const MAGATIA_BEAKER: i32 = 2618000;
pub const BEAKER_FULL: u8 = 6;

/// A Magatia hub's doors to the rooms beside it, and those rooms' ways
/// back, which stay where they are as the rooms' boxes are shuffled.
pub const MAGATIA_SIDE_DOOR: i32 = 2618001;
pub const MAGATIA_LAST_SIDE_DOOR: i32 = 2618002;
pub const MAGATIA_ROOM_EXITS: std::ops::RangeInclusive<i32> = 2618003..=2618004;

/// The Pirate Ship's mob box, which takes four hits, and its item box and
/// the decks' mob box, which link to it.
pub const PIRATE_MOB_BOX: i32 = 2511000;
pub const PIRATE_ITEM_BOX: i32 = 2512000;
pub const PIRATE_DECK_BOX: i32 = 2511001;
/// The chest in each of the Pirate Ship's bellflower rooms, hidden until
/// Guon shows it, then opened by dropping the Lord Pirate's key (4031437)
/// on it, and the rooms' names for it, in order.
pub const PIRATE_CHEST: i32 = 2512001;
pub const PIRATE_CHEST_NAMES: [&str; 2] = ["treasure1", "treasure2"];
/// The seals on the doors of the Pirate Ship's fifth stage (2519000 to
/// 2519003), each taking an Old Metal Key (4001117), and their names.
pub const PIRATE_SEAL: i32 = 2519000;
pub const PIRATE_LAST_SEAL: i32 = 2519003;
pub const PIRATE_SEAL_NAMES: [&str; 4] = ["sMob1", "sMob2", "sMob3", "sMob4"];

/// Portals whose target spawn portal the target map lacks, so the target's
/// default spawn stands in: the Bart room's way out names "ao00", which
/// Nautilus' Navigation Room (120000102) has no portal by, and the way out
/// of Ardin's room names "st00", which Ardin's house (260000200) lacks.
const LOST_LINKS: [(i32, &str); 2] = [(BART_ROOM, "ntq00"), (crate::scripts::ARDINS_ROOM, "st00")];

/// Whether `map_id`'s `portal` leads to the target map's default spawn.
pub fn lost_link(map_id: i32, portal: &str) -> bool {
    LOST_LINKS.contains(&(map_id, portal))
}

/// The pink line whoever broke reactor `reactor_id` reads first, if any
/// (Cosmic `playerMessage(5, ..)`): the Dead Mine's.
pub fn breaker_notice(reactor_id: i32) -> Option<&'static str> {
    match reactor_id {
        ZPQ_TRAP => Some(keys::ZAKUM_MINE_UNKNOWN_FORCE),
        ZPQ_MIMIC_BOX => Some(keys::ZAKUM_MINE_MIMICS),
        _ => None,
    }
}

/// The state a reactor starts in, from 0 for most. Orbis's light waits at
/// 20, whose event takes the 20 cloud pieces dropped on it (Reactor.nx
/// `2006000.img/20`). Minerva's statue
/// waits at 6, whose event takes the Grass of Life (`2006001.img/6`; its
/// states 0 to 5 take nothing, and the scars hold the pieces instead).
/// The jails' critical box starts at 1: its state 0, a blank that fades
/// it in, has no event, so nothing would ever let it be hit (Cosmic leaves
/// it so; no script moves it).
pub fn first_state(reactor_id: i32) -> u8 {
    match reactor_id {
        OPQ_LIGHT => 20,
        OPQ_STATUE => 6,
        OPQ_CRITICAL_BOX => 1,
        _ => 0,
    }
}

/// Whether a reactor stays when it reaches its last state, rather than
/// going away: a moonflower stays in bloom, a scar keeps its piece.
pub fn stays(reactor_id: i32) -> bool {
    matches!(
        action(reactor_id),
        Action::Bloom | Action::Hold | Action::Spear | Action::KingGate
    ) || acts_staying(reactor_id)
}

/// Whether a reactor, reaching its end state (its last, or one whose own
/// event of the index it came by leads back:
/// `ReactorTemplate::acts_in`), does what breaking would and stays: the
/// item-triggered ones Cosmic's `hitReactor` triggers rather than destroys
/// and whose script acts there, the Forest of Poison Haze's pond and spine
/// and the Pirate Ship's seals. Other reactors with such end states (in NX
/// many, with no action here) only move.
pub fn acts_staying(reactor_id: i32) -> bool {
    matches!(
        reactor_id,
        ELLIN_POND | ELLIN_SPINE | PIRATE_SEAL..=PIRATE_LAST_SEAL
            | MAGATIA_SIDE_DOOR..=MAGATIA_LAST_SIDE_DOOR
    )
}

/// Whether a map's reactors change places each time it starts afresh: the
/// Berserk room's statues, Lord Jonathan's gulls, Eos Tower's dollhouses
/// and the Toy Factory's parts boxes (Server.NX `FieldSet.img/Ludi023`'s and
/// `Ludi020`'s `shuffle`), so the true ones are
/// somewhere new, the boxes of the Ludibrium Maze's rooms (Cosmic
/// `LudiMazePQ` `shuffleReactors`), and the Forest of Poison Haze's fifth
/// stage's (`EllinPQ`), and those of the Pirate Ship's first, third and
/// fourth stages, where invisible boxes (`2518000`) hold places for the
/// real ones (`PiratePQ`), and those of the two rooms beside a Magatia
/// hub, where the experiment files are (`MagatiaPQ`'s `shuffleReactors`
/// of `2518000` to `2612004`, so not their ways back: `shuffles`).
pub fn shuffled(map_id: i32) -> bool {
    use crate::events::{
        MagatiaSide, DOLLHOUSE_ROOM, ELLIN_STAGES, MAZE_GOAL, MAZE_ROOM, PARTS_ROOM, PIRATE_STAGES,
    };
    let magatia = MagatiaSide::ALL
        .iter()
        .flat_map(|side| side.side_doors())
        .any(|(_, room, _)| room == map_id);
    matches!(
        map_id,
        BERSERK_ROOM | BART_ROOM | DOLLHOUSE_ROOM | PARTS_ROOM
    ) || (MAZE_ROOM..MAZE_GOAL).contains(&map_id)
        || map_id == ELLIN_STAGES[4]
        || [PIRATE_STAGES[0], PIRATE_STAGES[2], PIRATE_STAGES[3]].contains(&map_id)
        || magatia
}

/// Whether reactor `reactor_id` changes places with the others on a
/// `shuffled` map: all but a Magatia side room's way back.
pub fn shuffles(reactor_id: i32) -> bool {
    !MAGATIA_ROOM_EXITS.contains(&reactor_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_dollhouse_throws_the_player_out_and_the_room_is_shuffled() {
        assert_eq!(
            action(DOLLHOUSE),
            Action::WarpMap {
                warp: Warp(EOS_TOWER_100, "q000"),
                notice: keys::DOLLHOUSE_WRONG,
            }
        );
        assert_eq!(action(2202000), Action::Drops, "the odd one");
        assert!(shuffled(crate::events::DOLLHOUSE_ROOM));
        assert!(shuffles(DOLLHOUSE) && shuffles(2202000));
        // The Toy Factory's parts boxes and their empty look-alikes.
        assert!(shuffled(crate::events::PARTS_ROOM));
        assert_eq!(action(2202001), Action::Drops);
        assert_eq!(action(2208000), Action::Drops);
    }

    #[test]
    fn find_the_jewels_boxes_send_players_to_the_rooms_their_info_names() {
        assert_eq!(
            action(9000000),
            Action::WarpAnyOf(&[109010104, 109010105, 109010106, 109010107, 109010108])
        );
        assert_eq!(action(9000001), Action::WarpAnyOf(&[109010109, 109010110]));
        let Action::WarpAnyOf(rooms) = action(9000002) else {
            panic!("{:?}", action(9000002));
        };
        assert_eq!(rooms, (109010201..=109010206).collect::<Vec<_>>());
        assert_eq!(action(9001000), Action::BringOut(JEWEL_MOB, 1));
        for item_box in 9002000..=9002002 {
            assert_eq!(action(item_box), Action::Drops);
        }
        if data().is_err() {
            return;
        }
        for (reactor_id, name) in [
            (9000000, "eventMap0"),
            (9000001, "eventMap1"),
            (9000002, "eventMap2"),
            (9001000, "eventMob0"),
            (9002002, "eventItem2"),
        ] {
            let template = template(reactor_id).unwrap();
            assert_eq!(template.action.as_deref(), Some(name));
        }
    }

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
        let Action::SpawnAt {
            mob_id, at, music, ..
        } = action(PAPULATUS_CRACK)
        else {
            panic!("{:?}", action(PAPULATUS_CRACK));
        };
        assert_eq!(
            (mob_id, at, music),
            (8500000, (-410, -400), Some("Bgm09/TimeAttack"))
        );
        assert_eq!(action(2302003), Action::Drops);
        assert_eq!(action(2000), Action::Drops);
    }

    #[test]
    fn the_jails_levers_drop_spears_and_their_boxes_spray() {
        assert_eq!(action(OPQ_LEVER), Action::Lever);
        for statue in crate::guild_quest::STATUES {
            assert_eq!(action(statue), Action::Statue);
        }
        let spray = Action::Spray {
            at_least: 15,
            mesos: (100, 400),
        };
        assert_eq!(
            (action(OPQ_JAIL_BOX), action(OPQ_CRITICAL_BOX)),
            (spray, spray)
        );
        assert_eq!(
            (first_state(OPQ_JAIL_BOX), first_state(OPQ_CRITICAL_BOX)),
            (0, 1)
        );
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
    fn minervas_statue_takes_the_grass_of_life_and_brings_her_out() {
        let minerva = Action::AppearAndClears(crate::npc_scripts::MINERVA);
        assert_eq!((action(OPQ_STATUE), first_state(OPQ_STATUE)), (minerva, 6));
        assert!(!stays(OPQ_STATUE));
        let Some(statue) = template(OPQ_STATUE) else {
            return;
        };
        assert!(statue.takes_items(6) && !statue.takes_items(0));
        let grass = crate::events::GRASS_OF_LIFE;
        assert_eq!(statue.item_dropped(6, grass, 1, (0, 0)), Some((0, 7)));
        assert!(statue.is_last(7));
    }

    #[test]
    fn the_treasure_rooms_boxes_spray_loot_and_bring_out_minerva() {
        let Action::SprayAndAppear {
            at_least,
            mesos,
            npc_id,
            at,
        } = action(2002014)
        else {
            panic!("{:?}", action(2002014));
        };
        assert_eq!((at_least, mesos), (15, (100, 400)));
        assert_eq!((npc_id, at), (crate::npc_scripts::MINERVA, (46, 840)));
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
    fn the_guild_quest_s_gates_and_spears_stay_set() {
        use crate::guild_quest::SPEAR;
        assert_eq!(action(9208005), Action::Hold);
        assert_eq!(action(9208006), Action::Hold);
        assert_eq!(action(SPEAR), Action::Spear);
        assert_eq!(action(9208004), Action::Drops);
        assert!(stays(9208005) && stays(9208006) && stays(SPEAR));
        // The waterway's doors take a stack of exactly so many Marks.
        for (door, marks) in [(9208010, 9), (9208011, 8), (9208012, 20)] {
            assert_eq!(action(door), Action::Hold);
            assert!(stays(door));
            let Some(door) = template(door) else { continue };
            assert!(door.item_dropped(0, 4001035, marks, (0, 0)).is_some());
            assert!(door.item_dropped(0, 4001035, marks - 1, (0, 0)).is_none());
            assert!(door.is_last(1) && door.time_out(1).is_none());
        }
        use crate::guild_quest::{GARGOYLE_ALTAR, JR_GARGOYLE, JR_GARGOYLES, JR_GARGOYLES_AT};
        assert_eq!(
            action(GARGOYLE_ALTAR),
            Action::BringOutAt {
                mob_id: JR_GARGOYLE,
                count: JR_GARGOYLES,
                at: JR_GARGOYLES_AT
            }
        );
        if let Some(altar) = template(GARGOYLE_ALTAR) {
            assert!(altar.item_dropped(0, 4001036, 1, (0, 0)).is_some());
        }
        // The grave's altar takes the shoes, pants, top and crown in turn,
        // and the last brings out the soul.
        use crate::guild_quest::{CLOTHES_ALTAR, SHAREN_SOUL};
        let Action::Appear { npc_id, .. } = action(CLOTHES_ALTAR) else {
            panic!("{:?}", action(CLOTHES_ALTAR));
        };
        assert_eq!(npc_id, SHAREN_SOUL);
        if let Some(altar) = template(CLOTHES_ALTAR) {
            let clothes = [4001032, 4001031, 4001033, 4001034];
            for (state, piece) in (0..).zip(clothes) {
                assert_eq!(
                    altar
                        .item_dropped(state, piece, 1, (0, 0))
                        .map(|(_, next)| next),
                    Some(state + 1)
                );
                let other = clothes[(usize::from(state) + 1) % 4];
                assert!(altar.item_dropped(state, other, 1, (0, 0)).is_none());
            }
            assert!(altar.is_last(4));
        }
        // The King's Corridor's door takes the earring and stays open.
        use crate::guild_quest::{KING_GATE_ITEM, KING_GATE_REACTOR};
        assert_eq!(action(KING_GATE_REACTOR), Action::KingGate);
        assert!(stays(KING_GATE_REACTOR));
        if let Some(gate) = template(KING_GATE_REACTOR) {
            assert!(gate.item_dropped(0, KING_GATE_ITEM, 1, (0, 0)).is_some());
            assert!(gate.item_dropped(0, 4001024, 1, (0, 0)).is_none());
            assert!(gate.is_last(1) && gate.time_out(1).is_none());
        }
        let (Some(stone), Some(metal), Some(spear)) =
            (template(9208006), template(9208005), template(SPEAR))
        else {
            return;
        };
        assert!(stone.item_dropped(0, 4001026, 1, (0, 0)).is_some());
        assert!(metal.item_dropped(0, 4001037, 1, (0, 0)).is_some());
        assert!(spear.item_dropped(0, 4001025, 1, (0, 0)).is_some());
        assert!(spear.item_dropped(0, 4001026, 1, (0, 0)).is_none());
        assert!(spear.is_last(1) && stone.is_last(1) && metal.is_last(1));
        // Nothing set goes back on its own.
        for set in [&spear, &stone, &metal] {
            assert_eq!(set.time_out(1), None);
        }
    }

    #[test]
    fn the_forests_pond_drops_and_its_spine_stays_and_clears() {
        assert_eq!(action(ELLIN_POND), Action::Drops);
        assert!(acts_staying(ELLIN_POND));
        assert_eq!(action(ELLIN_SPINE), Action::DropsAndClears);
        assert!(stays(ELLIN_SPINE) && acts_staying(ELLIN_SPINE));
        assert_eq!(action(ELLIN_STONE_BOX), Action::DropsAndClears);
        assert_eq!(action(3002002), Action::Drops);
        assert_eq!(action(ELLIN_FOUNTAIN), Action::ExpToAll(52_000));
        assert_eq!(action(MAGATIA_BEAKER), Action::Beaker);
        assert!(!stays(MAGATIA_BEAKER) && !acts_staying(MAGATIA_BEAKER));
        for door in MAGATIA_SIDE_DOOR..=MAGATIA_LAST_SIDE_DOOR {
            assert_eq!(action(door), Action::SideDoor);
            assert!(acts_staying(door) && shuffles(door));
        }
        assert!(MAGATIA_ROOM_EXITS.all(|exit| !shuffles(exit)));
        assert!(shuffles(2612001) && shuffles(2518000));
        for side in crate::events::MagatiaSide::ALL {
            let hub = side.stages()[3];
            assert!(shuffled(hub + 1) && shuffled(hub + 2));
            assert!(!shuffled(hub) && !shuffled(hub + 3));
        }
        assert!(!stays(ELLIN_FOUNTAIN) && !acts_staying(ELLIN_FOUNTAIN));
        assert!(!stays(ELLIN_STONE_BOX) && !acts_staying(ELLIN_STONE_BOX));
        assert!(matches!(
            action(ELLIN_ALTAR),
            Action::SpawnAt {
                mob_id: crate::events::ELLIN_GOLEM,
                ..
            }
        ));
        assert!(shuffled(crate::events::ELLIN_STAGES[4]));
        assert!(!shuffled(crate::events::ELLIN_STAGES[5]));
    }

    #[test]
    fn the_gardens_pots_grow_royals_now_and_then_and_always_after_the_seventh() {
        assert_eq!(action(2001000), Action::Pot);
        assert_eq!(action(2001001), Action::Pot);
        assert_eq!(action(2001016), Action::PapaPixie);
        assert_eq!(action(2002003), Action::DropsAndClears);
        assert!(!acts_staying(2002003));
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
            let records = (0..7).filter(|d| takes(MUSIC, "", SUNDAY_RECORD + d, day));
            assert_eq!(records.collect::<Vec<_>>(), [i32::from(day)]);
        }
        assert!(takes(2001002, "", 4001074, 3));
    }

    #[test]
    fn each_trial_grounds_seal_takes_only_its_rooms_key() {
        use crate::events::CRYSTAL_KEYS;
        assert_eq!(action(HTPQ_PASS_KEY), Action::PassKey);
        for room in 1..=4 {
            let name = format!("passKey{room}");
            let keys = CRYSTAL_KEYS
                .iter()
                .filter(|&&key| takes(HTPQ_PASS_KEY, &name, key, 0));
            assert_eq!(keys.collect::<Vec<_>>(), [&CRYSTAL_KEYS[room]]);
        }
        assert!(!takes(HTPQ_PASS_KEY, "", CRYSTAL_KEYS[1], 0));
        assert!(!takes(HTPQ_PASS_KEY, "passKey5", CRYSTAL_KEYS[1], 0));
        if let Ok(maps) = crate::get_game_data() {
            for room in 1..=4 {
                let field = maps.field(crate::events::HTPQ.stage_map(room)).unwrap();
                let seals: Vec<_> = field
                    .reactors
                    .iter()
                    .filter(|reactor| reactor.reactor_id == HTPQ_PASS_KEY)
                    .map(|reactor| reactor.name.clone())
                    .collect();
                assert_eq!(seals, [format!("passKey{room}")]);
            }
        }
        let Ok(data) = data() else { return };
        let seal = data.template(HTPQ_PASS_KEY).unwrap().unwrap();
        for (event, &key) in (0..).zip(&CRYSTAL_KEYS[1..]) {
            assert_eq!(seal.item_dropped(0, key, 1, (0, 0)), Some((event, 1)));
        }
        assert!(seal.is_last(1));
    }

    #[test]
    fn the_trial_grounds_lamp_turns_on_every_hit_and_never_breaks() {
        let Ok(data) = data() else { return };
        let lamp = data.template(HTPQ_LAMP).unwrap().unwrap();
        for (state, next) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            assert_eq!(lamp.swing(state, 2).map(|(_, to)| to), Some(next));
            assert!(!lamp.is_last(next));
        }
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
    fn the_mazes_boxes_bring_out_pairs_of_mobs_by_box_and_room() {
        use crate::events::{MAZE_GOAL, MAZE_ROOM};
        assert_eq!(action(8091000), Action::MazeBox);
        assert_eq!(action(8091004), Action::MazeBox);
        assert_eq!(action(MAZE_ROOM_BOX), Action::MazeBox);
        assert_eq!(maze_box(8091000, MAZE_ROOM), [(9400210, 2), (9400209, 2)]);
        assert_eq!(
            maze_box(8091003, MAZE_ROOM + 7),
            [(9400215, 2), (9400216, 2)]
        );
        for room in [0, 10, 14] {
            assert_eq!(
                maze_box(MAZE_ROOM_BOX, MAZE_ROOM + room),
                [(9400209, 6), (9400210, 9)]
            );
        }
        assert_eq!(
            maze_box(MAZE_ROOM_BOX, MAZE_ROOM + 5),
            [(9400217, 2), (9400218, 3)]
        );
        assert_eq!(
            maze_box(MAZE_ROOM_BOX, MAZE_ROOM + 1),
            [(9400213, 2), (9400214, 3)]
        );
        assert_eq!(
            maze_box(MAZE_ROOM_BOX, MAZE_ROOM + 13),
            [(9400209, 2), (9400210, 3)]
        );
        for room in MAZE_ROOM..MAZE_GOAL {
            assert!(shuffled(room));
            for id in [8091000, 8091001, 8091002, 8091003, 8091004, MAZE_ROOM_BOX] {
                for (mob, _) in maze_box(id, room) {
                    assert!((9400209..=9400218).contains(&mob), "{id} {room}");
                }
            }
        }
        assert!(!shuffled(MAZE_GOAL));
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

    #[test]
    fn the_dead_mines_boxes_send_back_or_hold_mimics_and_say_so() {
        let Action::Warp(back) = action(ZPQ_TRAP) else {
            panic!("{:?}", action(ZPQ_TRAP));
        };
        assert_eq!(action(ZPQ_MIMIC_BOX), Action::BringOut(ZPQ_MIMIC, 3));
        assert!(breaker_notice(ZPQ_TRAP).is_some() && breaker_notice(ZPQ_MIMIC_BOX).is_some());
        // Its chests drop their loot, unannounced.
        assert_eq!(action(2112012), Action::Drops);
        assert_eq!(breaker_notice(2112012), None);
        if crate::game_data::get().is_err() {
            return;
        }
        assert_eq!(back.to(), (crate::events::ZPQ_HUB, 0));
    }

    #[test]
    fn the_real_bart_sets_jonathans_record_and_the_way_out_finds_a_spawn() {
        assert_eq!(
            action(REAL_BART),
            Action::Record {
                quest: JONATHANS_COURT,
                record: BART_RECORD,
                value: BART_FOUND,
                notice: keys::JONATHAN_BART_FOUND,
            }
        );
        assert_eq!(action(1200000), Action::Drops);
        assert!(shuffled(BART_ROOM));
        assert!(lost_link(BART_ROOM, "ntq00") && !lost_link(BART_ROOM, "sp"));
        let Ok(maps) = crate::game_data::get() else {
            return;
        };
        let room = maps.field(BART_ROOM).unwrap();
        let out = room.portal_by_name("ntq00").unwrap();
        let to = maps.field(out.to_map.unwrap()).unwrap();
        assert!(to.resolve_spawn_portal(&out.to_name).is_none());
        assert!(to.resolve_spawn_portal("").is_some());
    }

    #[test]
    fn zakums_altar_brings_zakum_out() {
        assert_eq!(action(ZAKUM_ALTAR), Action::Zakum);
        assert_eq!(breaker_notice(ZAKUM_ALTAR), None);
    }

    #[test]
    fn horntails_stone_breaks_on_its_sixth_hit() {
        assert_eq!(action(HORNTAIL_STONE), Action::Horntail);
        assert_eq!(breaker_notice(HORNTAIL_STONE), None);
        let Ok(data) = data() else { return };
        let stone = data.template(HORNTAIL_STONE).unwrap().unwrap();
        assert_eq!(stone.states.len(), 7);
        assert!((0..6).all(|state| !stone.is_last(state)));
        assert!(stone.is_last(6));
    }

    #[test]
    fn the_pirate_ships_boxes_break_on_their_fourth_hit_into_mobs_or_items() {
        use crate::events::PIRATE_STAGES;
        assert_eq!(
            action(PIRATE_MOB_BOX),
            Action::BringOutLots([(9300109, 3), (9300110, 5)])
        );
        assert_eq!(
            action(PIRATE_DECK_BOX),
            Action::BringOutLots([(9300124, 6), (9300125, 6)])
        );
        assert_eq!(
            action(PIRATE_ITEM_BOX),
            Action::Spray {
                at_least: 15,
                mesos: (30, 60)
            }
        );
        assert_eq!(
            action(PIRATE_CHEST),
            Action::Spray {
                at_least: 15,
                mesos: (50, 100)
            }
        );
        // The invisible ones hold places, and are what the shuffle moves
        // the boxes among.
        assert_eq!(action(2518000), Action::Drops);
        for (stage, map) in PIRATE_STAGES.into_iter().enumerate() {
            assert_eq!(shuffled(map), [0, 2, 3].contains(&stage), "{map}");
        }
        // The seals take a key each, and stay, set.
        for seal in PIRATE_SEAL..=PIRATE_LAST_SEAL {
            assert_eq!(action(seal), Action::Seal { half: (160, 75) });
            assert!(acts_staying(seal) && stays(seal), "{seal}");
        }
        let Ok(data) = data() else { return };
        for box_id in [PIRATE_MOB_BOX, PIRATE_ITEM_BOX, PIRATE_DECK_BOX] {
            let reactor = data.template(box_id).unwrap().unwrap();
            assert!((0..4).all(|state| !reactor.is_last(state)), "{box_id}");
            assert!(reactor.is_last(4), "{box_id}");
        }
        for seal in PIRATE_SEAL..=PIRATE_LAST_SEAL {
            let reactor = data.template(seal).unwrap().unwrap();
            assert!(!reactor.is_last(0) && reactor.is_last(1), "{seal}");
        }
    }
}
