//! Party events played in a copy of their maps of their own (an instance):
//! who may start one, where it is played, and when it ends. Pure rules;
//! the runtime's event actor keeps the time and moves the players.

use crate::ClientId;
use game::{InventoryChange, Player};
use game_data::Area;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Kerning City's party quest, started at Lakelis.
    Kpq,
    /// Henesys's party quest, started at Tory.
    Hpq,
    /// Ludibrium's party quest, started at the Red Sign on Eos Tower's
    /// 101st floor.
    Lpq,
    /// Orbis's party quest in the Tower of Goddess, started at Wonky.
    Opq,
    /// Zakum's Dead Mine, the first of Adobis's trials, started at Adobis.
    Zpq,
    /// The expedition to Zakum's altar, gathered by Adobis (2030013) at the
    /// Entrance to Zakum (`crate::squads`).
    Zakum,
    /// The fight with Papulatus at the Origin of Clocktower, which a
    /// party's leader starts by its portal at Deep Inside the Clocktower.
    Papulatus,
}

/// An event's maps and terms.
#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    /// Where the party gathers and its leader starts the event.
    pub lobby: i32,
    /// The maps the party plays in its own copies of.
    pub maps: RangeInclusive<i32>,
    /// Where everyone starts.
    pub entry: i32,
    /// Where everyone goes when they leave or the event ends. Never one of
    /// the event's maps, even within `maps`.
    pub exit: i32,
    pub levels: RangeInclusive<u8>,
    /// How many may play, the leader included.
    pub team: RangeInclusive<usize>,
    pub minutes: u64,
    /// The EXP every member gets for clearing each stage, in order. Stage n
    /// is played on `entry + (n - 1) * stride`, unless `stage_maps` says.
    pub exp_per_stage: &'static [i32],
    /// How far apart the stage maps' ids are; the maps between them are
    /// rooms off the stages.
    pub stride: i32,
    /// The map each stage is played on, in order, for an event whose
    /// stages don't follow `stride`; empty for one whose do.
    pub stage_maps: &'static [i32],
    /// The stages the leader clears just by talking to their NPC.
    pub talk_clears: &'static [u8],
    /// The stages any member may clear: when a reactor on their map drops
    /// the stage's loot, whoever broke it (the Tower of Goddess's Salion
    /// stage, `net::reactors::Action::Salion`), or when their NPC finds the
    /// stage done, whoever asked (Eak in the music, head-count and lever rooms).
    pub member_clears: &'static [u8],
    /// Where those on a stage's map go when it is cleared: the stage and
    /// the portal on its map.
    pub clear_portals: &'static [(u8, &'static str)],
    /// The stage maps whose `next00` portal stays shut until their stage is
    /// cleared.
    pub gated: &'static [i32],
    /// The stages cleared by standing on their maps' areas (`on_areas`).
    pub puzzles: RangeInclusive<u8>,
    /// How many of a puzzle stage's areas the team must stand on, one
    /// member on each.
    pub areas_taken: u32,
    /// Items of use only in the event, taken from whoever leaves it.
    pub items: &'static [i32],
    /// The passes the leader hands Cloto to clear the last stage, which
    /// wins the event.
    pub passes: i32,
    /// The stages cleared by the leader handing their NPC some items.
    pub hand_ins: &'static [HandIn],
    /// Whether the stage maps have a `gate` object, shown once their stage
    /// is cleared (a map without one shows nothing).
    pub gates: bool,
    /// Where the whole team goes on winning, if anywhere.
    pub clear_map: Option<i32>,
    /// The bonus the team that won goes to: taken there by the leader, or
    /// at once (`Bonus::on_win`).
    pub bonus: Option<Bonus>,
    /// The item buffs, one drawn at random, that every member gets on
    /// arrival when the team has all five job lines among it (Orbis's
    /// "Blessing from Wonky the Fairy"); none for the other events.
    pub blessings: &'static [i32],
    /// Whether its team is a squad gathered at its NPC from anyone, not a
    /// party (`crate::squads`): it ends only when the last member leaves,
    /// whoever leads, and members on its roster may come back in while it
    /// runs (Cosmic `isExpeditionTeamLackingNow`).
    pub squad: bool,
    /// The reactors, by their map and name, that the event shuts (state
    /// 1) while it runs and opens (0) once it is won or over (Cosmic
    /// ZakumBattle's and PapulatusBattle's `updateGateState`).
    pub gate: &'static [(i32, &'static str)],
    /// Whether, like a squad's, it ends only when the last member leaves,
    /// whoever leads and whatever becomes of the party (Cosmic
    /// PapulatusBattle, whose `leftParty` and `changedLeader` do nothing).
    pub until_empty: bool,
}

/// A stage that `npc` clears once the leader hands it `count` of `item`
/// (Ludibrium's balloons, Cosmic `2040036`-`2040044`; Eak in two of the
/// Tower of Goddess's rooms).
#[derive(Debug, PartialEq, Eq)]
pub struct HandIn {
    pub stage: u8,
    pub npc: i32,
    pub item: i32,
    pub count: i16,
    /// What `npc` gives the leader for them, if anything.
    pub prize: Option<i32>,
    /// Whether the leader first hears what the stage asks, and only then
    /// may hand them in (Ludibrium's `statusStg`); Eak takes them at once.
    pub intro: bool,
}

/// Where a member stands with a stage's `HandIn` NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balloon {
    /// The stage is cleared; the way on is open.
    Cleared,
    /// Anyone but the leader, who is sent to fetch them.
    NotLeader,
    /// The leader's first talk: what the stage asks, and no hand-in yet.
    Intro,
    /// The leader again, who may hand the items in.
    Ready,
    /// The leader again on a puzzle stage, answering it: the team must
    /// stand as `combo` asks (`on_areas`).
    Answer { combo: u32 },
}

/// A won event's bonus: the team plays `map` for `minutes`, then whoever is
/// still there goes on to `exit`.
#[derive(Debug, PartialEq, Eq)]
pub struct Bonus {
    pub map: i32,
    pub exit: i32,
    pub minutes: u64,
    /// Whether the win itself takes the team there, rather than the leader
    /// later.
    pub on_win: bool,
}

impl Bonus {
    /// How long the bonus lasts, shortened like the event's time.
    pub fn duration(&self) -> Duration {
        scaled(Duration::from_secs(self.minutes * 60))
    }
}

/// Where a member stands with Aura on the Dead Mine's hub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aura {
    /// Before the win, when only the leader may hand her the Fire Ore.
    Playing { leader: bool },
    /// After it, with the documents or not, and the member's share of the
    /// prize taken or not.
    Won { documents: bool, taken: bool },
}

/// Kerning City's party quest (Cosmic `KerningPQ`, Server.NX
/// `FieldSet.img/Party1`: maps 103000800-805, 30 minutes). Stage EXP is
/// Cosmic's; `next00` leads on from 103000800-803, and 800-804 each have a
/// `gate` object (Map.nx). Stages 2 to 4 are Cloto's ropes, platforms and
/// barrels; stage 5's monsters drop the 10 passes that clear it (Cosmic
/// `9020001`, `clearPQ`). After the win Cloto gives each member a reward
/// and sends them to `KPQ_BONUS`, which has no time limit.
pub const KPQ: Event = Event {
    lobby: 103_000_000,
    maps: 103_000_800..=103_000_805,
    entry: 103_000_800,
    exit: 103_000_890,
    levels: 21..=30,
    team: 3..=4,
    minutes: 30,
    exp_per_stage: &[100, 200, 400, 800, 1500],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[103_000_800, 103_000_801, 103_000_802, 103_000_803],
    puzzles: 2..=4,
    areas_taken: 3,
    items: &[COUPON, PASS],
    passes: 10,
    hand_ins: &[],
    gates: true,
    clear_map: None,
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
};

/// Kerning City's bonus stage, 103000805, where the members of a team that
/// won go one by one from Cloto, and stay until they leave by Nella.
pub const KPQ_BONUS: i32 = 103_000_805;

/// What Cloto may give each member of a team that won Kerning City's party
/// quest: one of these items, and how many, all equally likely (Cosmic
/// `KerningPQ` `setEventRewards`).
pub const KPQ_REWARDS: [(i32, i16); 42] = [
    (2040505, 1),
    (2040514, 1),
    (2040502, 1),
    (2040002, 1),
    (2040602, 1),
    (2040402, 1),
    (2040802, 1),
    (1032009, 1),
    (1032004, 1),
    (1032005, 1),
    (1032006, 1),
    (1032007, 1),
    (1032010, 1),
    (1032002, 1),
    (1002026, 1),
    (1002089, 1),
    (1002090, 1),
    (2000003, 80),
    (2000001, 80),
    (2000002, 80),
    (2000006, 50),
    (2022003, 5),
    (2022000, 15),
    (2000004, 15),
    (4003000, 30),
    (4010000, 15),
    (4010001, 15),
    (4010002, 15),
    (4010003, 15),
    (4010004, 8),
    (4010005, 8),
    (4010006, 8),
    (4010007, 8),
    (4020000, 8),
    (4020001, 8),
    (4020002, 8),
    (4020003, 8),
    (4020004, 8),
    (4020005, 8),
    (4020006, 8),
    (4020007, 3),
    (4020008, 3),
];

/// Henesys's party quest on Primrose Hill (Cosmic `HenesysPQ`: maps
/// 910010000-400, of which 910010300 is the way out, 3 to 6 players from
/// level 10, 10 minutes). One stage, played on 910010000, cleared by
/// bringing Growlie 10 rice cakes; the seeds and cakes are the quest's own.
/// Winning takes the team to 910010100, from where Tommy may take it to
/// Pig Town (910010200) for 5 minutes, which ends at 910010400 (Cosmic
/// `1012113`, `scheduledTimeout`).
pub const HPQ: Event = Event {
    lobby: 100_000_200,
    maps: 910_010_000..=910_010_400,
    entry: 910_010_000,
    exit: 910_010_300,
    levels: 10..=u8::MAX,
    team: 3..=6,
    minutes: 10,
    exp_per_stage: &[1600],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[
        4001095, 4001096, 4001097, 4001098, 4001099, 4001100, RICE_CAKE,
    ],
    passes: 10,
    hand_ins: &[],
    gates: false,
    clear_map: Some(910_010_100),
    bonus: Some(Bonus {
        map: 910_010_200,
        exit: 910_010_400,
        minutes: 5,
        on_win: false,
    }),
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
};

/// Ludibrium's party quest in the Dimensional Crack (Cosmic `LudiPQ`: 5
/// to 6 players of levels 35 to 50, 45 minutes). Its stage maps are 100
/// apart, from 922010100 to the boss's 922010900, with rooms between; the
/// bonus is 922011000, where the win takes the team for a minute, the
/// reward 922011100, and 922010000 is the way out. Stage EXP is Cosmic's
/// (`expStages`, stage 6 none). Stage 8 is a puzzle: five members on the
/// right five of its nine crates. Stages 1 to 5, 7 and 8 shut their
/// `next00` until cleared (Map.nx, scripts `lpq0` to `lpq7`). The passes and the key are the quest's own, and so is the pass
/// stage 7's boxes take (4001156; the default 5A of
/// `v83ref/lanes/QUESTIONS.md`, 2026-10-22, LPQ).
pub const LPQ: Event = Event {
    lobby: 221_024_500,
    maps: 922_010_000..=922_011_100,
    entry: 922_010_100,
    exit: 922_010_000,
    levels: 35..=50,
    team: 5..=6,
    minutes: 45,
    exp_per_stage: &[210, 2520, 2940, 3360, 3770, 0, 4620, 5040, 5950],
    stride: 100,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[
        922_010_100,
        922_010_200,
        922_010_300,
        922_010_400,
        922_010_500,
        922_010_700,
        922_010_800,
    ],
    puzzles: 8..=8,
    areas_taken: 5,
    items: &[LPQ_PASS, LPQ_KEY, LPQ_BOX_PASS],
    passes: 0,
    hand_ins: &[
        lpq_hand_in(1, 2040036, LPQ_PASS, 25),
        lpq_hand_in(2, 2040037, LPQ_PASS, 15),
        lpq_hand_in(3, 2040038, LPQ_PASS, 32),
        lpq_hand_in(4, 2040039, LPQ_PASS, 6),
        lpq_hand_in(5, 2040040, LPQ_PASS, 24),
        lpq_hand_in(7, 2040042, LPQ_PASS, 3),
        lpq_hand_in(9, 2040044, LPQ_KEY, 1),
    ],
    gates: true,
    clear_map: None,
    bonus: Some(Bonus {
        map: 922_011_000,
        exit: 922_011_100,
        minutes: 1,
        on_win: true,
    }),
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
};

/// What Arturo may give each member of a team that won Ludibrium's party
/// quest: one of these items, and how many, all equally likely (Cosmic
/// `LudiPQ` `setEventRewards`, which lists one earring twice).
pub const LPQ_REWARDS: [(i32, i16); 77] = [
    (2040602, 1),
    (2040802, 1),
    (2040002, 1),
    (2040402, 1),
    (2040505, 1),
    (2040502, 1),
    (2040601, 1),
    (2044501, 1),
    (2044701, 1),
    (2044601, 1),
    (2041019, 1),
    (2041016, 1),
    (2041022, 1),
    (2041013, 1),
    (2041007, 1),
    (2043301, 1),
    (2040301, 1),
    (2040801, 1),
    (2040001, 1),
    (2040004, 1),
    (2040504, 1),
    (2040501, 1),
    (2040513, 1),
    (2043101, 1),
    (2044201, 1),
    (2044401, 1),
    (2040701, 1),
    (2044301, 1),
    (2043801, 1),
    (2040401, 1),
    (2043701, 1),
    (2040803, 1),
    (2000003, 85),
    (2000002, 85),
    (2000004, 10),
    (2000006, 60),
    (2000005, 2),
    (2022000, 20),
    (2001001, 15),
    (2001002, 15),
    (2022003, 20),
    (2001000, 15),
    (2020014, 10),
    (2020015, 5),
    (4003000, 35),
    (1102003, 1),
    (1102004, 1),
    (1102000, 1),
    (1102002, 1),
    (1102001, 1),
    (1102011, 1),
    (1102012, 1),
    (1102013, 1),
    (1102014, 1),
    (1032011, 1),
    (1032012, 1),
    (1032013, 1),
    (1032002, 1),
    (1032008, 1),
    (1032011, 1),
    (2070011, 1),
    (4010003, 10),
    (4010000, 10),
    (4010006, 6),
    (4010002, 10),
    (4010005, 10),
    (4010004, 10),
    (4010001, 10),
    (4020001, 10),
    (4020002, 10),
    (4020008, 4),
    (4020007, 4),
    (4020003, 10),
    (4020000, 10),
    (4020004, 10),
    (4020005, 10),
    (4020006, 10),
];

/// Ludibrium's Pass of Dimension, which most of its stages' monsters and
/// boxes drop; the key the last stage takes; and the other pass, which
/// stage 7's monsters drop and its reactors take.
pub const LPQ_PASS: i32 = 4001022;
pub const LPQ_KEY: i32 = 4001023;
pub const LPQ_BOX_PASS: i32 = 4001156;

/// One of Ludibrium's stages, handed in to its balloon (Cosmic's counts,
/// `2040036`-`2040044`). Stage 6's maze and stage 8's crates are cleared
/// otherwise.
const fn lpq_hand_in(stage: u8, npc: i32, item: i32, count: i16) -> HandIn {
    HandIn {
        stage,
        npc,
        item,
        count,
        prize: None,
        intro: true,
    }
}

/// One of the Tower of Goddess's rooms: Eak takes `count` of `item` from
/// the leader for a piece of Minerva's statue, `piece`.
const fn opq_hand_in(stage: u8, item: i32, count: i16, piece: i32) -> HandIn {
    HandIn {
        stage,
        npc: crate::npc_scripts::EAK,
        item,
        count,
        prize: Some(piece),
        intro: false,
    }
}

/// Orbis's party quest in the Tower of Goddess (Cosmic `OrbisPQ`: 5 to 6
/// players of levels 51 to 70; Server.NX `FieldSet.img/Party5`: maps
/// 920010000-920011300, 3600 seconds, the default 1A of
/// `v83ref/lanes/QUESTIONS.md`, 2026-10-22, Orbis PQ). The team starts on
/// the cloud stage, 920010000, and leaves by 920011200, from where Eak
/// sends players back to Orbis; after the win Minerva gives her prizes on
/// 920011300.
/// Its items are the statue's pieces and everything its stages take
/// (Cosmic `setExclusiveItems`).
///
/// Its stages (Cosmic's `statusStg0` to `statusStg8`) are not 100 apart:
/// 1 is the cloud stage, cleared by the leader talking to Eak once the team
/// has brought out his light, which takes everyone on it to `st00`; 2 to 7
/// are the six rooms off the tower's hub (920010100), each holding a piece
/// of the statue; 8 is the garden (920010800); and 9, the win, is Minerva's
/// statue on the hub, which the Grass of Life brings back to life. Each
/// but the garden gives 3500 EXP (Cosmic `giveEventPlayersExp`; the
/// statue's script means to give the win's but fails before it). On the
/// win the team stays on the hub, where Minerva sends each member on to
/// her prize room, 920011300, alone (`OPQ_PRIZE_ROOM`).
pub const OPQ: Event = Event {
    lobby: 200_080_101,
    maps: 920_010_000..=920_011_300,
    entry: 920_010_000,
    exit: 920_011_200,
    levels: 51..=70,
    team: 5..=6,
    minutes: 60,
    exp_per_stage: &[3500, 3500, 3500, 3500, 3500, 3500, 3500, 0, 3500],
    stride: 100,
    stage_maps: &[
        920_010_000,
        920_010_200,
        OPQ_SALION_ROOM,
        920_010_400,
        920_010_500,
        920_010_600,
        920_010_700,
        920_010_800,
        OPQ_HUB,
    ],
    talk_clears: &[1],
    member_clears: &[
        OPQ_SALION_STAGE,
        OPQ_MUSIC_STAGE,
        OPQ_HEAD_COUNT_STAGE,
        OPQ_LEVER_STAGE,
        OPQ_GARDEN_STAGE,
        OPQ_WIN_STAGE,
    ],
    clear_portals: &[(1, "st00")],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &OPQ_ITEMS,
    passes: 0,
    hand_ins: &[
        // The walkway's pixies' 30 (Cosmic `2013001`, `statusStg1`).
        opq_hand_in(2, 4_001_050, 30, 4_001_044),
        // The 40 from the lounge's rooms (`statusStg5`).
        opq_hand_in(6, 4_001_052, 40, 4_001_048),
    ],
    gates: false,
    clear_map: None,
    bonus: None,
    // Attack, defence, accuracy and avoidability, or speed and jump, for
    // 30 minutes (Cosmic `afterSetup`).
    blessings: &[2_022_090, 2_022_091, 2_022_092, 2_022_093],
    squad: false,
    gate: &[],
    until_empty: false,
};

/// The Tower of Goddess's hub, where Minerva's statue stands and its six
/// rooms open off.
pub const OPQ_HUB: i32 = 920_010_100;
/// The Tower of Goddess's storage room, where Cellions come out at the
/// Salion's spots in turn, and its stage.
pub const OPQ_SALION_ROOM: i32 = 920_010_300;
pub const OPQ_SALION_STAGE: u8 = 3;
/// The statue's second piece, dropped by the storage room's last spot, or
/// given by Eak once the room is bare.
pub const OPQ_SALION_PIECE: i32 = 4_001_045;
/// The Tower of Goddess's music room, where the day's record must play,
/// and its stage.
pub const OPQ_MUSIC_ROOM: i32 = 920_010_400;
pub const OPQ_MUSIC_STAGE: u8 = 4;
/// The Tower of Goddess's sealed room, where Eak counts heads on its three
/// platforms (`HeadCount`), and its stage.
pub const OPQ_HEAD_COUNT_ROOM: i32 = 920_010_500;
pub const OPQ_HEAD_COUNT_STAGE: u8 = 5;
/// The Tower of Goddess's lounge, whose climb pads (`party3_r4pt`) lead
/// up to its upper rooms.
pub const OPQ_LOUNGE: i32 = 920_010_600;
/// The Tower of Goddess's tall room, climbed pad by pad (`party3_r6pt`)
/// to the five levers Eak asks about (`LeverCheck`), and its stage.
pub const OPQ_LEVER_ROOM: i32 = 920_010_700;
pub const OPQ_LEVER_STAGE: u8 = 7;
/// The lever reactors' names there.
pub const OPQ_LEVERS: [&str; 5] = ["1", "2", "3", "4", "5"];
/// The Tower of Goddess's garden, where Papa Pixie keeps the Grass of
/// Life, and its stage, cleared as the Grass of Life's pot breaks.
pub const OPQ_GARDEN: i32 = 920_010_800;
pub const OPQ_GARDEN_STAGE: u8 = 8;
/// The Grass of Life (Cosmic's "Root of Life"), which the leader brings
/// from the garden for Minerva's statue.
pub const GRASS_OF_LIFE: i32 = 4_001_055;
/// The Orbis party quest's last stage, won as the Grass of Life is set at
/// the foot of Minerva's statue, by whoever sets it.
pub const OPQ_WIN_STAGE: u8 = 9;
/// Where Minerva gives each member of the team that won one of
/// `OPQ_REWARDS`, sent there by her on the hub (Cosmic `2013002`).
pub const OPQ_PRIZE_ROOM: i32 = 920_011_300;
/// The tower's hidden storage, open off the hub (`in06`) once the event is
/// won, whose guards the leader must clear before Eak lets them on.
pub const OPQ_STORAGE: i32 = 920_011_000;
/// Eak's treasure room beyond it, for the leader alone: its boxes spray
/// loot, and the first to break brings out Minerva (`OPQ_BONUS_MINERVA_AT`),
/// who sends the leader on to `OPQ_PRIZE_ROOM`.
pub const OPQ_BONUS: i32 = 920_011_100;
/// Where Minerva comes out there (Cosmic `2002014`'s `spawnNpc`), on the
/// floor at y 840.
pub const OPQ_BONUS_MINERVA_AT: (i16, i16) = (46, 840);

/// What Minerva may give each member of a team that won Orbis's party
/// quest: one of these items, and how many, all equally likely (Cosmic
/// `OrbisPQ` `setEventRewards`).
pub const OPQ_REWARDS: [(i32, i16); 94] = [
    (2040602, 1),
    (2040802, 1),
    (2040002, 1),
    (2040402, 1),
    (2040505, 1),
    (2040502, 1),
    (2040601, 1),
    (2044501, 1),
    (2044701, 1),
    (2044601, 1),
    (2041019, 1),
    (2041016, 1),
    (2041022, 1),
    (2041013, 1),
    (2041007, 1),
    (2043301, 1),
    (2040301, 1),
    (2040801, 1),
    (2040001, 1),
    (2040004, 1),
    (2040504, 1),
    (2040501, 1),
    (2040513, 1),
    (2043101, 1),
    (2044201, 1),
    (2044401, 1),
    (2040701, 1),
    (2044301, 1),
    (2043801, 1),
    (2040401, 1),
    (2043701, 1),
    (2040803, 1),
    (2000003, 100),
    (2000002, 100),
    (2000004, 15),
    (2000006, 80),
    (2000005, 5),
    (2022000, 25),
    (2001001, 20),
    (2001002, 20),
    (2022003, 25),
    (2001000, 20),
    (2020014, 15),
    (2020015, 10),
    (4003000, 45),
    (1102015, 1),
    (1102016, 1),
    (1102017, 1),
    (1102018, 1),
    (1102021, 1),
    (1102022, 1),
    (1102023, 1),
    (1102024, 1),
    (1102084, 1),
    (1102085, 1),
    (1102086, 1),
    (1032019, 1),
    (1032020, 1),
    (1032021, 1),
    (1032014, 1),
    (2070011, 1),
    (4010003, 15),
    (4010000, 15),
    (4010006, 10),
    (4010002, 15),
    (4010005, 15),
    (4010004, 15),
    (4010001, 15),
    (4020001, 15),
    (4020002, 15),
    (4020008, 10),
    (4020007, 10),
    (4020003, 15),
    (4020000, 15),
    (4020004, 15),
    (4020005, 15),
    (4020006, 15),
    (2210000, 5),
    (2210001, 5),
    (2210002, 5),
    (2070006, 1),
    (2070005, 1),
    (2070007, 1),
    (2070004, 1),
    (2061003, 2000),
    (2060003, 2000),
    (2060004, 2000),
    (2061004, 2000),
    (2100000, 1),
    (2100001, 1),
    (2100002, 1),
    (2100003, 1),
    (2100004, 1),
    (2100005, 1),
];
/// The hub's scars, where the statue's six pieces go.
pub const OPQ_SCARS: [&str; 6] = ["scar1", "scar2", "scar3", "scar4", "scar5", "scar6"];

/// What Eak makes of the hub once its scars hold every piece: the team
/// goes to the garden while it isn't cleared, then waits for the Grass of
/// Life on the statue, then for Minerva (Cosmic `2013001`, 920010100).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statue {
    /// A scar still lacks its piece.
    Gathering,
    Garden,
    GrassOfLife,
    Won,
}

/// The Orbis party quest's own items, 4001044 to 4001063.
const OPQ_ITEMS: [i32; 20] = {
    let mut items = [0; 20];
    let mut i = 0;
    while i < items.len() {
        items[i] = 4_001_044 + i as i32;
        i += 1;
    }
    items
};

/// What the Moon Bunny makes on Primrose Hill, and Growlie wants.
pub const RICE_CAKE: i32 = 4001101;

/// What Tory gives each member of a party that won Henesys's party quest,
/// once (Cosmic `setEventRewards`; the default in
/// `v83ref/lanes/QUESTIONS.md`, 2026-10-22, row 180).
pub const HPQ_REWARD: i32 = 4001158;

/// Zakum's Dead Mine (Cosmic `ZakumPQ`): a maze of mine shafts, 280010000
/// to 280011006, all hub, rooms and end rooms with no monsters of their
/// own. Its one stage is won when the leader hands Aura, on the hub, the
/// Fire Ore from the giant chest in one of the end rooms; with it 30 of
/// the mine's documents earn more EXP and a better prize
/// (`ZPQ_DOCUMENTS`). The Room of Tragedy, 280090000, where those who die
/// or come back to the game inside land, is outside it.
pub const ZPQ: Event = Event {
    lobby: 211_042_300,
    maps: 280_010_000..=280_011_006,
    entry: ZPQ_HUB,
    exit: 211_042_300,
    levels: 50..=u8::MAX,
    team: 1..=6,
    minutes: 30,
    exp_per_stage: &[12_000],
    stride: 10,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[ZPQ_DOCUMENT, ZPQ_KEY, FIRE_ORE],
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: None,
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
};

/// The Dead Mine's hub, where Aura stands and its way out is.
pub const ZPQ_HUB: i32 = 280_010_000;
/// The mine's documents, under its rocks, and the keys that open its giant
/// chest, seven thrown at it.
pub const ZPQ_DOCUMENT: i32 = 4_001_015;
pub const ZPQ_KEY: i32 = 4_001_016;
/// What the giant chest holds, for the leader to hand Aura.
pub const FIRE_ORE: i32 = 4_001_018;
/// How many documents Aura takes with the Fire Ore for the better prize.
pub const ZPQ_DOCUMENTS_WANTED: i16 = 30;
/// Not a stage: the bit of the event's cleared stages that says the
/// documents came with the Fire Ore (Cosmic's `gotDocuments`), set as the
/// event is won (`EventCommand::WinWithDocuments`).
pub const ZPQ_DOCUMENTS: u8 = 2;
/// The EXP every member gets for the win with the documents, instead of
/// the stage's.
pub const ZPQ_DOCUMENTS_EXP: i32 = 20_000;
/// What Aura gives each member once the event is won: a piece of the Fire
/// Ore, and with the documents also the mine's return scrolls.
pub const FIRE_ORE_PIECE: i32 = 4_031_061;
pub const DEAD_MINE_SCROLL: (i32, i16) = (2_030_007, 5);

/// The Door to Zakum's gate reactor (2118002) and its map, shut while an
/// expedition is under way.
pub const ZAKUM_GATE: (i32, &str) = (211_042_300, "gate");

/// The expedition to Zakum (Cosmic `ZakumBattle`): the squad's members
/// on the Entrance to Zakum when its leader starts it go to their own copy
/// of the altar, for two hours. Zakum's last body's death wins it (its one
/// stage, cleared by any member there); the gate at the Door to Zakum stays
/// shut until then, or until it ends.
pub const ZAKUM: Event = Event {
    lobby: crate::npc_scripts::ENTRANCE_TO_ZAKUM,
    maps: crate::mobs::ZAKUM_ALTAR_MAP..=crate::mobs::ZAKUM_ALTAR_MAP,
    entry: crate::mobs::ZAKUM_ALTAR_MAP,
    exit: crate::npc_scripts::ENTRANCE_TO_ZAKUM,
    levels: 50..=u8::MAX,
    team: 6..=30,
    minutes: 120,
    exp_per_stage: &[0],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[1],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[],
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: None,
    bonus: None,
    blessings: &[],
    squad: true,
    gate: &[ZAKUM_GATE],
    until_empty: false,
};

/// Deep Inside the Clocktower, the way to Papulatus, and the Origin of
/// Clocktower where he is fought.
pub const CLOCKTOWER_DEPTHS: i32 = 220_080_000;
pub const ORIGIN_OF_CLOCKTOWER: i32 = 220_080_001;

/// The fight with Papulatus (Cosmic `PapulatusBattle`, Server.NX
/// `FieldSet.img/Populatus`): one to six of a party, all on Deep Inside
/// the Clocktower with their leader, go to their own copy of the Origin of
/// Clocktower for 45 minutes. Its three `ludigate` reactors before the
/// portal stay shut until it ends, which is only when the last of them
/// leaves. Papulatus's death on the copy wins it.
pub const PAPULATUS: Event = Event {
    lobby: CLOCKTOWER_DEPTHS,
    maps: ORIGIN_OF_CLOCKTOWER..=ORIGIN_OF_CLOCKTOWER,
    entry: ORIGIN_OF_CLOCKTOWER,
    exit: CLOCKTOWER_DEPTHS,
    levels: 1..=u8::MAX,
    team: 1..=6,
    minutes: 45,
    exp_per_stage: &[0],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[1],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[],
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: None,
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[
        (CLOCKTOWER_DEPTHS, "ludigate1"),
        (CLOCKTOWER_DEPTHS, "ludigate2"),
        (CLOCKTOWER_DEPTHS, "ludigate3"),
    ],
    until_empty: true,
};

impl EventKind {
    pub const ALL: [EventKind; 7] = [
        EventKind::Kpq,
        EventKind::Hpq,
        EventKind::Lpq,
        EventKind::Opq,
        EventKind::Zpq,
        EventKind::Zakum,
        EventKind::Papulatus,
    ];

    pub fn event(self) -> &'static Event {
        match self {
            EventKind::Kpq => &KPQ,
            EventKind::Hpq => &HPQ,
            EventKind::Lpq => &LPQ,
            EventKind::Opq => &OPQ,
            EventKind::Zpq => &ZPQ,
            EventKind::Zakum => &ZAKUM,
            EventKind::Papulatus => &PAPULATUS,
        }
    }
}

/// Whether `map_id` is one of any event's maps (never an exit), which
/// only a member of a running event may be on.
pub fn event_map(map_id: i32) -> bool {
    EventKind::ALL.iter().any(|kind| kind.event().holds(map_id))
}

/// A party member who might come along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub id: ClientId,
    pub level: u8,
    pub map_id: i32,
}

/// Whether a player's party may start an event, as its NPC tells them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    NoParty,
    NotLeader,
    /// Too few or too many members in the lobby at the right levels, or the
    /// leader isn't one of them.
    NotEligible,
    /// Another party is playing it on this channel.
    Busy,
    Ready,
}

impl Event {
    /// How long the event lasts: `minutes`, shortened by
    /// `RUSTMS_EVENT_SCALE` (a whole number, default 1) for testing.
    pub fn duration(&self) -> Duration {
        scaled(Duration::from_secs(self.minutes * 60))
    }

    /// How many must go for the event to start: the start of `team`, or
    /// for a squad `RUSTMS_EXPEDITION_MIN` if set, for testing.
    pub fn least(&self) -> usize {
        let fewer = || {
            std::env::var("RUSTMS_EXPEDITION_MIN")
                .ok()?
                .parse()
                .ok()
                .filter(|least| *least >= 1)
        };
        self.squad
            .then(fewer)
            .flatten()
            .unwrap_or(*self.team.start())
    }

    /// Whether only the last member's leaving ends it: a squad's, or one
    /// that says so (`until_empty`).
    pub fn lasts_until_empty(&self) -> bool {
        self.squad || self.until_empty
    }

    /// Whether `map_id` is one of the event's maps; its exit never is.
    pub fn holds(&self, map_id: i32) -> bool {
        self.maps.contains(&map_id) && map_id != self.exit
    }

    /// The map stage `stage` (from 1) is played on.
    pub fn stage_map(&self, stage: u8) -> i32 {
        if !self.stage_maps.is_empty() {
            let index = usize::from(stage).saturating_sub(1);
            return self.stage_maps.get(index).copied().unwrap_or(self.entry);
        }
        self.entry + (i32::from(stage) - 1) * self.stride
    }

    /// The stage played on `map_id`, if any; none is played in its rooms.
    pub fn stage_on(&self, map_id: i32) -> Option<u8> {
        if !self.stage_maps.is_empty() {
            let index = self.stage_maps.iter().position(|&map| map == map_id)?;
            return u8::try_from(index + 1).ok();
        }
        let offset = map_id.checked_sub(self.entry)?;
        if offset.checked_rem(self.stride)? != 0 {
            return None;
        }
        let stage = u8::try_from(offset / self.stride + 1).ok()?;
        (1..=self.exp_per_stage.len())
            .contains(&usize::from(stage))
            .then_some(stage)
    }

    /// The portal on `stage`'s map that those on it go to when it is
    /// cleared, if any.
    pub fn clear_portal(&self, stage: u8) -> Option<&'static str> {
        self.clear_portals
            .iter()
            .find(|(cleared, _)| *cleared == stage)
            .map(|(_, portal)| *portal)
    }

    /// The hand-in that clears `stage`, if it is cleared that way.
    pub fn hand_in(&self, stage: u8) -> Option<&HandIn> {
        self.hand_ins.iter().find(|hand_in| hand_in.stage == stage)
    }

    /// The hand-in `npc` takes on `map_id`, if any.
    pub fn hand_in_on(&self, map_id: i32, npc: i32) -> Option<&HandIn> {
        self.hand_in(self.stage_on(map_id)?)
            .filter(|hand_in| hand_in.npc == npc)
    }

    /// The stage that must be cleared before `portal` on `map_id` lets
    /// anyone through.
    pub fn gate_stage(&self, map_id: i32, portal: &str) -> Option<u8> {
        if portal != "next00" || !self.gated.contains(&map_id) {
            return None;
        }
        self.stage_on(map_id)
    }

    /// Take the event's own items from `player`, who is leaving it (Cosmic
    /// `setExclusiveItems`).
    pub fn take_items(&self, player: &mut Player) -> Vec<InventoryChange> {
        let mut changes = Vec::new();
        for &item_id in self.items {
            let held = i16::try_from(player.inventory.count(item_id)).unwrap_or(i16::MAX);
            if held > 0 {
                changes.extend(
                    player
                        .inventory_mut()
                        .remove(item_id, held)
                        .unwrap_or_default(),
                );
            }
        }
        changes
    }

    /// The stage whose clear wins the event.
    pub fn last_stage(&self) -> u8 {
        u8::try_from(self.exp_per_stage.len()).unwrap_or(u8::MAX)
    }

    /// `who` talks to Cloto on the last stage, `cleared` or not: the leader
    /// hands her the passes, which only the leader may do.
    pub fn last_stage_talk(&self, who: ClientId, team: &Team, cleared: bool) -> Cloto {
        if cleared {
            Cloto::Won
        } else if who == team.leader() {
            Cloto::Leader {
                stage: self.last_stage(),
                needed: self.passes,
            }
        } else {
            Cloto::Gather
        }
    }

    /// The EXP for clearing `stage`.
    pub fn stage_exp(&self, stage: u8) -> i32 {
        usize::from(stage)
            .checked_sub(1)
            .and_then(|index| self.exp_per_stage.get(index))
            .copied()
            .unwrap_or(0)
    }

    /// Those of the leader's party online on the channel who come along:
    /// the members in the lobby at the right levels (Cosmic
    /// `getEligibleParty`). `None` unless the leader is one of them and
    /// they are enough, and not too many.
    pub fn team(&self, leader: ClientId, candidates: &[Candidate]) -> Option<Vec<ClientId>> {
        let team: Vec<_> = candidates
            .iter()
            .filter(|c| c.map_id == self.lobby && self.levels.contains(&c.level))
            .map(|c| c.id)
            .collect();
        (team.contains(&leader) && self.team.contains(&team.len())).then_some(team)
    }

    /// The blessing a team of these `jobs` gets, if any: one of
    /// `blessings`, at random, when they cover the warrior, magician,
    /// bowman, thief and pirate lines, whatever the advancement (Cosmic
    /// `isTeamAllJobs`, whose `getJobNiche` is the job's hundreds digit,
    /// so Cygnus Knights count for their line too).
    pub fn blessing(&self, jobs: &[i16], rng: &mut impl rand::Rng) -> Option<i32> {
        let lines = jobs
            .iter()
            .fold(0u16, |lines, &job| lines | 1 << (job / 100 % 10));
        if self.blessings.is_empty() || lines & ALL_JOB_LINES != ALL_JOB_LINES {
            return None;
        }
        Some(self.blessings[rng.gen_range(0, self.blessings.len())])
    }
}

/// The job lines' bits (`1 << line`), warrior 1 to pirate 5, beginners
/// (0) left out.
const ALL_JOB_LINES: u16 = 0b11_1110;

/// How often a friendly mob makes what it drops: its `dropItemPeriod`, in
/// tens of seconds, and a third of that for the Moon Bunny (Cosmic's, as
/// a whole period of 60 s would leave no time for 10 rice cakes; the
/// default in `v83ref/lanes/QUESTIONS.md`, 2026-10-22, row 180), shortened
/// like the events' time (`scaled`). None for a mob that makes
/// nothing.
pub fn friendly_drop_period(mob_id: i32, period: i32) -> Option<Duration> {
    let period = Duration::from_secs(u64::try_from(period).ok().filter(|&p| p > 0)? * 10);
    let period = if mob_id == crate::reactors::MOON_BUNNY {
        period / 3
    } else {
        period
    };
    Some(scaled(period))
}

/// An event's `time`, shortened by `RUSTMS_EVENT_SCALE` for testing.
pub fn scaled(time: Duration) -> Duration {
    time / scale()
}

fn scale() -> u32 {
    static SCALE: OnceLock<u32> = OnceLock::new();
    *SCALE.get_or_init(|| {
        std::env::var("RUSTMS_EVENT_SCALE")
            .ok()
            .and_then(|scale| scale.parse().ok())
            .filter(|scale| *scale >= 1)
            .unwrap_or(1)
    })
}

/// Who is still playing. The event ends when its leader leaves or too few
/// remain, and once won, when the last of them leaves (Cosmic
/// `isEventTeamLackingNow`). A squad's, and one that lasts until empty,
/// ends only when the last leaves (`isExpeditionTeamLackingNow`), and a
/// squad's members may come back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    leader: ClientId,
    members: Vec<ClientId>,
    least: usize,
    won: bool,
    /// Whether only the last member's leaving ends it.
    until_empty: bool,
    /// A squad's roster, who alone may come (back) in.
    roster: Option<HashSet<ClientId>>,
}

impl Team {
    pub fn new(event: &Event, leader: ClientId, members: Vec<ClientId>) -> Self {
        Self {
            leader,
            roster: event.squad.then(|| members.iter().copied().collect()),
            members,
            least: *event.team.start(),
            won: false,
            until_empty: event.lasts_until_empty(),
        }
    }

    /// Put those who stayed behind at a squad's start on its roster too.
    pub fn enlist(&mut self, roster: &[ClientId]) {
        if let Some(listed) = self.roster.as_mut() {
            listed.extend(roster);
        }
    }

    /// `member` of a squad's roster comes (back) in. Returns whether they
    /// may: not once it is won, nor if they aren't on it or are in.
    pub fn rejoin(&mut self, member: ClientId) -> bool {
        let listed = self
            .roster
            .as_ref()
            .is_some_and(|roster| roster.contains(&member));
        if !listed || self.won || self.has(member) {
            return false;
        }
        self.members.push(member);
        true
    }

    /// The team won the event: from now on only the last member's leaving
    /// ends it.
    pub fn win(&mut self) {
        self.won = true;
    }

    pub fn leader(&self) -> ClientId {
        self.leader
    }

    pub fn members(&self) -> &[ClientId] {
        &self.members
    }

    pub fn has(&self, member: ClientId) -> bool {
        self.members.contains(&member)
    }

    /// `member` left the event's maps, the party, or the game. Returns
    /// whether that ends the event.
    pub fn leave(&mut self, member: ClientId) -> bool {
        let Some(index) = self.members.iter().position(|&m| m == member) else {
            return false;
        };
        self.members.remove(index);
        if self.won || self.until_empty {
            return self.members.is_empty();
        }
        member == self.leader || self.members.len() < self.least
    }

    /// The leader handed the party to `to`, who leads the team from now
    /// on. Returns whether that ends the event: it does if `to` is not in
    /// its maps, unless it is won (Cosmic `changedLeader`). A squad isn't
    /// a party, so a party's hand-over changes nothing for it, and one that
    /// lasts until empty only follows the new leader.
    pub fn hand_over(&mut self, to: ClientId) -> bool {
        if self.roster.is_some() {
            return false;
        }
        self.leader = to;
        !self.won && !self.until_empty && !self.has(to)
    }
}

/// What a member's script tells their event, once its items have changed
/// hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventCommand {
    /// Cloto gave the member a pass.
    PassGiven,
    /// The leader handed Cloto the passes for stage `n`.
    ClearStage(u8),
    /// The party lost what it was to protect (the Moon Bunny): the event
    /// ends for everyone, unless it is won.
    Fail,
    /// The leader takes the team that won to the event's bonus.
    StartBonus,
    /// The leader handed the party to this member (Cosmic `changedLeader`).
    HandOver(ClientId),
    /// The leader takes the whole team to one of the event's maps, its
    /// clock still running (Cosmic `warpEventTeam`).
    MoveTeam(i32),
    /// The leader handed Aura the Fire Ore and the documents: the Dead
    /// Mine is won, with `ZPQ_DOCUMENTS` set first.
    WinWithDocuments,
    /// The member took their share of the won event's prize from Aura,
    /// once each (Cosmic's `gridInsert`): only their own ticket keeps it,
    /// so it goes when they leave.
    PrizeTaken,
}

/// The coupons Kerning City's party quest's first stage has its members
/// collect, and the pass Cloto gives for the right number.
pub const COUPON: i32 = 4001007;
pub const PASS: i32 = 4001008;

/// The answers to Cloto's stage 1 questions, in order (Cosmic `9020001`):
/// each is a job advancement's requirement, the level a warrior needs, the
/// STR a warrior needs, the INT a magician needs, the DEX a bowman needs,
/// the DEX a thief needs, the level of the second advancement, and the
/// level a magician needs.
pub const ANSWERS: [i32; 7] = [10, 35, 20, 25, 25, 30, 8];

/// Where a player stands with Cloto on a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cloto {
    /// The stage is cleared; the way on is open.
    Cleared,
    /// The last stage is cleared: the event is won.
    Won,
    /// A member other than the leader on a puzzle stage, which is the
    /// leader's to answer.
    Follow,
    /// A member other than the leader on the last stage, who gathers its
    /// passes for the leader to hand in.
    Gather,
    /// The leader, on first talking to her on puzzle stage `stage`: she sets
    /// the puzzle.
    Intro { stage: u8 },
    /// The leader, answering puzzle stage `stage`: the team must stand as
    /// `combo` asks (`on_areas`).
    Puzzle { stage: u8, combo: u32 },
    /// The leader, who clears `stage` by handing in `needed` passes: on
    /// stage 1 one from each of the others, on the last stage the event's
    /// `passes`.
    Leader { stage: u8, needed: i32 },
    /// Another member, asked `question` (an index into `ANSWERS`), told it
    /// for the first time if `new`.
    Member {
        question: usize,
        new: bool,
        passed: bool,
    },
}

/// Stage 1 of Kerning City's party quest: each member other than the
/// leader is asked a question, and earns a pass by holding the answer's
/// number of coupons; the leader clears the stage with everyone's pass.
#[derive(Debug, Default)]
pub struct Stage1 {
    questions: HashMap<ClientId, usize>,
    passed: HashSet<ClientId>,
    cleared: bool,
}

impl Stage1 {
    /// `who` talks to Cloto. A member is asked a question the first time,
    /// `draw`n then, and the same one after that.
    pub fn talk(&mut self, who: ClientId, team: &Team, draw: impl FnOnce() -> usize) -> Cloto {
        if self.cleared {
            return Cloto::Cleared;
        }
        if who == team.leader() {
            let needed = team.members().len().saturating_sub(1);
            return Cloto::Leader {
                stage: 1,
                needed: i32::try_from(needed).unwrap_or(i32::MAX),
            };
        }
        let new = !self.questions.contains_key(&who);
        let question = *self.questions.entry(who).or_insert_with(draw);
        Cloto::Member {
            question,
            new,
            passed: self.passed.contains(&who),
        }
    }

    /// Cloto gave `who` their pass.
    pub fn pass_given(&mut self, who: ClientId) {
        self.passed.insert(who);
    }

    /// The leader handed in the passes. Returns whether that cleared the
    /// stage, which it does once.
    pub fn clear(&mut self) -> bool {
        !std::mem::replace(&mut self.cleared, true)
    }
}

/// A question for a member who hasn't been asked one: random, or the one
/// `RUSTMS_KPQ_QUESTION` (1 to 7) names for testing.
pub fn draw_question() -> usize {
    static PINNED: OnceLock<Option<usize>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        std::env::var("RUSTMS_KPQ_QUESTION")
            .ok()
            .and_then(|question| question.parse::<usize>().ok())
            .filter(|question| (1..=ANSWERS.len()).contains(question))
            .map(|question| question - 1)
    });
    pinned.unwrap_or_else(|| rand::random::<usize>() % ANSWERS.len())
}

/// Stages 2 to 4 of Kerning City's party quest: the leader's team stands
/// on three of the map's areas, the right three, which are drawn when the
/// leader first talks to Cloto there (Cosmic `9020001`). Only the leader
/// may answer, as often as they like.
#[derive(Debug, Default)]
pub struct Puzzles {
    combos: HashMap<u8, u32>,
}

impl Puzzles {
    /// `who` talks to Cloto on puzzle stage `stage`, cleared or not. The
    /// leader's first talk `draw`s the stage's combo, if it can be drawn.
    pub fn talk(
        &mut self,
        stage: u8,
        who: ClientId,
        team: &Team,
        cleared: bool,
        draw: impl FnOnce() -> Option<u32>,
    ) -> Cloto {
        if cleared {
            return Cloto::Cleared;
        }
        if who != team.leader() {
            return Cloto::Follow;
        }
        if let Some(&combo) = self.combos.get(&stage) {
            return Cloto::Puzzle { stage, combo };
        }
        if let Some(combo) = draw() {
            self.combos.insert(stage, combo);
        }
        Cloto::Intro { stage }
    }

    /// Whether the puzzle of `stage` has been set, so may be answered.
    pub fn set(&self, stage: u8) -> bool {
        self.combos.contains_key(&stage)
    }
}

/// A combo of `taken` of a puzzle stage's `areas` areas: random, or the
/// one `RUSTMS_KPQ_COMBO` (from 0, in `combos`' order) names for testing.
/// `None` for a map without enough areas.
pub fn draw_combo(areas: usize, taken: u32) -> Option<u32> {
    static PINNED: OnceLock<Option<usize>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        std::env::var("RUSTMS_KPQ_COMBO")
            .ok()
            .and_then(|combo| combo.parse::<usize>().ok())
    });
    let count = combos(areas, taken).count();
    if count == 0 {
        return None;
    }
    let index = pinned
        .filter(|index| *index < count)
        .unwrap_or_else(|| rand::random::<usize>() % count);
    combos(areas, taken).nth(index)
}

/// Each way of choosing `taken` of `areas` areas, as a bit per area (bit 0
/// for the first), in a fixed order: on Kerning City's party quest three
/// of 4 for stage 2's ropes, of 5 for stage 3's platforms and of 6 for
/// stage 4's barrels (Cosmic `9020001`); on Ludibrium's five of stage 8's
/// 9 crates (`2040043`).
pub fn combos(areas: usize, taken: u32) -> impl Iterator<Item = u32> {
    let all = 1u32.checked_shl(areas as u32).unwrap_or(0);
    (0..all).filter(move |combo| combo.count_ones() == taken)
}

/// How many of the three members on the sealed room's platforms Eak wants
/// on each (Cosmic `2013001`, `stage4_0`-`2`): the first two drawn as
/// `round(random * left)` of the three, the last platform the rest. For
/// testing, `RUSTMS_OPQ_SPLIT` pins it as three digits that add up to 3
/// (`"210"`).
pub fn draw_split(rng: &mut impl rand::Rng) -> [u8; 3] {
    static PINNED: OnceLock<Option<[u8; 3]>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        pinned_digits("RUSTMS_OPQ_SPLIT", 0..=3).filter(|split| split.iter().sum::<u8>() == 3)
    });
    pinned.unwrap_or_else(|| {
        let mut left = 3;
        let mut split = [0; 3];
        for heads in &mut split[..2] {
            *heads = (rng.gen::<f64>() * f64::from(left)).round() as u8;
            left -= *heads;
        }
        split[2] = left;
        split
    })
}

/// How the team stands on the sealed room's platforms for Eak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadCount {
    /// Not three of them on the platforms in all.
    Few,
    /// Three, but not as he wants: `right` platforms have their number.
    Wrong {
        right: usize,
    },
    Right,
}

/// How `positions` stand on the first three of `areas` for `split`. A
/// position counts for the first area that holds it.
pub fn head_count(split: [u8; 3], areas: &[Area], positions: &[(i32, i32)]) -> HeadCount {
    let platforms = &areas[..areas.len().min(3)];
    let mut heads = [0u8; 3];
    for &(x, y) in positions {
        if let Some(area) = platforms.iter().position(|area| area.contains(x, y)) {
            heads[area] = heads[area].saturating_add(1);
        }
    }
    let right = heads.iter().zip(split).filter(|(h, s)| **h == *s).count();
    if heads.iter().map(|&h| usize::from(h)).sum::<usize>() != 3 {
        HeadCount::Few
    } else if right == 3 {
        HeadCount::Right
    } else {
        HeadCount::Wrong { right }
    }
}

/// The digits of the environment variable `var`, if it holds `N` of them,
/// each in `range`: a test's pin on a draw.
fn pinned_digits<const N: usize>(var: &str, range: RangeInclusive<u8>) -> Option<[u8; N]> {
    let pin = std::env::var(var).ok()?;
    let digits: Vec<u8> = pin.bytes().map(|b| b.wrapping_sub(b'0')).collect();
    let digits: [u8; N] = digits.try_into().ok()?;
    digits.iter().all(|d| range.contains(d)).then_some(digits)
}

/// The right pad of each row of a climb, drawn the first time anyone
/// steps on one (Cosmic `party3_r4pt`, `party3_r6pt`): on the lounge two
/// rows of three, on the tall room sixteen rows of four. For testing,
/// `RUSTMS_OPQ_LOUNGE` (`"23"`) and `RUSTMS_OPQ_TOWER` (16 digits) pin them.
pub fn draw_climb(map_id: i32, rng: &mut impl rand::Rng) -> Vec<u8> {
    static LOUNGE: OnceLock<Option<[u8; 2]>> = OnceLock::new();
    static TOWER: OnceLock<Option<[u8; 16]>> = OnceLock::new();
    let (rows, pads, pinned) = if map_id == OPQ_LOUNGE {
        let pinned = LOUNGE.get_or_init(|| pinned_digits("RUSTMS_OPQ_LOUNGE", 1..=3));
        (2, 3, pinned.map(|pin| pin.to_vec()))
    } else {
        let pinned = TOWER.get_or_init(|| pinned_digits("RUSTMS_OPQ_TOWER", 1..=4));
        (16, 4, pinned.map(|pin| pin.to_vec()))
    };
    pinned.unwrap_or_else(|| (0..rows).map(|_| rng.gen_range(1, pads + 1)).collect())
}

/// The portal a climb pad, `name` with id `portal_id`, sends a player to
/// on its own map given the right pads `combo`; `None` if it isn't one.
///
/// On the lounge (`r4rp<row><pad>`) the right pad of the first row leads
/// to the same pad a row up, three ids on, and that of the second to
/// `success` (1); a wrong one to `fail` (2), below the first row.
///
/// On the tall room (`rp<row, two digits><pad>`) the right pad leads to
/// the same pad a row up, four ids on, or after every fourth row to that
/// section's landing, `pt00`-`pt03` (1-4); a wrong one back to the start
/// of its section, the landing below it or, in the first, `r6fail` (5).
/// Cosmic's script means that but divides as JavaScript does, so from
/// rows 5-8, 10-12 and 14-16 its portal is a fraction or the very bottom.
pub fn climb(map_id: i32, combo: &[u8], portal_id: u8, name: &str) -> Option<u8> {
    let digits = |s: &str| s.parse::<usize>().ok();
    if map_id == OPQ_LOUNGE {
        let rest = name.strip_prefix("r4rp").filter(|rest| rest.len() == 2)?;
        let (row, pad) = (digits(&rest[..1])?, digits(&rest[1..])?);
        let right = *combo.get(row.checked_sub(1)?)?;
        return Some(match (usize::from(right) == pad, row) {
            (true, 1) => portal_id + 3,
            (true, _) => 1,
            (false, _) => 2,
        });
    }
    let rest = name.strip_prefix("rp").filter(|rest| rest.len() == 3)?;
    let (row, pad) = (digits(&rest[..2])?, digits(&rest[2..])?);
    let right = *combo.get(row.checked_sub(1)?)?;
    let section = (row - 1) / 4;
    Some(if usize::from(right) != pad {
        if section == 0 {
            5
        } else {
            section as u8
        }
    } else if row % 4 != 0 {
        portal_id + 4
    } else {
        (row / 4) as u8
    })
}

/// The two of the tall room's five levers Eak wants up, drawn the first
/// time he is asked (Cosmic `2013001`, `stage6_c`). For testing,
/// `RUSTMS_OPQ_LEVERS` pins them as their two names (`"14"`).
pub fn draw_levers(rng: &mut impl rand::Rng) -> [bool; 5] {
    static PINNED: OnceLock<Option<[u8; 2]>> = OnceLock::new();
    let pinned = PINNED
        .get_or_init(|| pinned_digits("RUSTMS_OPQ_LEVERS", 1..=5))
        .filter(|[a, b]| a != b)
        .map(|[a, b]| [usize::from(a) - 1, usize::from(b) - 1]);
    let [first, second] = pinned.unwrap_or_else(|| {
        let first = rng.gen_range(0, 5);
        let second = (first + rng.gen_range(1, 5)) % 5;
        [first, second]
    });
    let mut pair = [false; 5];
    pair[first] = true;
    pair[second] = true;
    pair
}

/// What Eak makes of the tall room's levers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeverCheck {
    /// Not two of them up.
    Few,
    /// Two, but not his pair: `one_right` if one of them is in it.
    Wrong {
        one_right: bool,
    },
    Right,
}

/// How the levers that are `up` stand against Eak's `pair`.
pub fn lever_check(pair: [bool; 5], up: [bool; 5]) -> LeverCheck {
    if up.iter().filter(|&&up| up).count() != 2 {
        LeverCheck::Few
    } else if up == pair {
        LeverCheck::Right
    } else {
        let one_right = up.iter().zip(pair).any(|(&up, pair)| up && pair);
        LeverCheck::Wrong { one_right }
    }
}

/// How many of `positions` stand on any of `areas`.
pub fn standing_on_areas(areas: &[Area], positions: &[(i32, i32)]) -> usize {
    let on_any = |&&(x, y): &&(i32, i32)| areas.iter().any(|area| area.contains(x, y));
    positions.iter().filter(on_any).count()
}

/// How the team stands on a puzzle stage whose answer is checked only
/// with `taken` on its areas (Ludibrium's stage 8, Cosmic `2040043`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Not `taken` of them on the areas.
    Few,
    Wrong,
    Right,
}

/// How `positions` stand on `areas` for `combo`, of `taken` areas.
pub fn standing(combo: u32, taken: u32, areas: &[Area], positions: &[(i32, i32)]) -> Standing {
    if standing_on_areas(areas, positions) != taken as usize {
        Standing::Few
    } else if on_areas(combo, areas, positions) {
        Standing::Right
    } else {
        Standing::Wrong
    }
}

/// Whether the team stands as `combo` asks: one of `positions` on each of
/// its areas, and none on the others. A position counts for the first of
/// `areas` that holds it; one on none of them doesn't count.
pub fn on_areas(combo: u32, areas: &[Area], positions: &[(i32, i32)]) -> bool {
    let mut standing = vec![0; areas.len()];
    for &(x, y) in positions {
        if let Some(area) = areas.iter().position(|area| area.contains(x, y)) {
            standing[area] += 1;
        }
    }
    standing
        .iter()
        .enumerate()
        .all(|(area, &count)| count == combo.checked_shr(area as u32).unwrap_or(0) & 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(id: ClientId, level: u8, map_id: i32) -> Candidate {
        Candidate { id, level, map_id }
    }

    #[test]
    fn eak_counts_three_heads_on_the_sealed_rooms_platforms() {
        let area = |x1, x2| Area {
            x1,
            x2,
            y1: 0,
            y2: 10,
        };
        let platforms = [area(0, 10), area(20, 30), area(40, 50)];
        let on = |platform: i32| (platform * 20 + 5, 5);
        let count = |positions: &[(i32, i32)]| head_count([2, 1, 0], &platforms, positions);
        assert_eq!(count(&[on(0), on(0), on(1)]), HeadCount::Right);
        // One off every platform doesn't count.
        assert_eq!(count(&[on(0), on(0), on(1), (100, 5)]), HeadCount::Right);
        assert_eq!(count(&[on(0), on(1)]), HeadCount::Few);
        assert_eq!(count(&[on(0), on(0), on(1), on(2)]), HeadCount::Few);
        assert_eq!(count(&[on(0), on(1), on(2)]), HeadCount::Wrong { right: 1 });
        assert_eq!(count(&[on(1), on(1), on(2)]), HeadCount::Wrong { right: 0 });
    }

    #[test]
    fn every_split_puts_three_on_the_platforms() {
        let mut rng = rand::thread_rng();
        let mut seen = HashSet::new();
        for _ in 0..2000 {
            let split = draw_split(&mut rng);
            assert_eq!(split.iter().sum::<u8>(), 3);
            seen.insert(split);
        }
        // Every way of putting three on three platforms comes up.
        assert_eq!(seen.len(), 10);
    }

    #[test]
    fn the_lounges_pads_lead_up_or_down_to_fail() {
        let combo = [2, 3];
        let step = |id, name| climb(OPQ_LOUNGE, &combo, id, name);
        assert_eq!(step(12, "r4rp12"), Some(15));
        assert_eq!(step(11, "r4rp11"), Some(2));
        assert_eq!(step(16, "r4rp23"), Some(1));
        assert_eq!(step(15, "r4rp22"), Some(2));
        assert_eq!(step(7, "r4rp20"), Some(2));
        assert_eq!(step(17, "st00"), None);
    }

    #[test]
    fn the_tall_rooms_pads_lead_up_a_row_or_back_to_the_sections_start() {
        let combo = [1, 2, 3, 4].repeat(4);
        let step = |row: usize, pad: usize| {
            let id = (24 + (row - 1) * 4 + (pad - 1)) as u8;
            climb(OPQ_LEVER_ROOM, &combo, id, &format!("rp{row:02}{pad}"))
        };
        assert_eq!(step(1, 1), Some(28));
        assert_eq!(step(2, 2), Some(33));
        assert_eq!(step(4, 4), Some(1));
        assert_eq!(step(8, 4), Some(2));
        assert_eq!(step(12, 4), Some(3));
        assert_eq!(step(16, 4), Some(4));
        assert_eq!(step(1, 3), Some(5));
        assert_eq!(step(4, 1), Some(5));
        assert_eq!(step(5, 3), Some(1));
        assert_eq!(step(8, 1), Some(1));
        assert_eq!(step(9, 3), Some(2));
        assert_eq!(step(13, 3), Some(3));
        assert_eq!(step(16, 3), Some(3));
        assert_eq!(climb(OPQ_LEVER_ROOM, &combo, 5, "r6fail"), None);
    }

    #[test]
    fn every_climb_and_lever_pair_comes_up() {
        let mut rng = rand::thread_rng();
        let mut lounge = HashSet::new();
        let mut pairs = HashSet::new();
        for _ in 0..2000 {
            let combo = draw_climb(OPQ_LOUNGE, &mut rng);
            assert!(combo.iter().all(|pad| (1..=3).contains(pad)));
            lounge.insert(combo);
            let tower = draw_climb(OPQ_LEVER_ROOM, &mut rng);
            assert_eq!(tower.len(), 16);
            assert!(tower.iter().all(|pad| (1..=4).contains(pad)));
            let pair = draw_levers(&mut rng);
            assert_eq!(pair.iter().filter(|&&up| up).count(), 2);
            pairs.insert(pair);
        }
        assert_eq!(lounge.len(), 9);
        assert_eq!(pairs.len(), 10);
    }

    #[test]
    fn eak_wants_two_levers_up_and_hints_at_the_pair() {
        let pair = [true, false, false, true, false];
        let up = |names: &[usize]| {
            let mut up = [false; 5];
            for &name in names {
                up[name - 1] = true;
            }
            lever_check(pair, up)
        };
        assert_eq!(up(&[1, 4]), LeverCheck::Right);
        assert_eq!(up(&[]), LeverCheck::Few);
        assert_eq!(up(&[1]), LeverCheck::Few);
        assert_eq!(up(&[1, 2, 4]), LeverCheck::Few);
        assert_eq!(up(&[1, 2]), LeverCheck::Wrong { one_right: true });
        assert_eq!(up(&[2, 3]), LeverCheck::Wrong { one_right: false });
    }

    #[test]
    fn the_moon_bunny_makes_a_rice_cake_every_20_seconds() {
        // Unscaled outside the harness.
        let bunny = friendly_drop_period(crate::reactors::MOON_BUNNY, 6);
        assert_eq!(bunny, Some(Duration::from_secs(20) / scale()));
        let watch_hog = friendly_drop_period(9300102, 6);
        assert_eq!(watch_hog, Some(Duration::from_secs(60) / scale()));
        assert_eq!(friendly_drop_period(100100, 0), None);
    }

    #[test]
    fn kpq_stages_open_their_next_portal_once_cleared() {
        assert_eq!(KPQ.stage_on(103_000_800), Some(1));
        assert_eq!(KPQ.stage_on(103_000_804), Some(5));
        assert_eq!(KPQ.stage_on(103_000_805), None);
        assert_eq!(KPQ.stage_on(103_000_000), None);
        assert_eq!(KPQ.stage_map(1), KPQ.entry);
        assert_eq!(KPQ.gate_stage(103_000_800, "next00"), Some(1));
        assert_eq!(KPQ.gate_stage(103_000_803, "next00"), Some(4));
        assert_eq!(KPQ.gate_stage(103_000_804, "next00"), None);
        assert_eq!(KPQ.gate_stage(103_000_800, "st00"), None);
        assert_eq!((KPQ.stage_exp(1), KPQ.stage_exp(5)), (100, 1500));
        assert_eq!((KPQ.stage_exp(0), KPQ.stage_exp(6)), (0, 0));
    }

    #[test]
    fn lpq_stages_are_100_maps_apart_with_rooms_between() {
        assert_eq!(LPQ.stage_on(922_010_100), Some(1));
        assert_eq!(LPQ.stage_on(922_010_900), Some(9));
        assert_eq!(LPQ.stage_on(922_010_201), None);
        assert_eq!(LPQ.stage_on(922_011_000), None);
        assert_eq!(LPQ.stage_on(922_010_000), None);
        assert_eq!(
            (LPQ.stage_map(1), LPQ.stage_map(9)),
            (LPQ.entry, 922_010_900)
        );
        assert_eq!(LPQ.last_stage(), 9);
        assert_eq!(LPQ.gate_stage(922_010_800, "next00"), Some(8));
        assert_eq!(LPQ.gate_stage(922_010_600, "next00"), None);
        assert_eq!(LPQ.gate_stage(922_010_201, "next00"), None);
        assert!(LPQ.holds(922_011_100) && !LPQ.holds(LPQ.exit));
        assert_eq!(EventKind::Lpq.event(), &LPQ);
    }

    #[test]
    fn opq_holds_the_tower_but_not_its_way_out() {
        assert!(OPQ.holds(OPQ.entry) && OPQ.holds(920_011_300));
        assert!(!OPQ.holds(OPQ.exit) && !OPQ.holds(OPQ.lobby));
        assert_eq!((OPQ.items[0], OPQ.items[19]), (4_001_044, 4_001_063));
        assert_eq!(EventKind::Opq.event(), &OPQ);
    }

    #[test]
    fn the_dead_mine_is_one_stage_without_the_room_of_tragedy() {
        assert!(ZPQ.holds(ZPQ_HUB) && ZPQ.holds(280_010_011) && ZPQ.holds(280_011_006));
        assert!(!ZPQ.holds(ZPQ.exit) && !ZPQ.holds(280_090_000));
        assert_eq!((ZPQ.last_stage(), ZPQ.stage_exp(1)), (1, 12_000));
        assert_eq!(ZPQ.stage_map(1), ZPQ_HUB);
        assert!(ZPQ_DOCUMENTS > ZPQ.last_stage());
        assert_eq!(EventKind::Zpq.event(), &ZPQ);
        // The piece of Fire Ore is Adobis's next trial's, so it stays.
        assert!(!ZPQ.items.contains(&FIRE_ORE_PIECE));
    }

    #[test]
    fn only_a_team_of_every_job_line_is_blessed_in_orbis() {
        let mut rng = rand::thread_rng();
        // A warrior, a Fire/Poison mage, a hunter, a Night Walker (thief
        // line) and a gunslinger, with a beginner along.
        let all = [110, 210, 300, 1400, 520, 0];
        let mut seen = HashSet::new();
        for _ in 0..200 {
            seen.insert(OPQ.blessing(&all, &mut rng).unwrap());
        }
        assert_eq!(seen, OPQ.blessings.iter().copied().collect());
        // No pirate: nothing, however many others.
        assert_eq!(OPQ.blessing(&[100, 200, 300, 400, 412, 0], &mut rng), None);
        assert_eq!(OPQ.blessing(&[], &mut rng), None);
        // The other events bless nobody.
        assert_eq!(LPQ.blessing(&all, &mut rng), None);
    }

    #[test]
    fn opq_stages_follow_its_table_not_its_stride() {
        assert_eq!(OPQ.stage_on(OPQ.entry), Some(1));
        assert_eq!(OPQ.stage_map(1), OPQ.entry);
        assert_eq!(OPQ.stage_on(920_010_200), Some(2));
        assert_eq!(OPQ.stage_map(7), 920_010_700);
        assert_eq!(OPQ.stage_on(920_010_800), Some(8));
        assert_eq!(OPQ.stage_on(OPQ_HUB), Some(9));
        assert_eq!(OPQ.last_stage(), 9);
        assert_eq!(OPQ.stage_map(OPQ.last_stage()), OPQ_HUB);
        assert_eq!(OPQ.stage_on(920_010_900), None);
        assert_eq!(OPQ.stage_on(OPQ.exit), None);
        assert_eq!(
            (OPQ.stage_exp(1), OPQ.stage_exp(7), OPQ.stage_exp(8)),
            (3500, 3500, 0)
        );
        assert_eq!(OPQ.stage_exp(OPQ_WIN_STAGE), 3500);
        assert!(OPQ.member_clears.contains(&OPQ_WIN_STAGE));
        assert_eq!(OPQ.clear_map, None);
        assert_eq!(OPQ.clear_portal(1), Some("st00"));
        assert_eq!(OPQ.clear_portal(2), None);
        // The others keep their stride.
        assert_eq!(LPQ.stage_on(922_010_300), Some(3));
        assert_eq!(KPQ.stage_map(5), 103_000_804);
        assert_eq!(KPQ.clear_portal(1), None);
    }

    #[test]
    fn lpq_balloons_take_their_stages_passes() {
        let first = LPQ.hand_in_on(922_010_100, 2040036).unwrap();
        assert_eq!((first.stage, first.item, first.count), (1, LPQ_PASS, 25));
        assert_eq!(LPQ.hand_in_on(922_010_100, 2040037), None);
        assert_eq!(LPQ.hand_in_on(922_010_201, 2040037), None);
        assert_eq!(LPQ.hand_in(6), None);
        assert_eq!(LPQ.hand_in(8), None);
        let boss = LPQ.hand_in_on(922_010_900, 2040044).unwrap();
        assert_eq!((boss.stage, boss.item, boss.count), (9, LPQ_KEY, 1));
        assert_eq!(boss.stage, LPQ.last_stage());
        assert_eq!(KPQ.hand_in(1), None);
    }

    #[test]
    fn eak_takes_the_walkways_and_the_lounges_gatherings_without_an_intro() {
        let walkway = OPQ
            .hand_in_on(920_010_200, crate::npc_scripts::EAK)
            .unwrap();
        assert_eq!(
            (walkway.stage, walkway.item, walkway.count, walkway.prize),
            (2, 4_001_050, 30, Some(4_001_044))
        );
        let lounge = OPQ
            .hand_in_on(920_010_600, crate::npc_scripts::EAK)
            .unwrap();
        assert_eq!((lounge.stage, lounge.prize), (6, Some(4_001_048)));
        assert!(!walkway.intro && !lounge.intro);
        assert_eq!(OPQ.hand_in_on(920_010_000, crate::npc_scripts::EAK), None);
        assert!(LPQ.hand_ins.iter().all(|h| h.intro && h.prize.is_none()));
    }

    #[test]
    fn leaving_kpq_takes_its_coupons_and_passes() {
        let mut player = crate::codec::stats::sample_player();
        for (slot, (item_id, quantity)) in [(COUPON, 30), (PASS, 2), (COUPON, 5), (4000000, 3)]
            .into_iter()
            .enumerate()
        {
            let item = game::Item::stack(item_id, quantity);
            player
                .inventory
                .insert(game::InventoryType::Etc, slot as i16 + 1, item);
        }
        assert_eq!(KPQ.take_items(&mut player).len(), 3);
        assert!(player.is_dirty());
        assert_eq!(
            player.inventory.count(COUPON) + player.inventory.count(PASS),
            0
        );
        assert_eq!(player.inventory.count(4000000), 3);
        assert!(KPQ.take_items(&mut player).is_empty());
    }

    #[test]
    fn hpq_is_played_on_primrose_hill_and_left_by_its_exit() {
        let data = crate::game_data::get().unwrap();
        let npcs = |map_id| -> Vec<i32> {
            data.field(map_id)
                .unwrap()
                .map_npcs
                .iter()
                .map(|npc| npc.npc_id)
                .collect()
        };
        assert!(npcs(HPQ.lobby).contains(&1012112));
        assert!(npcs(HPQ.entry).contains(&1012114));
        assert!(npcs(HPQ.exit).contains(&1012113));
        assert!(HPQ.holds(HPQ.entry) && HPQ.holds(910_010_400));
        assert!(!HPQ.holds(HPQ.exit) && !HPQ.holds(HPQ.lobby));
        assert_eq!(data.field(HPQ.entry).unwrap().forced_return, Some(HPQ.exit));
        assert_eq!((HPQ.stage_on(HPQ.entry), HPQ.last_stage()), (Some(1), 1));
        assert_eq!(HPQ.gate_stage(HPQ.entry, "next00"), None);
        let party = [
            at(1, 10, HPQ.lobby),
            at(2, 200, HPQ.lobby),
            at(3, 9, HPQ.lobby),
        ];
        assert_eq!(HPQ.team(1, &party), None);
        let party = [
            at(1, 10, HPQ.lobby),
            at(2, 200, HPQ.lobby),
            at(3, 11, HPQ.lobby),
        ];
        assert_eq!(HPQ.team(1, &party), Some(vec![1, 2, 3]));
    }

    #[test]
    fn the_team_is_those_in_the_lobby_at_the_right_levels() {
        let lobby = KPQ.lobby;
        let party = [
            at(1, 25, lobby),
            at(2, 21, lobby),
            at(3, 30, lobby),
            at(4, 31, lobby),
            at(5, 25, 100_000_000),
        ];
        assert_eq!(KPQ.team(1, &party), Some(vec![1, 2, 3]));
    }

    #[test]
    fn a_team_needs_its_leader_and_three_or_four() {
        let lobby = KPQ.lobby;
        let three = [at(1, 25, lobby), at(2, 25, lobby), at(3, 25, lobby)];
        assert_eq!(KPQ.team(4, &three), None);
        assert_eq!(KPQ.team(1, &three[..2]), None);
        let mut five = three.to_vec();
        five.extend([at(4, 25, lobby), at(5, 25, lobby)]);
        assert_eq!(KPQ.team(1, &five), None);
        assert_eq!(KPQ.team(1, &five[..4]), Some(vec![1, 2, 3, 4]));
        let low_leader = [at(1, 20, lobby), at(2, 25, lobby), at(3, 25, lobby)];
        assert_eq!(KPQ.team(1, &low_leader), None);
    }

    #[test]
    fn the_event_ends_when_its_leader_leaves_or_too_few_remain() {
        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        assert!(!team.leave(4));
        assert!(!team.leave(4));
        assert!(team.leave(3));

        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        assert!(team.leave(1));
        assert_eq!(team.members(), [2, 3, 4]);
    }

    #[test]
    fn a_new_leader_in_the_event_leads_it_and_one_outside_ends_it() {
        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        assert!(!team.hand_over(2));
        assert_eq!(team.leader(), 2);
        assert!(!team.leave(1));
        assert!(team.leave(2));

        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        team.leave(4);
        assert!(team.hand_over(4));

        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3]);
        team.win();
        assert!(!team.hand_over(5));
        assert!(!team.leave(1));
    }

    #[test]
    fn a_squad_ends_only_when_its_last_member_leaves_and_its_own_may_come_back() {
        let mut team = Team::new(&ZAKUM, 1, vec![1, 2, 3]);
        // Its leader's leaving, or too few, ends nothing; nor does a
        // party's hand-over move it.
        assert!(!team.leave(1));
        assert!(!team.hand_over(9));
        assert_eq!(team.leader(), 1);
        assert!(!team.leave(2));
        assert!(!team.rejoin(9), "not on the roster");
        team.enlist(&[9]);
        assert!(team.rejoin(9), "stayed behind");
        assert!(!team.leave(9));
        assert!(!team.rejoin(3), "still in");
        assert!(team.rejoin(1));
        assert_eq!(team.members(), [3, 1]);
        assert!(!team.leave(3));
        team.win();
        assert!(!team.rejoin(2), "won");
        assert!(team.leave(1));
        // A party's team never takes anyone back.
        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        assert!(!team.leave(4));
        assert!(!team.rejoin(4));
        assert_eq!(ZAKUM.least(), 6);
        assert!(EventKind::Zakum.event().holds(280_030_000));
        assert!(!ZAKUM.holds(ZAKUM.lobby));
    }

    #[test]
    fn papulatus_ends_only_when_its_last_member_leaves() {
        let mut team = Team::new(&PAPULATUS, 1, vec![1, 2, 3]);
        // A hand-over moves the lead, even to one outside, and ends nothing.
        assert!(!team.hand_over(9));
        assert_eq!(team.leader(), 9);
        assert!(!team.leave(1));
        assert!(!team.leave(2));
        assert!(!team.rejoin(1), "a party's team takes no one back");
        assert!(team.leave(3));
        assert!(PAPULATUS.team(1, &[at(1, 1, PAPULATUS.lobby)]).is_some());
        assert!(PAPULATUS.holds(ORIGIN_OF_CLOCKTOWER));
        assert!(!PAPULATUS.holds(CLOCKTOWER_DEPTHS));
        assert_eq!(PAPULATUS.gate.len(), 3);
    }

    #[test]
    fn a_won_event_ends_when_its_last_member_leaves() {
        let mut team = Team::new(&KPQ, 1, vec![1, 2, 3]);
        team.win();
        assert!(!team.leave(1));
        assert!(!team.leave(2));
        assert!(!team.leave(2));
        assert!(team.leave(3));
    }

    #[test]
    fn the_leader_wins_kpq_with_ten_passes_on_its_fifth_stage() {
        let team = Team::new(&KPQ, 1, vec![1, 2, 3]);
        assert_eq!(KPQ.last_stage(), 5);
        assert_eq!(KPQ.stage_on(KPQ.stage_map(5)), Some(5));
        assert_eq!(KPQ.gate_stage(KPQ.stage_map(5), "next00"), None);
        assert_eq!(
            KPQ.last_stage_talk(1, &team, false),
            Cloto::Leader {
                stage: 5,
                needed: 10
            }
        );
        assert_eq!(KPQ.last_stage_talk(2, &team, false), Cloto::Gather);
        assert_eq!(KPQ.last_stage_talk(1, &team, true), Cloto::Won);
        assert_eq!(KPQ.last_stage_talk(2, &team, true), Cloto::Won);
    }

    #[test]
    fn kpq_lasts_half_an_hour_on_its_own_maps() {
        assert_eq!(KPQ.duration(), Duration::from_secs(30 * 60));
        assert!(KPQ.holds(103_000_800) && KPQ.holds(103_000_805));
        assert!(!KPQ.holds(KPQ.exit) && !KPQ.holds(KPQ.lobby));
    }

    #[test]
    fn the_leader_answers_cloto_s_puzzles_drawn_at_their_first_talk() {
        let team = Team::new(&KPQ, 1, vec![1, 2, 3]);
        let mut puzzles = Puzzles::default();
        let mut draws = 0;
        let mut draw = |combo| {
            draws += 1;
            Some(combo)
        };

        assert_eq!(puzzles.talk(2, 2, &team, false, || draw(7)), Cloto::Follow);
        assert!(!puzzles.set(2));
        assert_eq!(
            puzzles.talk(2, 1, &team, false, || draw(7)),
            Cloto::Intro { stage: 2 }
        );
        assert!(puzzles.set(2) && !puzzles.set(3));
        for _ in 0..2 {
            assert_eq!(
                puzzles.talk(2, 1, &team, false, || draw(11)),
                Cloto::Puzzle { stage: 2, combo: 7 }
            );
        }
        // Each stage has its own.
        assert_eq!(
            puzzles.talk(3, 1, &team, false, || draw(11)),
            Cloto::Intro { stage: 3 }
        );
        assert_eq!(
            puzzles.talk(3, 1, &team, false, || draw(7)),
            Cloto::Puzzle {
                stage: 3,
                combo: 11
            }
        );
        assert_eq!(draws, 2);
        assert_eq!(puzzles.talk(2, 1, &team, true, || None), Cloto::Cleared);
        assert_eq!(puzzles.talk(2, 3, &team, true, || None), Cloto::Cleared);
        // A map without areas sets no puzzle.
        assert_eq!(
            puzzles.talk(4, 1, &team, false, || None),
            Cloto::Intro { stage: 4 }
        );
        assert!(!puzzles.set(4));
        assert_eq!(draw_combo(2, 3), None);
        assert!(draw_combo(4, 3).is_some_and(|combo| combos(4, 3).any(|c| c == combo)));
    }

    #[test]
    fn each_puzzle_stage_has_a_combo_for_every_choice_of_its_areas() {
        for (areas, taken, count) in [(4, 3, 4), (5, 3, 10), (6, 3, 20), (9, 5, 126)] {
            let all: Vec<u32> = combos(areas, taken).collect();
            assert_eq!(all.len(), count);
            assert!(all
                .iter()
                .all(|c| c.count_ones() == taken && *c < 1 << areas));
            assert_eq!(all.iter().collect::<HashSet<_>>().len(), count);
        }
    }

    #[test]
    fn the_team_must_stand_one_on_each_of_the_combos_areas() {
        let area = |x1| Area {
            x1,
            x2: x1 + 10,
            y1: 0,
            y2: 10,
        };
        let areas = [area(0), area(20), area(40), area(60)];
        let combo = 0b1011;
        let on = |area: i32| (area * 20 + 5, 5);

        assert!(on_areas(combo, &areas, &[on(0), on(1), on(3)]));
        // A fourth member off every area doesn't count.
        assert!(on_areas(combo, &areas, &[on(3), (100, 5), on(0), on(1)]));
        // Two on one area, one off them, or one on the wrong area.
        assert!(!on_areas(combo, &areas, &[on(0), on(0), on(3)]));
        assert!(!on_areas(combo, &areas, &[on(0), on(1), (15, 5)]));
        assert!(!on_areas(combo, &areas, &[on(0), on(1), on(2)]));
        // A box holds its first edges but not its second.
        assert!(on_areas(combo, &areas, &[(0, 0), (20, 9), (60, 0)]));
        assert!(!on_areas(combo, &areas, &[(10, 5), on(1), on(3)]));
        assert!(!on_areas(combo, &areas, &[on(0), (25, 10), on(3)]));
        assert_eq!(standing_on_areas(&areas, &[on(0), on(0), (100, 5)]), 2);
        // Checked only with that many on the areas, each one once.
        let stand = |positions: &[(i32, i32)]| standing(combo, 3, &areas, positions);
        assert_eq!(stand(&[on(0), on(1), (100, 5)]), Standing::Few);
        assert_eq!(stand(&[on(0), on(1), on(3), on(2)]), Standing::Few);
        assert_eq!(stand(&[on(0), on(0), on(3)]), Standing::Wrong);
        assert_eq!(stand(&[on(3), on(1), (100, 5), on(0)]), Standing::Right);
    }

    #[test]
    fn a_position_counts_for_the_first_area_holding_it() {
        let wide = Area {
            x1: 0,
            x2: 100,
            y1: 0,
            y2: 10,
        };
        let narrow = Area { x2: 10, ..wide };
        assert!(on_areas(0b01, &[wide, narrow], &[(5, 5)]));
        assert!(!on_areas(0b10, &[wide, narrow], &[(5, 5)]));
    }

    #[test]
    fn cloto_asks_each_member_one_question_and_the_leader_for_passes() {
        let team = Team::new(&KPQ, 1, vec![1, 2, 3, 4]);
        let mut stage = Stage1::default();
        assert_eq!(
            stage.talk(1, &team, || 0),
            Cloto::Leader {
                stage: 1,
                needed: 3
            }
        );
        let asked = Cloto::Member {
            question: 4,
            new: true,
            passed: false,
        };
        assert_eq!(stage.talk(2, &team, || 4), asked);
        assert_eq!(
            stage.talk(2, &team, || 6),
            Cloto::Member {
                question: 4,
                new: false,
                passed: false
            }
        );
        stage.pass_given(2);
        assert_eq!(
            stage.talk(2, &team, || 6),
            Cloto::Member {
                question: 4,
                new: false,
                passed: true
            }
        );
        assert!(stage.clear());
        assert!(!stage.clear());
        assert_eq!(stage.talk(3, &team, || 0), Cloto::Cleared);
        assert_eq!(stage.talk(1, &team, || 0), Cloto::Cleared);
    }
}
