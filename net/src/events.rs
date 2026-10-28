//! Party events played in a copy of their maps of their own (an instance):
//! who may start one, where it is played, and when it ends. Pure rules;
//! the runtime's event actor keeps the time and moves the players.

use crate::carnival::{CARNIVAL_LOBBY, CARNIVAL_ROOMS};
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
    /// The expedition to Horntail's Cave, gathered by the Mark of the Squad
    /// (2083004) at the Entrance to Horntail's Cave (`crate::squads`).
    Horntail,
    /// Horntail's party quest on the Trial Grounds, started at the
    /// gatekeeper (2083001) at the Cave Entrance.
    Htpq,
    /// Delli's rescue (the Corsairs' 6410), started at Delli (2095000) on
    /// Looking for Delli 3.
    Delli,
    /// The Ludibrium Maze, started at Rolly (9103001) in Ludibrium.
    LudiMaze,
    /// Protecting Tylus (the warriors' 6192), started at Tylus (2020008)
    /// in El Nath's Chief's Residence.
    Tylus,
    /// The Forest of Poison Haze, Ellin Forest's party quest, started at
    /// Ellin (2133000) in the Deep Fairy Forest.
    Ellin,
    /// The Pirate Ship, Herb Town's party quest against the Lord Pirate,
    /// started at Guon (2094000) Over the Pirate Ship.
    Pirate,
    /// The Mu Lung Dojo's 38 floors, taken alone, started at So Gong
    /// (2091005) in the Dojo's hall.
    Dojo,
    /// So Gong's trial for a first-time challenger, taken alone from the
    /// Dojo's hall: beat him to go on to the Dojo itself.
    DojoTrial,
    /// The Mu Lung Dojo taken by a party, started by its leader at So Gong
    /// in the hall: the same floors in copies of their own.
    DojoParty,
    /// Nett's Pyramid in one of its modes, taken alone, started at Duarte
    /// (2103013) in its lobby.
    Pyramid(PyramidMode),
    /// Nett's Pyramid in one of its modes, taken by a party of two or more,
    /// started by its leader at Duarte.
    PyramidParty(PyramidMode),
    /// Pharaoh Yeti's Tomb for one of the Pyramid's modes, taken alone,
    /// with that mode's gem at Duarte in the lobby or straight from its
    /// room in the Shades after a run.
    PyramidTomb(PyramidMode),
    /// Jr. Balrog's room before Tristan's Tomb (2245), taken alone from
    /// Mu Young (1061014) in the Sleepywood dungeon.
    Tristan,
    /// Eos Tower's dollhouse room, taken alone from Olson the Toy Soldier
    /// (2040002) to find the pendulum for Dollhouse (3230).
    Dollhouse,
    /// The Toy Factory's Sector 4, taken alone from Cheng the Assistant
    /// (2040003) to find the Machine Parts for The Missing Mechanical Parts
    /// (3239).
    MechanicalParts,
    /// King Pepe's room in the Mushroom Castle, taken alone through the
    /// East Castle Tower's `TD_MC_enterboss1` while Stop the Wedding (2330)
    /// still wants a Yeti brother.
    KingPepe,
    /// The Mushroom Castle's Wedding Hall, taken alone through its
    /// entrance's `TD_MC_enterboss2` to find Violetta and face the Prime
    /// Minister (Where's Violetta? and The Story of Betrayal, 2332-2333).
    WeddingHall,
    /// The Spirit of Rock's floors above Kerning Square's 7th floor, taken
    /// alone from June (1052125) by one helping Blake (2286-2288).
    RockSpirit,
    /// The VIP Zone's floors above Kerning Square's 7th floor, taken alone
    /// on handing June 10 VIP Invitations (Admission to the VIP Zone, 2291).
    RockSpiritVip,
    /// A Monster Carnival room (1 to 6), taken by a party from Spiegelmann
    /// (2042000) on the Carnival Field (`crate::carnival`).
    Carnival(u8),
    /// Romeo and Juliet's party quest in Magatia, from one of its sides.
    Magatia(MagatiaSide),
    /// The Guild Quest to Sharenian, a registered guild's turn on its
    /// channel (`crate::guild_quest`), its members let in by Shuang
    /// (9040000) at the Excavation Site's camp.
    GuildQuest,
}

/// Magatia's two sides, each with its own way into Romeo and Juliet's
/// party quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MagatiaSide {
    /// Zenumist's, started at Romeo (2112004).
    Zenumist,
    /// Alcadno's, started at Juliet (2112003).
    Alcadno,
}

impl MagatiaSide {
    pub const ALL: [MagatiaSide; 2] = [MagatiaSide::Zenumist, MagatiaSide::Alcadno];

    /// The side whose copied maps (its exit too) hold `map_id`, if any.
    pub fn of(map_id: i32) -> Option<MagatiaSide> {
        Self::ALL
            .into_iter()
            .find(|&side| EventKind::Magatia(side).event().maps.contains(&map_id))
    }

    /// The stage maps of its run, in order.
    pub fn stages(self) -> &'static [i32; 7] {
        match self {
            MagatiaSide::Zenumist => &ZENUMIST_STAGES,
            MagatiaSide::Alcadno => &ALCADNO_STAGES,
        }
    }

    /// The door of its beaker room, which opens as the stage clears.
    pub fn beaker_door(self) -> &'static str {
        match self {
            MagatiaSide::Zenumist => "rnj2_door",
            MagatiaSide::Alcadno => "jnr2_door",
        }
    }

    /// The two doors of its hub (`stages()[3]`) that take a Card Key
    /// (`MAGATIA_CARD_KEY`), each with the room beside the hub it lets in
    /// to and that room's own way back, which opens with it (Cosmic
    /// `2618002` and `2618001`, west and east).
    pub fn side_doors(self) -> [(&'static str, i32, &'static str); 2] {
        let hub = self.stages()[3];
        match self {
            MagatiaSide::Zenumist => [
                ("rnj3_out1", hub + 1, "rnj31_out"),
                ("rnj3_out2", hub + 2, "rnj32_out"),
            ],
            MagatiaSide::Alcadno => [
                ("jnr3_out1", hub + 1, "jnr31_out"),
                ("jnr3_out2", hub + 2, "jnr32_out"),
            ],
        }
    }

    /// The hub's way on, which its Romeo or Juliet opens once the files
    /// are handed in (Cosmic `rnj3_out3`).
    pub fn hub_door(self) -> &'static str {
        match self {
            MagatiaSide::Zenumist => "rnj3_out3",
            MagatiaSide::Alcadno => "jnr3_out3",
        }
    }

    /// The way out of Yulete's office (`stages()[4]`), which opens as its
    /// stage clears (Cosmic `2618005`, `rnj6_out`).
    pub fn office_door(self) -> &'static str {
        match self {
            MagatiaSide::Zenumist => "rnj6_out",
            MagatiaSide::Alcadno => "jnr6_out",
        }
    }

    /// Which of its four rope rooms, beside the landing (`stages()[5]`),
    /// `map_id` is, from 0.
    pub fn rope_room(self, map_id: i32) -> Option<u8> {
        let first = self.stages()[5] + 1;
        (first..first + MAGATIA_ROPE_ROOMS as i32)
            .contains(&map_id)
            .then(|| (map_id - first) as u8)
    }

    /// The letter one of its library's books may hide (Cosmic `2112007`
    /// and `2112013`).
    pub fn letter(self) -> i32 {
        match self {
            MagatiaSide::Zenumist => 4_001_131,
            MagatiaSide::Alcadno => 4_001_130,
        }
    }

    /// The one the party escorts in the boss's room: Juliet for
    /// Zenumist, Romeo for Alcadno (Mob.nx 9300137 and 9300138).
    pub fn escort(self) -> i32 {
        match self {
            MagatiaSide::Zenumist => 9_300_137,
            MagatiaSide::Alcadno => 9_300_138,
        }
    }

    /// The Frankenroid Yulete brings out, the mighty one if he was
    /// `caught` off guard (Cosmic `2112000` and `2112010`).
    pub fn frankenroid(self, caught: bool) -> i32 {
        match (self, caught) {
            (MagatiaSide::Zenumist, false) => 9_300_139,
            (MagatiaSide::Zenumist, true) => 9_300_140,
            (MagatiaSide::Alcadno, false) => 9_300_151,
            (MagatiaSide::Alcadno, true) => 9_300_152,
        }
    }

    /// What Romeo or Juliet gives each member after the win, unless the
    /// escort fainted (Cosmic `2112006` and `2112005`).
    pub fn gift(self) -> i32 {
        match self {
            MagatiaSide::Zenumist => 4_001_159,
            MagatiaSide::Alcadno => 4_001_160,
        }
    }

    /// The room of Yulete's traces, where a member goes after the win
    /// unless the plain Frankenroid was beaten without the escort
    /// fainting.
    pub fn traces_room(self) -> i32 {
        self.stages()[0] + 500
    }

    /// The room where the party is thanked and rewarded, last.
    pub fn thanks_room(self) -> i32 {
        self.stages()[0] + 600
    }
}

/// Nett's Pyramid's modes, in Duarte's order (Cosmic `PyramidMode`); each
/// has stage maps of its own, a run alone and with a party.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyramidMode {
    Easy,
    Normal,
    Hard,
    Hell,
}

impl PyramidMode {
    pub const ALL: [PyramidMode; 4] = [
        PyramidMode::Easy,
        PyramidMode::Normal,
        PyramidMode::Hard,
        PyramidMode::Hell,
    ];

    /// Its digit in its maps' ids, 0 to 3.
    pub const fn index(self) -> i32 {
        self as i32
    }
}

/// An event's maps and terms.
#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    /// Where the party gathers and its leader starts the event.
    pub lobby: i32,
    /// Other maps it may be started on, by a team there (Pharaoh Yeti's
    /// Tomb from its mode's room in the Shades).
    pub entrances: &'static [i32],
    /// The maps the party plays in its own copies of.
    pub maps: RangeInclusive<i32>,
    /// Where everyone starts.
    pub entry: i32,
    /// How many maps from `entry` on the members start on, each on one
    /// drawn for them (the Ludibrium Maze's rooms, Cosmic `playerEntry`);
    /// 1 for an event everyone starts on `entry`.
    pub entries: i32,
    /// Where everyone goes when they leave or the event ends. Never one of
    /// the event's maps, even within `maps`.
    pub exit: i32,
    pub levels: RangeInclusive<u8>,
    /// How far apart the team's levels may be, if that matters (the party
    /// Dojo's 30, Cosmic `2091005`'s `isBetween`).
    pub spread: Option<u8>,
    /// How many may play, the leader included.
    pub team: RangeInclusive<usize>,
    /// Its time limit; 0 for none.
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
    /// The friendly mob the team protects, if any: its going loses the
    /// event at once, and the time running out with it still there wins
    /// it (Cosmic DelliBattle's `friendlyKilled` and `scheduledTimeout`).
    pub survive: Option<i32>,
    /// How many may run at once on a channel, each team in its own copies
    /// of the maps: 1 for most, 15 for the Mu Lung Dojo's solo runs
    /// (Cosmic `Channel.ingressDojo`'s solo slots).
    pub runs: usize,
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

/// A won event's bonus: the team plays `map` for `seconds`, then whoever is
/// still there goes on to `exit`.
#[derive(Debug, PartialEq, Eq)]
pub struct Bonus {
    pub map: i32,
    pub exit: i32,
    pub seconds: u64,
    /// Whether the win itself takes the team there, rather than the leader
    /// later.
    pub on_win: bool,
}

impl Bonus {
    /// How long the bonus lasts, shortened like the event's time.
    pub fn duration(&self) -> Duration {
        scaled(Duration::from_secs(self.seconds))
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
    entrances: &[],
    maps: 103_000_800..=103_000_805,
    entry: 103_000_800,
    entries: 1,
    exit: 103_000_890,
    levels: 21..=30,
    spread: None,
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
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: 910_010_000..=910_010_400,
    entry: 910_010_000,
    entries: 1,
    exit: 910_010_300,
    levels: 10..=u8::MAX,
    spread: None,
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
        seconds: 5 * 60,
        on_win: false,
    }),
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: 922_010_000..=922_011_100,
    entry: 922_010_100,
    entries: 1,
    exit: 922_010_000,
    levels: 35..=50,
    spread: None,
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
        seconds: 60,
        on_win: true,
    }),
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: 920_010_000..=920_011_300,
    entry: 920_010_000,
    entries: 1,
    exit: 920_011_200,
    levels: 51..=70,
    spread: None,
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
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: 280_010_000..=280_011_006,
    entry: ZPQ_HUB,
    entries: 1,
    exit: 211_042_300,
    levels: 50..=u8::MAX,
    spread: None,
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
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: crate::mobs::ZAKUM_ALTAR_MAP..=crate::mobs::ZAKUM_ALTAR_MAP,
    entry: crate::mobs::ZAKUM_ALTAR_MAP,
    entries: 1,
    exit: crate::npc_scripts::ENTRANCE_TO_ZAKUM,
    levels: 50..=u8::MAX,
    spread: None,
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
    survive: None,
    runs: 1,
};

/// The Guild Quest to Sharenian (Cosmic `GuildQuest`): the guild whose
/// turn it is on the channel gathers in the waiting room, stage 1, whose
/// `join00` opens once its three minutes are up; Sharenian's stages follow
/// a hundred map ids apart, up to the boss's 990000900. Its members come in
/// one by one through Shuang while it gathers, and once it is under way it
/// ends when fewer than six remain or its registrant leaves. The Rubian
/// handed in wins it and takes the guild to Sharenian's treasure room for
/// half a minute (Cosmic `clearPQ`'s `bonusTime`), then out.
pub const GUILD_QUEST: Event = Event {
    lobby: crate::guild_quest::CAMP,
    entrances: &[],
    maps: crate::guild_quest::WAITING_ROOM..=990_001_101,
    entry: crate::guild_quest::WAITING_ROOM,
    entries: 1,
    exit: crate::guild_quest::RETURNING_PATH,
    levels: 1..=u8::MAX,
    spread: None,
    team: 6..=30,
    minutes: 90,
    exp_per_stage: &[0; 10],
    stride: 100,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[crate::guild_quest::WAITING_ROOM],
    puzzles: 1..=0,
    areas_taken: 0,
    items: crate::guild_quest::ITEMS,
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: None,
    bonus: Some(Bonus {
        map: crate::guild_quest::BONUS_MAP,
        exit: crate::guild_quest::RETURNING_PATH,
        seconds: 30,
        on_win: true,
    }),
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: true,
    survive: None,
    runs: 1,
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
    entrances: &[],
    maps: ORIGIN_OF_CLOCKTOWER..=ORIGIN_OF_CLOCKTOWER,
    entry: ORIGIN_OF_CLOCKTOWER,
    entries: 1,
    exit: CLOCKTOWER_DEPTHS,
    levels: 1..=u8::MAX,
    spread: None,
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
    survive: None,
    runs: 1,
};

/// The expedition to Horntail (Cosmic `HorntailBattle`): like Zakum's, the
/// squad's members on the Entrance to Horntail's Cave when its leader
/// starts it go to their own copy of the cave, from the first trial
/// (240060000) on to Horntail's Cave (240060200), for two hours. It ends
/// only when the last of them leaves (`isExpeditionTeamLackingNow`, which
/// asks for neither the leader nor the minimum once in), everyone going to
/// the Road in Between. Each trial is a stage, cleared when its head
/// (`HORNTAIL_HEADS`) dies, which unseals its door (`hontale_BR`); the
/// cave is the last, cleared, and the expedition won, when Horntail dies.
/// No stage gives EXP.
pub const HORNTAIL: Event = Event {
    lobby: crate::npc_scripts::HORNTAIL_CAVE_ENTRANCE,
    entrances: &[],
    maps: HORNTAIL_TRIAL_I..=HORNTAILS_CAVE,
    entry: HORNTAIL_TRIAL_I,
    entries: 1,
    exit: crate::npc_scripts::ROAD_IN_BETWEEN,
    levels: 100..=u8::MAX,
    spread: None,
    team: 6..=30,
    minutes: 120,
    exp_per_stage: &[0, 0, 0],
    stride: 1,
    stage_maps: &[HORNTAIL_TRIAL_I, HORNTAIL_TRIAL_II, HORNTAILS_CAVE],
    talk_clears: &[],
    member_clears: &[1, 2, 3],
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
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// The time Horntail's expedition has left once Horntail comes out.
pub const HORNTAIL_FIGHT_MINUTES: u64 = 60;

/// The Cave of Life's two trials, the first where Horntail's expedition
/// comes in, and Horntail's Cave, the last of its maps.
pub const HORNTAIL_TRIAL_I: i32 = 240_060_000;
pub const HORNTAIL_TRIAL_II: i32 = 240_060_100;
pub const HORNTAILS_CAVE: i32 = 240_060_200;

/// A head of Horntail's that a trial's copy opens with, and whose death
/// clears the trial's stage.
#[derive(Debug, PartialEq, Eq)]
pub struct TrialHead {
    pub map: i32,
    pub mob: i32,
    /// Where it comes out, on the ground below.
    pub at: (i16, i16),
}

/// The Left Head on the first trial and the Right Head on the second
/// (Cosmic HorntailBattle's `setup`, `monsterKilled`).
pub const HORNTAIL_HEADS: [TrialHead; 2] = [
    TrialHead {
        map: HORNTAIL_TRIAL_I,
        mob: 8_810_000,
        at: (960, 120),
    },
    TrialHead {
        map: HORNTAIL_TRIAL_II,
        mob: 8_810_001,
        at: (-420, 120),
    },
];

/// The head of the trial on `map_id`, if any, and the trial's stage.
pub fn trial_head(map_id: i32) -> Option<(u8, &'static TrialHead)> {
    let index = HORNTAIL_HEADS.iter().position(|head| head.map == map_id)?;
    Some((HORNTAIL.stage_on(map_id)?, &HORNTAIL_HEADS[index]))
}

/// Horntail's party quest (Cosmic `HorntailPQ`): six of a party, all of
/// level 120 or more, go from the Cave Entrance into their own copy of the
/// Trial Grounds for 25 minutes. Stages 1 to 4 are the rooms whose seals
/// break when their keys are thrown on them, stage 5 the last room's, which
/// the leader lifts with the Blue Crystal Key; handing the gatekeeper the
/// Blue Key and six Red Keys on either last map wins it (stage 6), which
/// takes the team to the Entrance to Horntail's Cave. On the first map the
/// leader hands the gatekeeper the five Crystal Keys, which takes the team
/// on to the lamp's room. No stage gives EXP, and there is no prize. A
/// member who dies lands outside it (240050500), and any one member's
/// leaving ends it for all (Cosmic `isEventTeamLackingNow`, with
/// six the least).
pub const HTPQ: Event = Event {
    lobby: crate::npc_scripts::CAVE_ENTRANCE,
    entrances: &[],
    maps: HTPQ_ENTRY..=HTPQ_DARK,
    entry: HTPQ_ENTRY,
    entries: 1,
    exit: crate::npc_scripts::CAVE_ENTRANCE,
    levels: 120..=u8::MAX,
    spread: None,
    team: 6..=6,
    minutes: 25,
    exp_per_stage: &[0; 6],
    stride: 1,
    stage_maps: &[
        HTPQ_ENTRY + 1,
        HTPQ_ENTRY + 2,
        HTPQ_ENTRY + 3,
        HTPQ_ENTRY + 4,
        HTPQ_ENTRY + 5,
    ],
    talk_clears: &[5],
    member_clears: &[1, 2, 3, 4],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[
        CRYSTAL_KEYS[0],
        CRYSTAL_KEYS[1],
        CRYSTAL_KEYS[2],
        CRYSTAL_KEYS[3],
        CRYSTAL_KEYS[4],
        BLUE_CRYSTAL_KEY,
        RED_CRYSTAL_KEY,
    ],
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: Some(crate::npc_scripts::HORNTAIL_CAVE_ENTRANCE),
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// The Trial Grounds' first map, where the team comes in and the leader
/// hands in the five Crystal Keys; the lamp's room they then go to; and
/// the two last maps, one of which the lamp opens.
pub const HTPQ_ENTRY: i32 = 240_050_100;
pub const HTPQ_LAMP: i32 = 240_050_200;
pub const HTPQ_LIGHT: i32 = 240_050_300;
pub const HTPQ_DARK: i32 = 240_050_310;
/// The five Crystal Keys: the first from the entry's monsters, the others
/// made by the seals of the four rooms.
pub const CRYSTAL_KEYS: [i32; 5] = [4_001_087, 4_001_088, 4_001_089, 4_001_090, 4_001_091];
/// The Blue Crystal Key, from the last room's monster, which lifts its
/// seal and goes to the gatekeeper at the end; and the Red Crystal Keys,
/// six of which go with it.
pub const BLUE_CRYSTAL_KEY: i32 = 4_001_092;
pub const RED_CRYSTAL_KEY: i32 = 4_001_093;
pub const RED_KEYS_WANTED: i16 = 6;
/// Where on the first map a room's key lands once its seal takes it: the
/// spot of the reactor `keyDrop1` there (Cosmic `2408002`).
pub const HTPQ_KEY_AT: (i16, i16) = (874, -55);

/// The Crystal Key that the seal of the room of `stage` (1 to 4) sends to
/// the first map once it takes the key thrown on it: the same key.
pub fn htpq_room_key(stage: u8) -> Option<i32> {
    (1..=4)
        .contains(&stage)
        .then(|| CRYSTAL_KEYS[usize::from(stage)])
}

/// Looking for Delli 3, where the party gathers; Save Delli!, where it
/// protects Willi; and the Nautilus Hideout Room past it.
pub const DELLI_LOBBY: i32 = 925_010_200;
pub const SAVE_DELLI: i32 = 925_010_300;
pub const NAUTILUS_HIDEOUT: i32 = 925_010_400;
/// Willi, the friendly mob on Save Delli! that the monsters there go for.
pub const WILLI: i32 = 9_300_162;

/// Delli's rescue (Cosmic `DelliBattle`): one or two of a party, all of
/// level 120 or more, on Looking for Delli 3 with their leader, go to their
/// own copy of Save Delli! for six minutes. Willi's going there loses it,
/// sending them back; the time running out with Willi still there wins it,
/// and every other monster there goes (Cosmic `killAllMonstersNotFriendly`).
/// Its one stage gives no EXP; the way out past it (`s4mind_end`) marks the
/// quest's record.
pub const DELLI: Event = Event {
    lobby: DELLI_LOBBY,
    entrances: &[],
    maps: SAVE_DELLI..=SAVE_DELLI,
    entry: SAVE_DELLI,
    entries: 1,
    exit: DELLI_LOBBY,
    levels: 120..=u8::MAX,
    spread: None,
    team: 1..=2,
    minutes: 6,
    exp_per_stage: &[0],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
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
    gate: &[],
    until_empty: false,
    survive: Some(WILLI),
    runs: 1,
};

/// Ludibrium, where the Ludibrium Maze's party gathers.
pub const LUDIBRIUM: i32 = 220_000_000;
/// The Ludibrium Maze's first room.
pub const MAZE_ROOM: i32 = 809_050_000;
/// The goal room, reached only from room 8, where Pietri takes the coupons.
pub const MAZE_GOAL: i32 = 809_050_015;
/// Where the team goes once Pietri has them.
pub const MAZE_CLEAR: i32 = 809_050_016;
/// The way out, where Rolly sends everyone back to Ludibrium.
pub const MAZE_EXIT: i32 = 809_050_017;
/// The Ludibrium Maze's coupons, from its monsters.
pub const MAZE_COUPON: i32 = 4_001_106;
/// How many coupons Pietri wants at the least for the win.
pub const MAZE_COUPONS_WANTED: i32 = 30;
/// The EXP every member gets for each coupon handed in.
pub const MAZE_COUPON_EXP: i32 = 50;
/// What Rolly in the clear room gives each member of a team that won the
/// Ludibrium Maze: one of these items, and how many, all equally likely
/// (Cosmic `LudiMazePQ` `setEventRewards`).
pub const MAZE_REWARDS: [(i32, i16); 47] = [
    (1442017, 1),
    (1322025, 1),
    (1032013, 1),
    (1302016, 1),
    (1072263, 1),
    (1032043, 1),
    (2000005, 1),
    (2000004, 5),
    (2001001, 5),
    (2001002, 5),
    (2020008, 20),
    (2020010, 20),
    (2030008, 20),
    (2030010, 20),
    (2030009, 20),
    (2022000, 50),
    (2001000, 50),
    (2022019, 50),
    (2020007, 100),
    (2020006, 100),
    (2020009, 100),
    (2000006, 100),
    (2040601, 1),
    (2040605, 1),
    (2040602, 1),
    (2041027, 1),
    (2041028, 1),
    (2041004, 1),
    (2041029, 1),
    (2041017, 1),
    (2041020, 1),
    (2040008, 1),
    (2040001, 1),
    (2040009, 1),
    (2040002, 1),
    (2040504, 1),
    (2040511, 1),
    (2040505, 1),
    (2040501, 1),
    (2040904, 1),
    (2040901, 1),
    (2040905, 1),
    (2040902, 1),
    (2040404, 1),
    (2040401, 1),
    (2040405, 1),
    (2040402, 1),
];

/// The Ludibrium Maze (Cosmic `LudiMazePQ`): three to six of a party, all
/// of level 51 to 70, in Ludibrium with their leader, go into their own
/// copy of the maze for 15 minutes, each on a room of the first 15 drawn
/// for them. Its rooms' portals lead from room to room, and only one leads
/// on to the goal room. There the leader hands Pietri (9103000) the
/// coupons, which wins it and takes the team to the clear room. A member
/// who dies lands on the way out, the maps' forcedReturn.
pub const LUDIMAZE: Event = Event {
    lobby: LUDIBRIUM,
    entrances: &[],
    maps: MAZE_ROOM..=MAZE_CLEAR,
    entry: MAZE_ROOM,
    entries: 15,
    exit: MAZE_EXIT,
    levels: 51..=70,
    spread: None,
    team: 3..=6,
    minutes: 15,
    exp_per_stage: &[0],
    stride: 1,
    stage_maps: &[MAZE_GOAL],
    talk_clears: &[],
    member_clears: &[],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &[MAZE_COUPON],
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: Some(MAZE_CLEAR),
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// The Chief's Residence in El Nath, where the party gathers; the room
/// where it protects Tylus; the room past it, where he thanks them; and
/// where everyone else goes.
pub const TYLUS_LOBBY: i32 = 211_000_001;
pub const TYLUS_ROOM: i32 = 921_100_300;
pub const TYLUS_THANKS: i32 = 921_100_301;
pub const TYLUS_EXIT: i32 = 211_040_100;
/// Tylus, the friendly mob in the room, who now and then drops his Trust
/// (4031495), which "Protecting Tylus" (6192) wants.
pub const TYLUS_MOB: i32 = 9_300_093;
pub const TYLUS_TRUST: i32 = 4_031_495;
pub const TYLUS_QUEST: i16 = 6192;
/// How often Tylus drops his Trust, in tens of seconds: Cosmic's
/// `dropItemPeriod`, which our Mob.nx lacks for him (the default in
/// `v83ref/lanes/QUESTIONS.md` a123).
const TYLUS_DROP_PERIOD: i32 = 10;

/// Protecting Tylus (Cosmic `ElnathPQ`): one to four of a party, all of
/// level 80 or more, in the Chief's Residence with their leader, go to
/// their own copy of the room for ten minutes, where kidnappers come for
/// Tylus and he drops his Trust now and then. Nobody wins it: its way out
/// leads on to Tylus's thanks with the Trust, and out without; the time
/// running out sends everyone out. Tylus's going loses nothing.
pub const TYLUS: Event = Event {
    lobby: TYLUS_LOBBY,
    entrances: &[],
    maps: TYLUS_ROOM..=TYLUS_ROOM,
    entry: TYLUS_ROOM,
    entries: 1,
    exit: TYLUS_EXIT,
    levels: 80..=u8::MAX,
    spread: None,
    team: 1..=4,
    minutes: 10,
    exp_per_stage: &[0],
    stride: 1,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &[],
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
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// The Deep Fairy Forest, where the party gathers; the Forest of Poison
/// Haze's entrance; its stages' maps, in order; and the prize map, which
/// every party shares and everyone who leaves lands on (Cosmic's
/// `exitMap`, and the maps' `returnMap`).
pub const ELLIN_LOBBY: i32 = 300_030_100;
pub const ELLIN_ENTRY: i32 = 930_000_000;
pub const ELLIN_STAGES: [i32; 6] = [
    930_000_100,
    930_000_200,
    930_000_300,
    930_000_400,
    930_000_500,
    930_000_600,
];
pub const ELLIN_EXIT: i32 = 930_000_800;
/// The Purification Marbles Ellin hands out on the forest's fourth stage,
/// so many at a time, to catch its Poison Flowers with; and the Monster
/// Marbles a catch makes, so many of which the leader brings her.
pub const PURIFICATION_MARBLE: i32 = 2_270_004;
pub const PURIFICATION_MARBLES_GIVEN: i16 = 10;
pub const MONSTER_MARBLE: i32 = 4_001_169;
pub const MONSTER_MARBLES_WANTED: i16 = 20;
/// The Purple Stone of Magic, which one of the fifth stage's boxes holds,
/// the leader shows the Sprite, and the sixth stage's altar takes; the
/// Poison Golem the altar brings out, and the last of the forms it dies
/// into, whose death wins the forest (Cosmic `EllinPQ.monsterKilled`).
pub const PURPLE_STONE: i32 = 4_001_163;
pub const ELLIN_GOLEM: i32 = 9_300_180;
pub const ELLIN_GOLEM_LAST: i32 = 9_300_182;
/// The altar's place on the sixth stage (Map.nx 930000600), where the
/// golem comes out (Cosmic `spawnMonster`, at the reactor).
pub const ELLIN_ALTAR_AT: (i16, i16) = (392, -73);
/// Diluted Poison, the Purple Stone of Magic, Monster Marbles and
/// Purification Marbles: of use only in the forest.
pub const ELLIN_ITEMS: [i32; 4] = [4_001_162, PURPLE_STONE, MONSTER_MARBLE, PURIFICATION_MARBLE];
/// The Golem's Marble the golem's last form may drop, which the prize
/// map's fountain takes for 52,000 EXP to everyone there; and the Altaire
/// Fragment its way home then gives, ten of which Ellin trades for
/// earrings (Cosmic `3008000`, `party6_stage800`, `2133000`).
pub const GOLEMS_MARBLE: i32 = 4_001_164;
pub const ELLIN_FOUNTAIN_EXP: i32 = 52_000;
pub const ALTAIRE_FRAGMENT: i32 = 4_001_198;

/// The Forest of Poison Haze (Cosmic `EllinPQ`): four to six of a party,
/// all of levels 44 to 55, in the Deep Fairy Forest with their leader, go
/// into their own copy of the forest for 30 minutes. From its entrance
/// they clear six stages in turn, the first by killing all its Poisoned
/// Lord Trees; the last, the Poison Golem's, wins it. No stage gives EXP.
/// The defaults are in `v83ref/lanes/QUESTIONS.md` a125.
pub const ELLIN: Event = Event {
    lobby: ELLIN_LOBBY,
    entrances: &[],
    maps: ELLIN_ENTRY..=ELLIN_EXIT,
    entry: ELLIN_ENTRY,
    entries: 1,
    exit: ELLIN_EXIT,
    levels: 44..=55,
    spread: None,
    team: 4..=6,
    minutes: 30,
    exp_per_stage: &[0; 6],
    stride: 1,
    stage_maps: &ELLIN_STAGES,
    talk_clears: &[],
    // The first by killing, the second by its spine, the maze by Ellin at
    // its end, the fourth by catching the last of its flowers, the fifth
    // by its box, the last by the golem's death.
    member_clears: &[1, 2, 3, 4, 5, 6],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &ELLIN_ITEMS,
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: None,
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// The stage cleared once the last mob on `map_id`, in an event's copy of
/// it, is caught, none being left to come back: the Forest of Poison
/// Haze's fourth, whose Poison Flowers grow again when killed (Cosmic
/// `EllinPQ.monsterKilled` without a killer).
pub fn cleared_by_catching(map_id: i32) -> Option<u8> {
    (map_id == ELLIN_STAGES[3]).then_some(4)
}

/// The stage cleared once `mob_id` dies on `map_id`, in an event's copy of
/// it: the Forest of Poison Haze's last, by its Super-Charged Poison Golem
/// (Cosmic `EllinPQ.monsterKilled`), each Dojo floor by its boss, and So
/// Gong's trial by So Gong.
pub fn cleared_by_boss(map_id: i32, mob_id: i32) -> Option<u8> {
    if dojo_boss(map_id) == Some(mob_id) {
        return dojo_floor(map_id);
    }
    if map_id == DOJO_TRIAL_MAP && mob_id == DOJO_TRIAL_MOB {
        return Some(1);
    }
    (map_id == ELLIN_STAGES[5] && mob_id == ELLIN_GOLEM_LAST).then_some(6)
}

/// The stage cleared once every mob on `map_id`, in an event's copy of
/// it, is dead, and every reactor `seals` names there set: the first of
/// the Forest of Poison Haze, the Pirate Ship's first, decks and doors,
/// the hall below each Magatia library, and Yulete's office (Cosmic
/// `EllinPQ`, `PiratePQ` and `MagatiaPQ_Z`'s `monsterKilled`).
pub fn cleared_by_killing(map_id: i32) -> Option<u8> {
    [
        (ELLIN_STAGES[0], 1),
        (PIRATE_ENTRY, 1),
        (PIRATE_STAGES[2], 3),
        (PIRATE_STAGES[3], 4),
        (PIRATE_STAGES[4], 5),
        (ZENUMIST_STAGES[1], 2),
        (ALCADNO_STAGES[1], 2),
        (ZENUMIST_STAGES[4], MAGATIA_OFFICE_STAGE),
        (ALCADNO_STAGES[4], MAGATIA_OFFICE_STAGE),
    ]
    .into_iter()
    .find_map(|(map, stage)| (map == map_id).then_some(stage))
}

/// The reactors on `map_id` that must all be set (past their first state)
/// before killing its last mob clears it, or the last of them set clears
/// it once no mob is left: the Pirate Ship's doors' seals (Cosmic
/// `2519000`, `PiratePQ.monsterKilled`).
pub fn seals(map_id: i32) -> &'static [&'static str] {
    if map_id == PIRATE_STAGES[4] {
        &crate::reactors::PIRATE_SEAL_NAMES
    } else {
        &[]
    }
}

/// The Lord Pirate the first member past the doors finds on the last
/// stage, by how many of the bellflower rooms' chests the team opened
/// (Cosmic `davy_next4`), and the point above the ground below which he
/// comes out.
pub fn lord_pirate(chests: u8) -> i32 {
    match chests {
        0 => 9_300_119,
        1 => 9_300_105,
        _ => 9_300_106,
    }
}
pub const LORD_PIRATE_AT: (i16, i16) = (777, 140);

/// Whether `mob_id` is a Lord Pirate, whose fall frees Wu Yang (Cosmic
/// `PiratePQ`'s `isLordPirate`, which names a fourth, 9300107, that no
/// script brings out).
pub fn is_lord_pirate(mob_id: i32) -> bool {
    matches!(mob_id, 9_300_105 | 9_300_106 | 9_300_107 | 9_300_119)
}

/// The Lord Pirate's Hats Wu Yang makes in the clear room, each for so
/// many feathers (`HPQ_REWARD`) and the one before, in turn (Cosmic
/// `2094001`). His third, 1003267, is not in v83's Character.wz, so the
/// ladder stops here (`v83ref/lanes/QUESTIONS.md` a132).
pub const PIRATE_HATS: [i32; 2] = [1_002_572, 1_002_573];
pub const PIRATE_HAT_FEATHERS: i16 = 20;

/// The EXP each member gets for beating the Lord Pirate, by how many of the
/// chests the team opened (Cosmic `PiratePQ`'s `clearPQ`).
pub fn pirate_win_exp(chests: u8) -> i32 {
    match chests {
        0 => 28_000,
        1 => 35_000,
        _ => 42_000,
    }
}

/// Over the Pirate Ship, in Herb Town, where the party gathers at Guon;
/// the ship's stages' maps, in order, the first its entry; the map the
/// team that beats the Lord Pirate goes to; and the way out, which every
/// map of the ship returns to (Map.nx `returnMap`, Cosmic `PiratePQ`'s
/// `exitMap`).
pub const PIRATE_LOBBY: i32 = 251_010_404;
pub const PIRATE_ENTRY: i32 = 925_100_000;
pub const PIRATE_STAGES: [i32; 6] = [
    PIRATE_ENTRY,
    925_100_100,
    925_100_200,
    925_100_300,
    925_100_400,
    925_100_500,
];
pub const PIRATE_CLEAR: i32 = 925_100_600;
pub const PIRATE_EXIT: i32 = 925_100_700;
/// The Old Metal Keys the fifth stage's doors take, and the three Marks of
/// the second, which the leader hands Guon in turn: of use only on the
/// ship.
pub const PIRATE_ITEMS: [i32; 4] = [4_001_117, 4_001_120, 4_001_121, 4_001_122];
/// The second stage's Marks, in the order the leader hands Guon
/// `PIRATE_MARKS_WANTED` of each, with the pirates that drop them: the
/// stage's only mobs until their Mark is in (Cosmic `PiratePQ`'s
/// `respawnStages`; String.nx: the Lord Pirate's Enraged Mr. Allis, Krus
/// and Captains).
pub const PIRATE_MARKS: [(i32, i32); 3] = [
    (4_001_120, 9_300_114),
    (4_001_121, 9_300_115),
    (4_001_122, 9_300_116),
];
pub const PIRATE_MARKS_WANTED: i16 = 20;

/// The only mob a copy of `map_id` made for an event brings out, at every
/// spawn point of it, kept filled: on the Pirate Ship's second stage, the
/// first Mark's pirates, until Guon swaps them (`EventCommand::HandInMarks`).
pub fn only_mobs(map_id: i32) -> Option<i32> {
    (map_id == PIRATE_STAGES[1]).then_some(PIRATE_MARKS[0].1)
}

/// The mobs a copy of `map_id` made for an event opens with, beyond its own
/// spawn points, as (mob, the point they come out on the ground below, how
/// many): each of the Pirate Ship's decks has none of its own, and opens
/// with five of the Lord Pirate's Krus (9300124) and five of his Captains
/// (9300125) at each end of each of its two floors; each of its bellflower
/// rooms with ten more of each of its two kinds of Bellflower (9300112,
/// 9300113), one kind at each end (Cosmic `PiratePQ`'s `setup`). A room of
/// Pharaoh Yeti's Tomb opens with its Pharaoh Jr. Yeti, at its spawn. None
/// come back.
pub fn opening_mobs(map_id: i32) -> &'static [(i32, (i16, i16), usize)] {
    const YETI: [(i32, (i16, i16), usize); 1] = [(PYRAMID_YETI, (0, 0), 1)];
    const HELL_YETI: [(i32, (i16, i16), usize); 1] = [(PYRAMID_HELL_YETI, (0, 0), 1)];
    let tomb = PYRAMID_TOMB_RUNS
        .iter()
        .position(|tomb| tomb.entry == map_id);
    match tomb {
        Some(3) => return &HELL_YETI,
        Some(_) => return &YETI,
        None => {}
    }
    const DECK: [(i32, (i16, i16), usize); 4] = [
        (9_300_124, (430, 75), 5),
        (9_300_125, (1600, 75), 5),
        (9_300_124, (430, 238), 5),
        (9_300_125, (1600, 238), 5),
    ];
    const BELLFLOWERS: [(i32, (i16, i16), usize); 2] =
        [(9_300_112, (0, 238), 10), (9_300_113, (1700, 238), 10)];
    match PIRATE_STAGES[2..4]
        .iter()
        .find(|&&deck| deck == map_id || deck + 1 == map_id)
    {
        Some(&deck) if deck == map_id => &DECK,
        Some(_) => &BELLFLOWERS,
        None => &[],
    }
}

/// The Pirate Ship (Cosmic `PiratePQ`): three to six of a party, all of
/// levels 55 to 100, Over the Pirate Ship with their leader, board their
/// own copy of the ship with 4 minutes on the clock, which starts over the
/// first time any of them reaches a later stage (`clock_on`). Six stages,
/// the last the Lord Pirate's; beating him takes the team to
/// `PIRATE_CLEAR`. The defaults are in `v83ref/lanes/QUESTIONS.md` a128.
pub const PIRATE: Event = Event {
    lobby: PIRATE_LOBBY,
    entrances: &[],
    maps: PIRATE_ENTRY..=PIRATE_EXIT,
    entry: PIRATE_ENTRY,
    entries: 1,
    exit: PIRATE_EXIT,
    levels: 55..=100,
    spread: None,
    team: 3..=6,
    minutes: 4,
    exp_per_stage: &[0; 6],
    stride: 1,
    stage_maps: &PIRATE_STAGES,
    talk_clears: &[],
    member_clears: &[1, 3, 4, 5],
    clear_portals: &[],
    gated: &[],
    puzzles: 1..=0,
    areas_taken: 0,
    items: &PIRATE_ITEMS,
    passes: 0,
    hand_ins: &[],
    gates: false,
    clear_map: Some(PIRATE_CLEAR),
    bonus: None,
    blessings: &[],
    squad: false,
    gate: &[],
    until_empty: false,
    survive: None,
    runs: 1,
};

/// Magatia's lobbies for Romeo and Juliet's party quest: Romeo's room on
/// the Zenumist side, Juliet's on the Alcadno side.
pub const ZENUMIST_LOBBY: i32 = 261_000_011;
pub const ALCADNO_LOBBY: i32 = 261_000_021;
/// The first of each side's copied maps (Map.nx 926100000 and 926110000);
/// the others keep the same offsets from it.
const ZENUMIST_ENTRY: i32 = 926_100_000;
const ALCADNO_ENTRY: i32 = 926_110_000;
/// How far each stage's map is from its side's first (Cosmic
/// `MagatiaPQ_Z`): the library, the hall below it, the beakers, the hub of
/// files, Yulete's room, the rope rooms' landing, and the boss's room.
const MAGATIA_STAGE_OFFSETS: [i32; 7] = [0, 1, 100, 200, 203, 300, 401];
/// The room both sides' maps let out on (its `forcedReturn`), whose portal
/// leads back to the side's lobby.
const MAGATIA_EXIT_OFFSET: i32 = 700;
pub const ZENUMIST_STAGES: [i32; 7] = magatia_stages(ZENUMIST_ENTRY);
pub const ALCADNO_STAGES: [i32; 7] = magatia_stages(ALCADNO_ENTRY);
/// The party quest's own items (Cosmic `setEventExclusives`), taken from
/// whoever leaves it.
pub const MAGATIA_ITEMS: [i32; 6] = [
    4_001_130, 4_001_131, 4_001_132, 4_001_133, 4_001_134, 4_001_135,
];

const fn magatia_stages(entry: i32) -> [i32; 7] {
    let mut maps = [0; 7];
    let mut i = 0;
    while i < maps.len() {
        maps[i] = entry + MAGATIA_STAGE_OFFSETS[i];
        i += 1;
    }
    maps
}

/// Romeo and Juliet's party quest from one side (Cosmic `MagatiaPQ_Z` and
/// `MagatiaPQ_A`, which differ only in their maps and NPCs): exactly four
/// of a party, all of levels 71 to 85, in the side's lobby with their
/// leader, go into their own copy of its maps for 45 minutes. With only
/// four, anyone leaving ends it for all. The stages' EXP is Cosmic's.
const fn magatia(lobby: i32, entry: i32, stages: &'static [i32; 7]) -> Event {
    Event {
        lobby,
        entrances: &[],
        maps: entry..=entry + MAGATIA_EXIT_OFFSET,
        entry,
        entries: 1,
        exit: entry + MAGATIA_EXIT_OFFSET,
        levels: 71..=85,
        spread: None,
        team: 4..=4,
        minutes: 45,
        exp_per_stage: &[0, 10_000, 20_000, 0, 20_000, 20_000, 0],
        stride: 1,
        stage_maps: stages,
        talk_clears: &[MAGATIA_FILES_STAGE],
        member_clears: &[
            1,
            2,
            3,
            MAGATIA_OFFICE_STAGE,
            MAGATIA_ROPES_STAGE,
            MAGATIA_BOSS_STAGE,
        ],
        clear_portals: &[],
        gated: &[],
        puzzles: 1..=0,
        areas_taken: 0,
        items: &MAGATIA_ITEMS,
        passes: 0,
        hand_ins: &[],
        gates: false,
        clear_map: None,
        bonus: None,
        blessings: &[],
        squad: false,
        gate: &[],
        until_empty: false,
        survive: None,
        runs: 1,
    }
}

/// Romeo and Juliet's party quest, by side.
pub const MAGATIA: [Event; 2] = [
    magatia(ZENUMIST_LOBBY, ZENUMIST_ENTRY, &ZENUMIST_STAGES),
    magatia(ALCADNO_LOBBY, ALCADNO_ENTRY, &ALCADNO_STAGES),
];

/// What one of the books in a Magatia library holds, each found once
/// (Cosmic `2112007`): mesos, EXP, the side's letter, or the switch that
/// clears the stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    Mesos,
    Exp,
    Letter,
    Pass,
}

/// How many books each library has (Map.nx), and the mesos or EXP one
/// gives (Cosmic's 500, there times the player's rate).
pub const MAGATIA_BOOKS: usize = 26;
pub const MAGATIA_BOOK_MESOS: i32 = 500;
pub const MAGATIA_BOOK_EXP: i32 = 500;

/// How many beakers each beaker room has to fill, and the stage that
/// clears (Cosmic `2618000`'s `statusStg3`).
pub const MAGATIA_BEAKERS: usize = 3;
pub const MAGATIA_BEAKER_STAGE: u8 = 3;

/// What the hub's doors to the rooms beside it take (9300148 drops it),
/// the two files those rooms' boxes hold, and the stage their hand-in
/// clears (Cosmic `2112006`'s `statusStg4`).
pub const MAGATIA_CARD_KEY: i32 = 4_001_133;
pub const MAGATIA_FILES: [i32; 2] = [4_001_134, 4_001_135];
pub const MAGATIA_FILES_STAGE: u8 = 4;

/// Yulete's office (`stages()[4]`): where he stands until he leaves
/// (Cosmic `MagatiaPQ_Z.afterSetup`'s `spawnNpc`), how near him anyone
/// in the office may be when he is talked to without his having seen
/// them coming (`2112000`'s `playersTooClose`), how long after the first
/// member arrives he leaves (`yuleteAction`, on the clock, not the event's
/// scale), and the mobs he leaves behind, five of each at each of three
/// spots. Killing them all clears the stage.
pub const YULETE_AT: (i16, i16) = (252, 243);
pub const YULETE_SEES: i32 = 310;
pub const YULETE_STAYS: Duration = Duration::from_secs(10);
pub const YULETE_GUARDS: [i32; 2] = [9_300_143, 9_300_144];
pub const YULETE_GUARD_SPOTS: [(i16, i16); 3] = [(-455, 135), (0, 135), (360, 135)];
pub const MAGATIA_OFFICE_STAGE: u8 = 5;

/// The rope rooms beside the landing, each taken by one member (Cosmic
/// `rnj4_r1`-`rnj4_r4`'s `statusStg5`), their rows of four ropes, and the
/// stage the first to reach a room's top clears (`rnj5_rp`'s
/// `statusStg6`), which shows nothing but gives its EXP.
pub const MAGATIA_ROPE_ROOMS: u8 = 4;
pub const MAGATIA_ROPE_ROWS: usize = 10;
pub const MAGATIA_ROPES_STAGE: u8 = 6;

/// The boss's room (`stages()[6]`, Cosmic `MagatiaPQ_Z`): Yulete waits
/// there from the first arrival, and talked to brings out his
/// Frankenroid where it stands; its death wins the run (the last stage,
/// which gives no EXP of its own but the run's, `magatia_win_exp`), and
/// the side's Romeo or Juliet comes out where they stand. In the rooms
/// after it, Yulete's traces and the one who thanks the party wait where
/// they stand.
pub const MAGATIA_BOSS_STAGE: u8 = 7;
pub const LAB_YULETE_AT: (i16, i16) = (200, 100);
pub const FRANKENROID_AT: (i16, i16) = (250, 100);
pub const MAGATIA_SAVED_AT: (i16, i16) = (-370, -150);
pub const MAGATIA_THANKS_AT: (i16, i16) = (200, 100);
/// The EXP each member gets for the win (Cosmic `monsterKilled`'s
/// `giveEventPlayersExp`): less if the escort fainted, and more for the
/// mighty Frankenroid that a caught Yulete brings out.
pub const MAGATIA_ESCORT_FAILED_EXP: i32 = 90_000;
pub const MAGATIA_FRANKENROID_EXP: i32 = 105_000;
pub const MAGATIA_MIGHTY_EXP: i32 = 140_000;
/// What the one who thanks the party gives each member: one of these
/// items, and how many, all equally likely (Cosmic `MagatiaPQ_Z`'s
/// `setEventRewards`).
pub const MAGATIA_REWARDS: [(i32, i16); 23] = [
    (2000003, 100),
    (2000002, 100),
    (2000004, 20),
    (2000005, 10),
    (2022003, 50),
    (1032016, 1),
    (1032015, 1),
    (1032014, 1),
    (2041212, 1),
    (2041020, 1),
    (2040502, 1),
    (2041016, 1),
    (2044701, 1),
    (2040301, 1),
    (2043201, 1),
    (2040501, 1),
    (2040704, 1),
    (2044001, 1),
    (2043701, 1),
    (2040803, 1),
    (1102026, 1),
    (1102028, 1),
    (1102029, 1),
];

/// The run's EXP for beating the Frankenroid, by whether the escort
/// fainted and whether Yulete was caught, which made it the mighty one.
/// Cosmic's Alcadno script pays the mighty one's either way; both sides
/// pay as its Zenumist script does.
pub fn magatia_win_exp(escort_failed: bool, caught: bool) -> i32 {
    match (escort_failed, caught) {
        (true, _) => MAGATIA_ESCORT_FAILED_EXP,
        (false, false) => MAGATIA_FRANKENROID_EXP,
        (false, true) => MAGATIA_MIGHTY_EXP,
    }
}

/// Whether `mob_id` is either side's Frankenroid, of either kind.
pub fn is_frankenroid(mob_id: i32) -> bool {
    MagatiaSide::ALL.into_iter().any(|side| {
        [false, true]
            .map(|caught| side.frankenroid(caught))
            .contains(&mob_id)
    })
}

/// What Yulete says in the boss's room, as text key (Cosmic `2112000`):
/// once the run is won, that he is beaten; once he brought out his
/// Frankenroid, to behold it; otherwise he brings it out, the mighty one
/// if he was caught off guard in his office.
pub fn yulete_in_lab(won: bool, called: bool, caught: bool) -> &'static str {
    use game_data::server_content::keys;
    match (won, called, caught) {
        (true, ..) => keys::MAGATIA_YULETE_BEATEN,
        (_, true, _) => keys::MAGATIA_YULETE_BEHOLD,
        (_, _, false) => keys::MAGATIA_YULETE_PRESENTS,
        (_, _, true) => keys::MAGATIA_YULETE_MIGHTY,
    }
}

/// What the whole run hears as Yulete brings out his Frankenroid, as
/// text key (Cosmic `2112000`'s `dropMessage(5, ..)`).
pub fn frankenroid_called(caught: bool) -> &'static str {
    use game_data::server_content::keys;
    match caught {
        true => keys::MAGATIA_FRANKENROID_MIGHTY,
        false => keys::MAGATIA_FRANKENROID,
    }
}

/// What everyone in the boss's room is told as the escort `mob_id`
/// faints, as text key, if it is one (Cosmic
/// `MobDamageMobFriendlyHandler`).
pub fn magatia_fainted(mob_id: i32) -> Option<&'static str> {
    use game_data::server_content::keys;
    match MagatiaSide::ALL
        .into_iter()
        .find(|side| side.escort() == mob_id)?
    {
        MagatiaSide::Zenumist => Some(keys::MAGATIA_JULIET_FAINTED),
        MagatiaSide::Alcadno => Some(keys::MAGATIA_ROMEO_FAINTED),
    }
}

/// What Yulete in his office makes of the party (Cosmic `yuleteTalked`):
/// no one has talked to him yet, someone caught him off guard, or he is
/// leaving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Yulete {
    Unmet,
    Caught,
    Leaving,
}

/// What Yulete says to a player who talks to him, as text key, and what
/// he makes of the party after (Cosmic `2112000`): once leaving, that he
/// is; with anyone in the office within `YULETE_SEES` of him, or before
/// Romeo or Juliet read the letter, that he saw the party coming, and he
/// is leaving; otherwise he is caught off guard.
pub fn yulete_talk(mood: Yulete, too_close: bool, shocked: bool) -> (&'static str, Yulete) {
    use game_data::server_content::keys;
    match mood {
        Yulete::Leaving => (keys::MAGATIA_YULETE_LEAVING, Yulete::Leaving),
        _ if too_close => (keys::MAGATIA_YULETE_WATCHED, Yulete::Leaving),
        _ if !shocked => (keys::MAGATIA_YULETE_SNEAKY, Yulete::Leaving),
        _ => (keys::MAGATIA_YULETE_CAUGHT, Yulete::Caught),
    }
}

/// What Yulete tells the whole party as he leaves his office, as text
/// key, by whether he was `caught` off guard (Cosmic `yuleteAction`'s
/// `yuletePassed`, which later decides the Frankenroid the party meets).
pub fn yulete_leaves(caught: bool) -> &'static str {
    use game_data::server_content::keys;
    match caught {
        true => keys::MAGATIA_YULETE_DISGUSTED,
        false => keys::MAGATIA_YULETE_FOOLED,
    }
}

/// What each of a library's books holds, in the order of the map's books,
/// drawn for each run (Cosmic `MagatiaPQ_Z.afterSetup`): seven of mesos,
/// seven of EXP, the letter, the switch, and ten with nothing.
pub fn draw_books(rng: &mut impl rand::Rng) -> [Option<Find>; MAGATIA_BOOKS] {
    use rand::seq::SliceRandom;
    let mut books = [None; MAGATIA_BOOKS];
    let finds = [
        (Find::Mesos, 7),
        (Find::Exp, 7),
        (Find::Letter, 1),
        (Find::Pass, 1),
    ];
    let mut slots = books.iter_mut();
    for (find, count) in finds {
        for book in slots.by_ref().take(count) {
            *book = Some(find);
        }
    }
    books.shuffle(rng);
    books
}

/// The Mu Lung Dojo's entrance, where the notice board sends a challenger
/// and its `dojang_exit` portal leads out (Cosmic's
/// `MapId.MU_LUNG_DOJO_EXIT`, which is not `DOJO_EXIT`).
pub const DOJO_ENTRANCE: i32 = 925020000;
/// The hall where So Gong (2091005) starts a run.
pub const DOJO_HALL: i32 = 925020001;
/// Where a run's time running out leaves its challenger.
pub const DOJO_EXIT: i32 = 925020002;
/// The rooftop a challenger who beats the 38th floor goes to.
pub const DOJO_ROOFTOP: i32 = 925020003;
/// The first floor; floor n is `DOJO_FIRST + (n - 1) * 100`.
pub const DOJO_FIRST: i32 = 925020100;
/// How many floors there are.
pub const DOJO_FLOORS: u8 = 38;
/// Mu Lung's Dojo, where the notice board (2091006) stands.
pub const DOJO_BOARD_MAP: i32 = 250000100;
/// The board's portal on the entrance a challenger arrives by (`out00`).
pub const DOJO_BOARD_PORTAL: u8 = 4;
/// The floor-n boss is `DOJO_BOSS_BASE` plus its round (`dojo_round`).
pub const DOJO_BOSS_BASE: i32 = 9300183;
/// Where a floor's boss may come out, one drawn at random, on the ground
/// below (Cosmic `dojang_1st`).
pub const DOJO_BOSS_AT: [(i16, i16); 3] = [(140, 0), (190, 7), (187, 7)];
/// The `Effect/Summon.img` entry a floor's boss comes out with.
pub const DOJO_BOSS_EFFECT: u8 = 15;
/// The way up a floor's upper tier: `dojang_up`'s portal (Cosmic
/// `goDojoUp`).
pub const DOJO_UP_PORTAL: u8 = 6;
/// The reactor that is each fighting floor's door to its way on.
pub const DOJO_DOOR: &str = "door";
/// The weather whose message bar taunts a challenger on the entrance
/// (Cosmic `dojang_Msg`).
pub const DOJO_TAUNT_WEATHER: i32 = 5120024;
/// How long the taunt stays (Cosmic `startMapEffect`'s 30000 ms).
pub const DOJO_TAUNT_TIME: Duration = Duration::from_secs(30);

/// Every floor: whoever kills a floor's boss clears it.
const DOJO_CLEARS: [u8; DOJO_FLOORS as usize] = {
    let mut floors = [0; DOJO_FLOORS as usize];
    let mut i = 0;
    while i < floors.len() {
        floors[i] = i as u8 + 1;
        i += 1;
    }
    floors
};

/// The Mu Lung Dojo, taken alone (Cosmic `2091005`, `dojang_*`): 38
/// floors, one boss each but on every sixth, a rest floor. 15 may run at
/// once on a channel, each in its own copies.
pub const DOJO: Event = Event {
    lobby: DOJO_HALL,
    entrances: &[],
    maps: DOJO_FIRST..=DOJO_FIRST + (DOJO_FLOORS as i32 - 1) * 100,
    entry: DOJO_FIRST,
    entries: 1,
    exit: DOJO_EXIT,
    levels: 25..=u8::MAX,
    spread: None,
    team: 1..=1,
    minutes: 5,
    exp_per_stage: &[0; DOJO_FLOORS as usize],
    stride: 100,
    stage_maps: &[],
    talk_clears: &[],
    member_clears: &DOJO_CLEARS,
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
    gate: &[],
    until_empty: true,
    survive: None,
    runs: 15,
};

/// The Monster Carnival's rooms, 1 to 6 (Cosmic `2042000`,
/// `MonsterCarnival`): a party goes from the Carnival Field straight to
/// the room's battle field, which lasts as long as its `timeDefault` says
/// (`crate::carnival::battle_length`), and then to the exit room. Rooms 1
/// to 4 take two to four, 5 and 6 three to six.
pub const CARNIVAL: [Event; CARNIVAL_ROOMS as usize] = [
    carnival(1),
    carnival(2),
    carnival(3),
    carnival(4),
    carnival(5),
    carnival(6),
];

const fn carnival(room: u8) -> Event {
    let waiting = crate::carnival::room_map(room);
    Event {
        lobby: CARNIVAL_LOBBY,
        maps: waiting..=waiting + 4,
        entry: waiting,
        exit: crate::carnival::CARNIVAL_EXIT,
        levels: crate::carnival::CARNIVAL_LEVELS,
        team: if room <= 4 { 2..=4 } else { 3..=6 },
        minutes: 0,
        exp_per_stage: &[0],
        member_clears: &[],
        runs: 1,
        ..DOJO
    }
}

/// So Gong's trial room (Cosmic's first of five, 925020010-14).
pub const DOJO_TRIAL_MAP: i32 = 925020010;
/// So Gong himself, as the trial's monster; beaten, he lets out the
/// room's `timeMob` and runs away.
pub const DOJO_TRIAL_MOB: i32 = 9300269;

/// So Gong's trial (Cosmic `2091005`, `dojang_tuto`): one room, taken
/// alone, with no time limit, won by beating So Gong. Its way out up the
/// ladder then takes the challenger back to the hall, done with it. Five
/// may run at once on a channel, as Cosmic has five rooms.
pub const DOJO_TRIAL: Event = Event {
    lobby: DOJO_HALL,
    entrances: &[],
    maps: DOJO_TRIAL_MAP..=DOJO_TRIAL_MAP,
    entry: DOJO_TRIAL_MAP,
    entries: 1,
    exit: DOJO_EXIT,
    levels: 25..=u8::MAX,
    spread: None,
    team: 1..=1,
    minutes: 0,
    exp_per_stage: &[0],
    stride: 100,
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
    gate: &[],
    until_empty: true,
    survive: None,
    runs: 5,
};

/// The party Dojo's first floor; floor n is `DOJO_PARTY_FIRST + (n - 1) *
/// 100`, each a copy of the solo floor (`info/link`).
pub const DOJO_PARTY_FIRST: i32 = 925030100;

/// The Mu Lung Dojo taken by a party (Cosmic `2091005`'s second choice):
/// the leader takes in those of the party in the hall, at levels within
/// 30 of each other, to the same 38 floors, with the same clock. Only its
/// leader may take those on a rest floor on, and going on to one takes
/// everyone on the five floors before it there (`EventCommand::DojoRest`).
/// Five may run at once on a channel (Cosmic's party slots).
pub const DOJO_PARTY: Event = Event {
    maps: DOJO_PARTY_FIRST..=DOJO_PARTY_FIRST + (DOJO_FLOORS as i32 - 1) * 100,
    entry: DOJO_PARTY_FIRST,
    team: 1..=6,
    spread: Some(30),
    runs: 5,
    ..DOJO
};

/// Nett's Pyramid's lobby, where Duarte (2103013) stands.
pub const PYRAMID_LOBBY: i32 = 926010000;
/// The easy mode's result map (field type 24), which its runs end on;
/// mode m's is `PYRAMID_RESULT + m`, for runs alone or with a party.
pub const PYRAMID_RESULT: i32 = 926010001;
/// The easy mode's room in the Shades, where its result map leads; mode
/// m's is `PYRAMID_SHADES + m`.
pub const PYRAMID_SHADES: i32 = 926020001;
/// How many stages a run has.
pub const PYRAMID_STAGES: u8 = 5;
/// How long each stage after the first lasts (Cosmic `Pyramid.timer`'s
/// 180 s); the first's 120 are the event's `minutes`.
pub const PYRAMID_STAGE_TIME: Duration = Duration::from_secs(180);
/// The levels below which no mode is open, and above which only Hell is
/// (Cosmic `2103013`).
pub const PYRAMID_LEVELS: RangeInclusive<u8> = 40..=60;
/// The mode icons Duarte offers, easy's first (Cosmic `2103013`).
pub const PYRAMID_MODE_ICON: i32 = 3994115;

/// A run of Nett's Pyramid in `mode`, alone or with a `party` (Cosmic
/// `NPCConversationManager.createPyramid`, `Pyramid`): five stages, on
/// `926010000 + party * 10000 + mode * 1000 + stage * 100`, each played
/// to its clock's end, the first for 2 minutes and the rest for 3, then
/// the mode's result map. A party needs two or more in the lobby. Five
/// runs alone and ten with a party may go at once on a channel in each
/// mode, as the stage maps have that many copies (Map.nx).
const fn pyramid(party: bool, mode: PyramidMode) -> Event {
    let base = PYRAMID_LOBBY + if party { 10000 } else { 0 } + mode.index() * 1000;
    Event {
        lobby: PYRAMID_LOBBY,
        maps: base + 100..=base + PYRAMID_STAGES as i32 * 100,
        entry: base + 100,
        exit: PYRAMID_RESULT + mode.index(),
        levels: match mode {
            PyramidMode::Hell => *PYRAMID_LEVELS.start()..=u8::MAX,
            _ => PYRAMID_LEVELS,
        },
        team: if party { 2..=6 } else { 1..=1 },
        minutes: 2,
        exp_per_stage: &[0; PYRAMID_STAGES as usize],
        member_clears: &[],
        runs: if party { 10 } else { 5 },
        ..DOJO
    }
}

/// Nett's Pyramid taken alone, by mode.
pub const PYRAMID: [Event; 4] = [
    pyramid(false, PyramidMode::Easy),
    pyramid(false, PyramidMode::Normal),
    pyramid(false, PyramidMode::Hard),
    pyramid(false, PyramidMode::Hell),
];

/// Nett's Pyramid taken by a party, by mode.
pub const PYRAMID_PARTY: [Event; 4] = [
    pyramid(true, PyramidMode::Easy),
    pyramid(true, PyramidMode::Normal),
    pyramid(true, PyramidMode::Hard),
    pyramid(true, PyramidMode::Hell),
];

/// The easy mode's first room of Pharaoh Yeti's Tomb; mode m's is
/// `PYRAMID_TOMB + 20 * m`, the first of its 20 (Map.nx: 926010010 to 89,
/// two bodies, the last 20 Hell's).
pub const PYRAMID_TOMB: i32 = 926010010;
/// The easy mode's gem for the Tomb; mode m's is `PYRAMID_GEM + m`
/// (Sapphire to Topaz, in Cosmic `2103013`'s order).
pub const PYRAMID_GEM: i32 = 4001322;
/// Pharaoh Jr. Yeti (9700019, and 9700029 for Hell), alone in the Tomb.
pub const PYRAMID_YETI: i32 = 9700019;
pub const PYRAMID_HELL_YETI: i32 = 9700029;

/// Pharaoh Yeti's Tomb in `mode`, taken alone from level 40 with the
/// mode's gem in the lobby, or from `shades` after a run through all five
/// stages: one room, with no time limit, left by its `out00` to the lobby.
/// Twenty may go at once on a channel in each mode, as it has 20 rooms.
const fn pyramid_tomb(mode: PyramidMode, shades: &'static [i32]) -> Event {
    let room = PYRAMID_TOMB + mode.index() * 20;
    Event {
        lobby: PYRAMID_LOBBY,
        entrances: shades,
        maps: room..=room,
        entry: room,
        exit: PYRAMID_LOBBY,
        levels: *PYRAMID_LEVELS.start()..=u8::MAX,
        team: 1..=1,
        minutes: 0,
        exp_per_stage: &[],
        member_clears: &[],
        runs: 20,
        ..DOJO
    }
}

/// Pharaoh Yeti's Tomb, by mode.
pub const PYRAMID_TOMB_RUNS: [Event; 4] = [
    pyramid_tomb(PyramidMode::Easy, &[PYRAMID_SHADES]),
    pyramid_tomb(PyramidMode::Normal, &[PYRAMID_SHADES + 1]),
    pyramid_tomb(PyramidMode::Hard, &[PYRAMID_SHADES + 2]),
    pyramid_tomb(PyramidMode::Hell, &[PYRAMID_SHADES + 3]),
];

/// The room before Tristan's Tomb, 910520000, where the unknown Jr. Balrog
/// waits (Map.nx `life`, spawned once).
pub const TRISTAN_ROOM: i32 = 910520000;
/// The Sleepywood dungeon's map where Mu Young (1061014) stands, and the way
/// back from the room (its `forcedReturn` and its `out00`).
pub const MU_YOUNG_MAP: i32 = 105100100;
/// The unknown Jr. Balrog, whose fall brings out Tristan's Spirit.
pub const TRISTAN_BALROG: i32 = 9_300_326;
/// Tristan's Spirit (1061015), where 2238 ends, and the point above the
/// ground below which it comes out (Cosmic `BalrogQuest.monsterKilled`).
pub const TRISTANS_SPIRIT: i32 = 1_061_015;
pub const TRISTANS_SPIRIT_AT: (i16, i16) = (0, 115);

/// To Tristan's Tomb (2245): Mu Young's start script takes a player on 2238
/// alone into their own copy of the room for ten minutes, one at a time on
/// a channel (Cosmic `BalrogQuest`, whose script turns a second player
/// away). It is never won; it ends when they leave by `out00` or the time
/// runs out.
pub const TRISTAN: Event = Event {
    lobby: MU_YOUNG_MAP,
    entrances: &[],
    maps: TRISTAN_ROOM..=TRISTAN_ROOM,
    entry: TRISTAN_ROOM,
    exit: MU_YOUNG_MAP,
    levels: 50..=u8::MAX,
    team: 1..=1,
    minutes: 10,
    exp_per_stage: &[],
    member_clears: &[],
    runs: 1,
    ..DOJO
};

/// Eos Tower's 100th floor, 221024400, where Olson the Toy Soldier keeps the
/// way into the dollhouse room.
pub const EOS_TOWER_100: i32 = 221024400;
/// Its portal `q000` (id 4), by Olson, where the room lets out.
pub const EOS_TOWER_100_RETURN: &str = "q000";
pub const EOS_TOWER_100_RETURN_ID: u8 = 4;
/// The dollhouse room, 922000010, where one of the dollhouses is the monster
/// that took the pendulum (Map.nx: nine reactors 2200000 and one 2202000).
pub const DOLLHOUSE_ROOM: i32 = 922000010;
/// Dollhouse (3230), which wants the pendulum back.
pub const DOLLHOUSE_QUEST: i16 = 3230;
/// The pendulum the odd dollhouse drops (the pack's reactor drops for
/// 2202000), which Mark the Toy Soldier takes.
pub const DOLLHOUSE_PENDULUM: i32 = 4_031_145;

/// The dollhouse room: Olson takes a player on Dollhouse alone into their
/// own copy for ten minutes, one at a time on a channel (Server.NX
/// `FieldSet.img/Ludi023`: `timeOut` 600, `shuffle` 1; Cosmic `DollHouse`).
/// The dollhouses change places in each copy (`reactors::shuffled`). It is
/// never won; it ends when Mark sends them back, a wrong dollhouse throws
/// them out, or the time runs out. The pendulum stays behind.
pub const DOLLHOUSE: Event = Event {
    lobby: EOS_TOWER_100,
    entrances: &[],
    maps: DOLLHOUSE_ROOM..=DOLLHOUSE_ROOM,
    entry: DOLLHOUSE_ROOM,
    exit: EOS_TOWER_100,
    levels: 25..=u8::MAX,
    team: 1..=1,
    minutes: 10,
    exp_per_stage: &[],
    member_clears: &[],
    items: &[DOLLHOUSE_PENDULUM],
    runs: 1,
    ..DOJO
};

/// Ludibrium's Toy Factory <Process 1> Zone 1, 220020000, where Cheng the
/// Assistant keeps the way into Sector 4.
pub const TOY_FACTORY: i32 = 220020000;
/// The Toy Factory's Sector 4, 922000000, where the Machine Parts are hidden
/// (Map.nx: ten reactors 2202001 and two look-alikes 2208000 that hold
/// nothing), and Cheng waits by its far end.
pub const PARTS_ROOM: i32 = 922000000;
/// The Secret Passage, 922000009, where Sector 4 lets out (its
/// `forcedReturn`); its `out00` (`ludi021`) leads back to the factory.
pub const PARTS_PASSAGE: i32 = 922000009;
/// The Toy Factory <Apparatus Room>, 220020600, where the Secret Passage's
/// `out00` leads.
pub const APPARATUS_ROOM: i32 = 220020600;
/// The Missing Mechanical Parts (3239), which wants ten Machine Parts.
pub const PARTS_QUEST: i16 = 3239;
/// The Machine Parts the parts boxes drop (the pack's reactor drops for
/// 2202001), ten of which Cheng takes.
pub const MACHINE_PART: i32 = 4_031_092;

/// Sector 4: Cheng takes a player on The Missing Mechanical Parts alone into
/// their own copy for twenty minutes, one at a time on a channel (Server.NX
/// `FieldSet.img/Ludi020`: `timeOut` 1200, `shuffle` 1; Cosmic `q3239`).
/// The boxes change places in each copy (`reactors::shuffled`). It is never
/// won; it ends when Cheng sends the player out, or the time runs out, to
/// the Secret Passage. The parts stay behind.
pub const MECHANICAL_PARTS: Event = Event {
    lobby: TOY_FACTORY,
    entrances: &[],
    maps: PARTS_ROOM..=PARTS_ROOM,
    entry: PARTS_ROOM,
    exit: PARTS_PASSAGE,
    levels: 30..=u8::MAX,
    team: 1..=1,
    minutes: 20,
    exp_per_stage: &[],
    member_clears: &[],
    items: &[MACHINE_PART],
    runs: 1,
    ..DOJO
};

/// The Mushroom Castle's East Castle Tower, 106021400: the way up to King
/// Pepe's room and the Wedding Hall (`TD_MC_enterboss1`), and back.
pub const EAST_CASTLE_TOWER: i32 = 106021400;
/// King Pepe's room, 106021500, where a Yeti brother rides out with him.
pub const KING_PEPE_ROOM: i32 = 106021500;
/// Stop the Wedding (2330), which wants one of each Yeti brother, King
/// Pepe on his back (Quest.nx `Check.img/2330/1/mob`, in this order).
pub const STOP_THE_WEDDING: i16 = 2330;
pub const YETI_BROTHERS: [i32; 3] = [3_300_005, 3_300_006, 3_300_007];
/// The point above the ground below which a brother comes out (Cosmic
/// `pepeking_effect`).
pub const YETI_BROTHER_AT: (i16, i16) = (-28, -67);
/// The key to the Wedding Hall, which King Pepe drops once all three are
/// beaten, and which Where's Violetta? (2332) starts with.
pub const WEDDING_HALL_KEY: i32 = 4_032_388;

/// King Pepe's room: the tower's portal takes a player on Stop the Wedding
/// alone into their own copy for twenty minutes, one at a time on a channel
/// (Cosmic `KingPepeAndYetis`, one lobby). Each copy opens with one of the
/// brothers, drawn at random (Cosmic `pepeking_effect`). It is never won;
/// it ends when they leave by `out01` or the time runs out.
pub const KING_PEPE: Event = Event {
    lobby: EAST_CASTLE_TOWER,
    entrances: &[],
    maps: KING_PEPE_ROOM..=KING_PEPE_ROOM,
    entry: KING_PEPE_ROOM,
    exit: EAST_CASTLE_TOWER,
    levels: 30..=u8::MAX,
    team: 1..=1,
    minutes: 20,
    exp_per_stage: &[],
    member_clears: &[],
    runs: 1,
    ..DOJO
};

/// The way to the Wedding Hall, 106021402, whose `TD_MC_enterboss2` opens
/// the hall to one holding the key.
pub const WEDDING_HALL_DOOR: i32 = 106021402;
/// The Wedding Hall, 106021600, where Violetta waits.
pub const WEDDING_HALL: i32 = 106021600;
/// Where's Violetta? (2332), which the key starts, and The Story of
/// Betrayal (2333), which wants the Prime Minister beaten.
pub const WHERES_VIOLETTA: i16 = 2332;
pub const STORY_OF_BETRAYAL: i16 = 2333;
pub const PRIME_MINISTER: i32 = 3_300_008;
/// The point above the ground below which he comes out (Cosmic
/// `MK_PrimeMinister`).
pub const PRIME_MINISTER_AT: (i16, i16) = (292, 143);

/// The Wedding Hall: its door takes a player on 2332 or 2333 alone into
/// their own copy for ten minutes, one at a time on a channel (Cosmic
/// `MK_PrimeMinister`, one lobby; the quest says "Must be alone"). It is
/// never won; it ends when they leave by `left00` or the time runs out.
/// The Prime Minister comes out when Violetta starts 2333, or on entering
/// with it started and him not yet beaten (`findvioleta`).
pub const WEDDING_HALL_EVENT: Event = Event {
    lobby: WEDDING_HALL_DOOR,
    entrances: &[],
    maps: WEDDING_HALL..=WEDDING_HALL,
    entry: WEDDING_HALL,
    exit: WEDDING_HALL_DOOR,
    levels: 30..=u8::MAX,
    team: 1..=1,
    minutes: 10,
    exp_per_stage: &[],
    member_clears: &[],
    runs: 1,
    ..DOJO
};

/// Kerning Square's 7th floor, 103040400, where June (1052125) keeps the
/// way up to the Spirit of Rock's floors, and where they let out (their
/// `forcedReturn`).
pub const KERNING_SQUARE_7F: i32 = 103040400;
/// June, by the way up.
pub const JUNE: i32 = 1_052_125;
/// Kerning Square's 6th floor below it, where its `goDown` leads (to
/// `fromUp`, portal 1).
pub const KERNING_SQUARE_6F: i32 = 103040300;
/// The Spirit of Rock's floors: the Cheap Amplifiers' (410), the Fancy
/// Amplifiers' (420) and the Spirit of Rock's own stage (430).
pub const ROCK_SPIRIT_FLOOR: i32 = 103040410;
pub const ROCK_SPIRIT_STAGE: i32 = 103040430;
/// The VIP Zone's floors (440 to 460), the same floors with other mobs.
pub const ROCK_SPIRIT_VIP_FLOOR: i32 = 103040440;
pub const ROCK_SPIRIT_VIP_STAGE: i32 = 103040460;

/// The Spirit of Rock's floors: June takes one helping Blake (2286-2288)
/// up alone for an hour, eight at once on a channel (Cosmic `RockSpirit`).
/// It is never won; it ends when they leave the floors or the time runs
/// out.
pub const ROCK_SPIRIT: Event = Event {
    lobby: KERNING_SQUARE_7F,
    entrances: &[],
    maps: ROCK_SPIRIT_FLOOR..=ROCK_SPIRIT_STAGE,
    entry: ROCK_SPIRIT_FLOOR,
    exit: KERNING_SQUARE_7F,
    levels: 1..=u8::MAX,
    team: 1..=1,
    minutes: 60,
    exp_per_stage: &[],
    member_clears: &[],
    runs: 8,
    ..DOJO
};

/// The VIP Zone's floors: June takes one who hands in 10 VIP Invitations
/// (2291) up alone for half an hour, eight at once on a channel (Cosmic
/// `RockSpiritVIP`); the quest itself asks for levels 35 to 50.
pub const ROCK_SPIRIT_VIP: Event = Event {
    maps: ROCK_SPIRIT_VIP_FLOOR..=ROCK_SPIRIT_VIP_STAGE,
    entry: ROCK_SPIRIT_VIP_FLOOR,
    minutes: 30,
    ..ROCK_SPIRIT
};

/// Every run of Nett's Pyramid Duarte offers, alone first, then with a
/// party, each by mode.
pub fn pyramid_kinds() -> impl Iterator<Item = EventKind> {
    let alone = PyramidMode::ALL.map(EventKind::Pyramid);
    let party = PyramidMode::ALL.map(EventKind::PyramidParty);
    alone.into_iter().chain(party)
}

/// The Pyramid run and stage played on `map_id`, if it is one of its
/// stage maps.
pub fn pyramid_stage(map_id: i32) -> Option<(EventKind, u8)> {
    pyramid_kinds().find_map(|kind| Some((kind, kind.event().stage_on(map_id)?)))
}

/// Whether Dojo floor `floor` is a rest floor, with no boss: every sixth.
pub fn rest_floor(floor: u8) -> bool {
    floor % 6 == 0
}

/// The Dojo floor `map_id` is, alone or with a party.
pub fn dojo_floor(map_id: i32) -> Option<u8> {
    DOJO.stage_on(map_id)
        .or_else(|| DOJO_PARTY.stage_on(map_id))
}

/// Which boss floor `floor` is, counting only those (Cosmic
/// `realstage`): the number it shows on arrival.
pub fn dojo_round(floor: u8) -> u8 {
    floor - floor / 6
}

/// The boss that comes out the first time its challenger reaches
/// `map_id`, if it is a Dojo boss floor.
pub fn dojo_boss(map_id: i32) -> Option<i32> {
    let floor = dojo_floor(map_id)?;
    (!rest_floor(floor)).then(|| DOJO_BOSS_BASE + i32::from(dojo_round(floor)))
}

/// What the first time any member reaches one of an event's maps does to
/// its clock (`clock_on`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockOn {
    /// It starts over with this many minutes.
    Restart(u64),
    /// It stops, with no time limit until it starts over again.
    Stop,
}

/// What an event's clock does the first time any member reaches `map_id`,
/// in its copy:
/// - the Pirate Ship's second to fifth stages give 6 minutes, the Lord
///   Pirate's 8 (Cosmic `PiratePQ.changedMapInside`);
/// - the Dojo's first floor after each rest floor starts a section of six,
///   with `(floor > 36 ? 15 : floor / 6 + 5)` minutes, and a rest floor
///   stops the clock (Cosmic `Channel.startDojoSchedule`, which a rest
///   floor's `dojang_1st` dismisses). Its first section's 5 are the
///   event's own `minutes`.
pub fn clock_on(map_id: i32) -> Option<ClockOn> {
    if let Some(floor) = dojo_floor(map_id) {
        return match floor {
            _ if rest_floor(floor) => Some(ClockOn::Stop),
            _ if floor > 1 && (floor - 1) % 6 == 0 => {
                let minutes = if floor > 36 { 15 } else { floor / 6 + 5 };
                Some(ClockOn::Restart(u64::from(minutes)))
            }
            _ => None,
        };
    }
    match PIRATE_STAGES[1..].iter().position(|&map| map == map_id)? {
        4 => Some(ClockOn::Restart(8)),
        _ => Some(ClockOn::Restart(6)),
    }
}

/// How long after a Dojo section's clock shows 0 its challenger is sent
/// out (Cosmic `startDojoSchedule`'s 3000 ms on top of the clock).
pub const DOJO_OVERTIME: Duration = Duration::from_secs(3);

/// Whether `mob_id` is the friendly mob some event's team protects.
pub fn protected(mob_id: i32) -> bool {
    EventKind::ALL
        .iter()
        .any(|kind| kind.event().survive == Some(mob_id))
}

impl EventKind {
    pub const ALL: [EventKind; 45] = [
        EventKind::Kpq,
        EventKind::Hpq,
        EventKind::Lpq,
        EventKind::Opq,
        EventKind::Zpq,
        EventKind::Zakum,
        EventKind::Papulatus,
        EventKind::Horntail,
        EventKind::Htpq,
        EventKind::Delli,
        EventKind::LudiMaze,
        EventKind::Tylus,
        EventKind::Ellin,
        EventKind::Pirate,
        EventKind::Dojo,
        EventKind::DojoTrial,
        EventKind::DojoParty,
        EventKind::Pyramid(PyramidMode::Easy),
        EventKind::Pyramid(PyramidMode::Normal),
        EventKind::Pyramid(PyramidMode::Hard),
        EventKind::Pyramid(PyramidMode::Hell),
        EventKind::PyramidParty(PyramidMode::Easy),
        EventKind::PyramidParty(PyramidMode::Normal),
        EventKind::PyramidParty(PyramidMode::Hard),
        EventKind::PyramidParty(PyramidMode::Hell),
        EventKind::PyramidTomb(PyramidMode::Easy),
        EventKind::PyramidTomb(PyramidMode::Normal),
        EventKind::PyramidTomb(PyramidMode::Hard),
        EventKind::PyramidTomb(PyramidMode::Hell),
        EventKind::Carnival(1),
        EventKind::Carnival(2),
        EventKind::Carnival(3),
        EventKind::Carnival(4),
        EventKind::Carnival(5),
        EventKind::Carnival(6),
        EventKind::Magatia(MagatiaSide::Zenumist),
        EventKind::Magatia(MagatiaSide::Alcadno),
        EventKind::Tristan,
        EventKind::Dollhouse,
        EventKind::MechanicalParts,
        EventKind::KingPepe,
        EventKind::WeddingHall,
        EventKind::RockSpirit,
        EventKind::RockSpiritVip,
        EventKind::GuildQuest,
    ];

    /// The info quest whose record (`game::quest_ex`) keeps the player's
    /// runs of this party quest, and the rewards that record shows as had
    /// (`have`, or `have0`... for several), as QuestInfo's texts for it
    /// name them.
    pub fn record_quest(self) -> Option<(i16, &'static [i32])> {
        match self {
            EventKind::Hpq => Some((1200, &[1_002_798])),
            EventKind::Kpq => Some((1201, &[1_072_369])),
            EventKind::Lpq => Some((1202, &[1_022_073])),
            EventKind::Opq => Some((1203, &[1_082_232])),
            EventKind::Pirate => Some((1204, &[1_002_571, 1_002_572, 1_002_573, 1_002_574])),
            EventKind::Magatia(_) => Some((1205, &[1_122_010])),
            EventKind::Ellin => Some((1206, &[1_032_060, 1_032_061])),
            EventKind::Carnival(_) => Some((1301, &[1_122_007])),
            _ => None,
        }
    }

    pub fn event(self) -> &'static Event {
        match self {
            EventKind::Kpq => &KPQ,
            EventKind::Hpq => &HPQ,
            EventKind::Lpq => &LPQ,
            EventKind::Opq => &OPQ,
            EventKind::Zpq => &ZPQ,
            EventKind::Zakum => &ZAKUM,
            EventKind::Papulatus => &PAPULATUS,
            EventKind::Horntail => &HORNTAIL,
            EventKind::Htpq => &HTPQ,
            EventKind::Delli => &DELLI,
            EventKind::LudiMaze => &LUDIMAZE,
            EventKind::Tylus => &TYLUS,
            EventKind::Ellin => &ELLIN,
            EventKind::Pirate => &PIRATE,
            EventKind::Dojo => &DOJO,
            EventKind::DojoTrial => &DOJO_TRIAL,
            EventKind::DojoParty => &DOJO_PARTY,
            EventKind::Pyramid(mode) => &PYRAMID[mode as usize],
            EventKind::PyramidParty(mode) => &PYRAMID_PARTY[mode as usize],
            EventKind::PyramidTomb(mode) => &PYRAMID_TOMB_RUNS[mode as usize],
            EventKind::Carnival(room) => &CARNIVAL[usize::from(room.clamp(1, CARNIVAL_ROOMS) - 1)],
            EventKind::Magatia(side) => &MAGATIA[side as usize],
            EventKind::Tristan => &TRISTAN,
            EventKind::Dollhouse => &DOLLHOUSE,
            EventKind::MechanicalParts => &MECHANICAL_PARTS,
            EventKind::KingPepe => &KING_PEPE,
            EventKind::WeddingHall => &WEDDING_HALL_EVENT,
            EventKind::RockSpirit => &ROCK_SPIRIT,
            EventKind::RockSpiritVip => &ROCK_SPIRIT_VIP,
            EventKind::GuildQuest => &GUILD_QUEST,
        }
    }

    /// The mode of a Nett's Pyramid run, alone or with a party.
    pub fn pyramid_mode(self) -> Option<PyramidMode> {
        match self {
            EventKind::Pyramid(mode) | EventKind::PyramidParty(mode) => Some(mode),
            _ => None,
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
    /// for a squad or the Guild Quest `RUSTMS_EXPEDITION_MIN` if set, and
    /// for a Monster Carnival room one with `RUSTMS_CARNIVAL_SOLO=1`, for
    /// testing.
    pub fn least(&self) -> usize {
        let fewer = || {
            std::env::var("RUSTMS_EXPEDITION_MIN")
                .ok()?
                .parse()
                .ok()
                .filter(|least| *least >= 1)
        };
        let solo = || std::env::var("RUSTMS_CARNIVAL_SOLO").is_ok_and(|solo| solo == "1");
        if self.lobby == CARNIVAL_LOBBY && solo() {
            return 1;
        }
        (self.squad || self.lobby == crate::guild_quest::CAMP)
            .then(fewer)
            .flatten()
            .unwrap_or(*self.team.start())
    }

    /// Whether only the last member's leaving ends it: a squad's, or one
    /// that says so (`until_empty`).
    pub fn lasts_until_empty(&self) -> bool {
        self.squad || self.until_empty
    }

    /// Whether it is taken alone, without a party: a team of one.
    pub fn solo(&self) -> bool {
        *self.team.end() == 1
    }

    /// The map a member starts on: `entry`, or one of the `entries` maps
    /// from it.
    pub fn draw_entry(&self, rng: &mut impl rand::Rng) -> i32 {
        self.entry + rng.gen_range(0, self.entries.max(1))
    }

    /// Whether a team may start it on `map_id`: its lobby or one of its
    /// `entrances`.
    pub fn starts_on(&self, map_id: i32) -> bool {
        map_id == self.lobby || self.entrances.contains(&map_id)
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
        let gate = portal == "next00"
            || (portal == crate::guild_quest::GATE_PORTAL
                && map_id == crate::guild_quest::WAITING_ROOM);
        if !gate || !self.gated.contains(&map_id) {
            return None;
        }
        self.stage_on(map_id)
    }

    /// Take the event's own items from `player`, who is leaving it, worn
    /// ones too (Cosmic `setExclusiveItems`, `removeAll`). Returns the
    /// changes and whether any was worn.
    pub fn take_items(&self, player: &mut Player) -> (Vec<InventoryChange>, bool) {
        let mut changes = Vec::new();
        let mut worn = false;
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
            let slots: Vec<i16> = (player.inventory.items(game::InventoryType::Equipped))
                .filter(|(_, item)| item.item_id == item_id)
                .map(|(slot, _)| slot)
                .collect();
            for slot in slots {
                if let Ok(taken) =
                    player
                        .inventory_mut()
                        .take(game::InventoryType::Equipped, slot, 1)
                {
                    changes.extend(taken);
                    worn = true;
                }
            }
        }
        (changes, worn)
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
            .filter(|c| self.starts_on(c.map_id) && self.levels.contains(&c.level))
            .collect();
        let levels = team.iter().map(|c| c.level);
        let apart = levels.clone().max().unwrap_or(0) - levels.min().unwrap_or(0);
        let team: Vec<_> = team.iter().map(|c| c.id).collect();
        (team.contains(&leader)
            && (self.least()..=*self.team.end()).contains(&team.len())
            && self.spread.is_none_or(|spread| apart <= spread))
        .then_some(team)
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
/// like the events' time (`scaled`); Tylus's is Cosmic's. None for a mob
/// that makes nothing.
pub fn friendly_drop_period(mob_id: i32, period: i32) -> Option<Duration> {
    let period = if mob_id == TYLUS_MOB {
        TYLUS_DROP_PERIOD
    } else {
        period
    };
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

    /// `member` comes in with another party, as a Monster Carnival's
    /// challengers do.
    pub fn admit(&mut self, member: ClientId) {
        if !self.has(member) {
            self.members.push(member);
        }
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
    /// lasts until empty only follows a new leader who is in it.
    pub fn hand_over(&mut self, to: ClientId) -> bool {
        if self.roster.is_some() || self.until_empty && !self.has(to) {
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
    /// The leader handed Pietri this many coupons, at least
    /// `MAZE_COUPONS_WANTED`: the Ludibrium Maze is won, with
    /// `MAZE_COUPON_EXP` for each to every member (Cosmic
    /// `giveEventPlayersExp`, then `clearPQ`).
    WinWithCoupons(i32),
    /// The member's field brought out the event's boss, whose clock then
    /// starts over: Horntail's (`HORNTAIL_FIGHT_MINUTES`, Cosmic
    /// `restartEventTimer`).
    RestartClock,
    /// The member reached this map of the event's, whose clock may start
    /// over the first time anyone does (`clock_on`).
    Reached(i32),
    /// The member took their share of the won event's prize from Aura,
    /// once each (Cosmic's `gridInsert`): only their own ticket keeps it,
    /// so it goes when they leave.
    PrizeTaken,
    /// The leader handed Guon the Pirate Ship's `n`th Mark (from 1), in
    /// turn: the pirates of that Mark go, giving nothing, the next Mark's
    /// come, and the third clears the second stage.
    HandInMarks(u8),
    /// A member opened one of the Pirate Ship's bellflower rooms' chests.
    ChestOpened,
    /// A member went past the Pirate Ship's doors: the first brings out the
    /// Lord Pirate on the last stage (`lord_pirate`).
    PastTheDoors,
    /// A member took the party Dojo's way on to rest floor `n`, the floor
    /// before it cleared: everyone on the five floors before it goes there
    /// too, with those floors' points (Cosmic `dojang_next`).
    DojoRest(u8),
    /// The leader takes those on the party Dojo's rest floor `n` on to the
    /// next (Cosmic `2091005`'s `warpParty`).
    DojoOn(u8),
    /// An attack on a Nett's Pyramid stage killed, cool-killed or missed
    /// its mobs (Cosmic `Pyramid.kill`, `cool`, `miss`).
    Massacre(crate::pyramid::Hits),
    /// A member used Rage of Pharaoh, spending one of the Nett's Pyramid
    /// run's uses.
    Rage,
    /// A reactor here opened the way back from the room beside it: open
    /// `name` on the copy of `map_id` too, until its time there is up
    /// (Cosmic `2618001`'s `act`).
    OpenReactor { map_id: i32, name: &'static str },
    /// A member gave Romeo or Juliet their side's letter: the team hears
    /// it shook them, once.
    Shocked,
    /// Yulete left his office, caught off guard or not
    /// (`yulete_leaves`): the team hears what he said, once.
    YuleteLeft(bool),
    /// Yulete brought out his Frankenroid, the mighty one if he was
    /// caught off guard (`frankenroid_called`): the team hears it.
    FrankenroidOut(bool),
    /// The escort in the boss's room fainted (Cosmic `friendlyKilled`'s
    /// `escortFail`).
    EscortFailed,
    /// A member's attack in a Monster Carnival battle killed mobs worth
    /// this many CP (their `getCP`).
    CarnivalCp(i32),
    /// A member arrived on the Monster Carnival battle field, whose screen
    /// takes the score.
    CarnivalArrived,
    /// A member spends CP on `index` of the Carnival window's tab `tab`.
    CarnivalSpend { tab: u8, index: u8 },
    /// A member picked up an item that casts the Monster Carnival skill
    /// with this id (`MCSkill.img`) on the other side (`nuffSkill`).
    CarnivalCast(i32),
    /// A member died on the Monster Carnival battle field.
    CarnivalDied,
    /// The Guild Quest's registrant is ready for the statue keeper's test:
    /// the statues show the part's combo (`guild_quest::Statues::show`).
    ShowStatues,
    /// A member struck the statue numbered this, from 1.
    StatueStruck(u8),
    /// The Guild Quest's room of the spears has this many spears set.
    SpearsSet(u8),
    /// The Guild Quest's King's Corridor's door took the earring.
    KingGateOpened,
    /// A member died in the Guild Quest's last room, where `here` stand
    /// (they too) with `mobs` alive.
    KingRoomFell { here: usize, mobs: usize },
    /// A member who died has revived.
    Revived,
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
pub(crate) fn pinned_digits<const N: usize>(
    var: &str,
    range: RangeInclusive<u8>,
) -> Option<[u8; N]> {
    let pin = std::env::var(var).ok()?;
    let digits: Vec<u8> = pin.bytes().map(|b| b.wrapping_sub(b'0')).collect();
    let digits: [u8; N] = digits.try_into().ok()?;
    digits.iter().all(|d| range.contains(d)).then_some(digits)
}

/// The right pad of each row of a climb, drawn the first time anyone
/// steps on one (Cosmic `party3_r4pt`, `party3_r6pt`, `jnr5_rp`): on the
/// lounge two rows of three, on the tall room sixteen rows of four, and
/// in a Magatia rope room ten rows of four ropes, numbered from 0. For
/// testing, `RUSTMS_OPQ_LOUNGE` (`"23"`), `RUSTMS_OPQ_TOWER` (16 digits)
/// and `RUSTMS_MAGATIA_ROPES` (10) pin them.
pub fn draw_climb(map_id: i32, rng: &mut impl rand::Rng) -> Vec<u8> {
    static LOUNGE: OnceLock<Option<[u8; 2]>> = OnceLock::new();
    static TOWER: OnceLock<Option<[u8; 16]>> = OnceLock::new();
    static ROPES: OnceLock<Option<[u8; MAGATIA_ROPE_ROWS]>> = OnceLock::new();
    if MagatiaSide::ALL
        .iter()
        .any(|side| side.rope_room(map_id).is_some())
    {
        let pinned = ROPES.get_or_init(|| pinned_digits("RUSTMS_MAGATIA_ROPES", 0..=3));
        return pinned.map(|pin| pin.to_vec()).unwrap_or_else(|| {
            (0..MAGATIA_ROPE_ROWS)
                .map(|_| rng.gen_range(0, MAGATIA_ROPE_ROOMS))
                .collect()
        });
    }
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
///
/// In a Magatia rope room (`pt<row><rope>`, from 0) the right rope leads
/// to the same rope a row up, four ids on, and that of the last row to
/// `success` (1); a wrong one to `fail` (2), at the bottom (Cosmic
/// `rnj5_rp`).
pub fn climb(map_id: i32, combo: &[u8], portal_id: u8, name: &str) -> Option<u8> {
    let digits = |s: &str| s.parse::<usize>().ok();
    if MagatiaSide::ALL
        .iter()
        .any(|side| side.rope_room(map_id).is_some())
    {
        let rest = name.strip_prefix("pt").filter(|rest| rest.len() == 2)?;
        let (row, rope) = (digits(&rest[..1])?, digits(&rest[1..])?);
        let right = *combo.get(row)?;
        return Some(match (usize::from(right) == rope, row + 1 < combo.len()) {
            (true, true) => portal_id + 4,
            (true, false) => 1,
            (false, _) => 2,
        });
    }
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

/// The jails under the sealed room, each with its count of levers
/// (`lever1`.. on its map), as Cosmic's `party3_jailin` counts them.
pub const OPQ_JAILS: [(i32, usize); 3] = [(920_010_910, 7), (920_010_920, 6), (920_010_930, 6)];

/// The levers of jail `map_id`, if it is one.
pub fn jail_levers(map_id: i32) -> Option<usize> {
    OPQ_JAILS
        .iter()
        .find(|&&(jail, _)| jail == map_id)
        .map(|&(_, levers)| levers)
}

/// The levers of jail `map_id` that open its way on, drawn the first time
/// anyone tries it: each up or down by a coin (Cosmic `party3_jailin`; a
/// draw of none leaves the way open). Bit `i` is lever `i`. For testing,
/// `RUSTMS_OPQ_JAIL1`..`3` pin each jail's as the names of the levers up
/// (`"135"`).
pub fn draw_jail(map_id: i32, rng: &mut impl rand::Rng) -> u8 {
    let levers = jail_levers(map_id).unwrap_or(0);
    let jail = OPQ_JAILS.iter().position(|&(jail, _)| jail == map_id);
    let pinned = jail
        .and_then(|jail| std::env::var(format!("RUSTMS_OPQ_JAIL{}", jail + 1)).ok())
        .filter(|pin| {
            pin.bytes()
                .all(|b| (b'1'..=b'0' + levers as u8).contains(&b))
        });
    match pinned {
        Some(pin) => pin.bytes().fold(0, |mask, b| mask | 1 << (b - b'0')),
        None => (1..=levers).fold(0, |mask, i| {
            if rng.gen_bool(0.5) {
                mask | 1 << i
            } else {
                mask
            }
        }),
    }
}

/// What a jail's way on makes of its levers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JailCheck {
    /// Its draw wanted no lever up, or it was opened before (Cosmic sets
    /// the draw to 0): anyone goes by, however they stand.
    Open,
    /// They stand as drawn, which opens it.
    Solved,
    /// This many of them stand against the draw.
    Misplaced(usize),
}

/// A jail's levers against its draw `mask`; `up[i]` is lever `i + 1`.
pub fn check_jail(mask: u8, up: &[bool]) -> JailCheck {
    if mask == 0 {
        return JailCheck::Open;
    }
    let misplaced = |(i, &up): (usize, &bool)| up != (mask >> (i + 1) & 1 == 1);
    match up
        .iter()
        .enumerate()
        .filter(|&lever| misplaced(lever))
        .count()
    {
        0 => JailCheck::Solved,
        n => JailCheck::Misplaced(n),
    }
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
    fn the_dojo_has_thirty_two_bosses_and_a_rest_every_sixth_floor() {
        assert_eq!(DOJO.stage_on(DOJO_FIRST), Some(1));
        assert_eq!(DOJO.stage_on(DOJO_FIRST + 3700), Some(38));
        assert_eq!(DOJO.stage_on(DOJO_FIRST + 3800), None);
        assert_eq!(DOJO.stage_on(DOJO_FIRST + 1), None);
        assert!(!DOJO.holds(DOJO_HALL) && !DOJO.holds(DOJO_EXIT));
        assert!(rest_floor(6) && rest_floor(36) && !rest_floor(1) && !rest_floor(38));
        assert_eq!(dojo_round(5), 5);
        assert_eq!(dojo_round(7), 6);
        // Mano on the first floor, and the last boss on the 38th.
        assert_eq!(dojo_boss(DOJO_FIRST), Some(9300184));
        assert_eq!(dojo_boss(DOJO_FIRST + 3700), Some(9300215));
        assert_eq!(dojo_boss(DOJO_FIRST + 500), None);
    }

    #[test]
    fn a_party_takes_on_the_dojo_on_its_own_floors_within_thirty_levels() {
        assert_eq!(DOJO_PARTY.stage_on(DOJO_PARTY_FIRST), Some(1));
        assert_eq!(DOJO_PARTY.stage_on(DOJO_PARTY_FIRST + 3700), Some(38));
        assert_eq!(dojo_floor(DOJO_PARTY_FIRST + 500), Some(6));
        assert_eq!(dojo_floor(DOJO_FIRST + 500), Some(6));
        assert_eq!(dojo_floor(DOJO_HALL), None);
        assert_eq!(dojo_boss(DOJO_PARTY_FIRST), Some(9300184));
        assert_eq!(dojo_boss(DOJO_PARTY_FIRST + 500), None);
        assert_eq!(cleared_by_boss(DOJO_PARTY_FIRST + 600, 9300189), Some(7));
        assert_eq!(clock_on(DOJO_PARTY.stage_map(6)), Some(ClockOn::Stop));
        assert_eq!(clock_on(DOJO_PARTY.stage_map(7)), Some(ClockOn::Restart(6)));
        assert_eq!(DOJO_PARTY.runs, 5);
        let lobby = DOJO_PARTY.lobby;
        let team = DOJO_PARTY.team(1, &[at(1, 30, lobby), at(2, 60, lobby)]);
        assert_eq!(team, Some(vec![1, 2]));
        let broad = DOJO_PARTY.team(1, &[at(1, 30, lobby), at(2, 61, lobby)]);
        assert_eq!(broad, None);
        assert_eq!(DOJO_PARTY.team(1, &[at(1, 30, lobby)]), Some(vec![1]));
    }

    #[test]
    fn the_pyramid_runs_five_stages_in_each_mode_alone_or_with_a_party() {
        let normal = EventKind::Pyramid(PyramidMode::Normal).event();
        assert_eq!(normal.entry, 926011100);
        assert_eq!(normal.stage_map(5), 926011500);
        assert_eq!(normal.stage_on(926011300), Some(3));
        // Its copies are the instances', not the map ids' last digit.
        assert_eq!(normal.stage_on(926011301), None);
        assert_eq!(normal.exit, 926010002);
        assert!(!event_map(normal.exit) && !event_map(PYRAMID_LOBBY));
        assert!(!event_map(PYRAMID_SHADES));
        let hell = EventKind::PyramidParty(PyramidMode::Hell).event();
        assert_eq!((hell.entry, hell.stage_map(5)), (926023100, 926023500));
        assert_eq!(hell.exit, 926010004);
        assert_eq!(
            pyramid_stage(926021300),
            Some((EventKind::PyramidParty(PyramidMode::Normal), 3))
        );
        assert_eq!(pyramid_stage(PYRAMID_RESULT), None);
        assert_eq!(pyramid_kinds().count(), 8);
        // Two minutes for the first stage; the event's clock gives the rest.
        assert_eq!(normal.minutes, 2);
        assert_eq!(normal.last_stage(), PYRAMID_STAGES);
        assert!(normal.solo() && normal.lasts_until_empty());
        assert_eq!((normal.runs, hell.runs), (5, 10));
        // Only Hell is open above level 60, and a party needs two.
        assert!(normal.levels.contains(&60) && !normal.levels.contains(&61));
        assert!(hell.levels.contains(&120) && !hell.levels.contains(&39));
        let lobby = PYRAMID_LOBBY;
        assert_eq!(hell.team(1, &[at(1, 70, lobby)]), None);
        let team = hell.team(1, &[at(1, 70, lobby), at(2, 45, lobby)]);
        assert_eq!(team, Some(vec![1, 2]));
        assert_eq!(clock_on(normal.stage_map(2)), None);
    }

    #[test]
    fn pharaoh_yetis_tomb_is_one_room_alone_from_the_lobby_or_the_shades() {
        let hard = EventKind::PyramidTomb(PyramidMode::Hard).event();
        assert_eq!((hard.entry, hard.exit), (926010050, PYRAMID_LOBBY));
        assert!(hard.holds(926010050) && !hard.holds(926010051));
        assert!(hard.solo() && hard.lasts_until_empty());
        assert_eq!((hard.minutes, hard.last_stage(), hard.runs), (0, 0, 20));
        assert!(hard.starts_on(PYRAMID_LOBBY) && hard.starts_on(PYRAMID_SHADES + 2));
        assert!(!hard.starts_on(PYRAMID_SHADES + 3));
        assert!(hard.team(1, &[at(1, 120, PYRAMID_SHADES + 2)]).is_some());
        assert_eq!(hard.team(1, &[at(1, 39, PYRAMID_LOBBY)]), None);
        assert_eq!(pyramid_stage(926010050), None);
        assert_eq!(
            EventKind::PyramidTomb(PyramidMode::Hard).pyramid_mode(),
            None
        );
        let hell = EventKind::PyramidTomb(PyramidMode::Hell).event();
        assert_eq!(hell.entry, 926010070);
        assert_eq!(opening_mobs(hell.entry), [(PYRAMID_HELL_YETI, (0, 0), 1)]);
        assert_eq!(opening_mobs(hard.entry), [(PYRAMID_YETI, (0, 0), 1)]);
        assert!(event_map(926010010) && !event_map(PYRAMID_LOBBY));
    }

    #[test]
    fn jr_balrogs_room_is_taken_alone_from_mu_young_for_ten_minutes() {
        let tristan = EventKind::Tristan.event();
        assert_eq!((tristan.entry, tristan.exit), (TRISTAN_ROOM, MU_YOUNG_MAP));
        assert!(tristan.solo() && tristan.lasts_until_empty());
        assert_eq!(
            (tristan.minutes, tristan.last_stage(), tristan.runs),
            (10, 0, 1)
        );
        assert!(tristan.starts_on(MU_YOUNG_MAP));
        assert!(tristan.team(1, &[at(1, 50, MU_YOUNG_MAP)]).is_some());
        assert_eq!(tristan.team(1, &[at(1, 49, MU_YOUNG_MAP)]), None);
        assert!(event_map(TRISTAN_ROOM) && !event_map(MU_YOUNG_MAP));
    }

    #[test]
    fn the_spirit_of_rocks_floors_are_taken_alone_from_kerning_square() {
        let rock = EventKind::RockSpirit.event();
        assert_eq!(
            (rock.entry, rock.exit, rock.minutes, rock.runs),
            (ROCK_SPIRIT_FLOOR, KERNING_SQUARE_7F, 60, 8)
        );
        assert!(rock.solo() && rock.lasts_until_empty());
        assert!(rock.starts_on(KERNING_SQUARE_7F));
        assert!(rock.team(1, &[at(1, 35, KERNING_SQUARE_7F)]).is_some());
        let vip = EventKind::RockSpiritVip.event();
        assert_eq!(
            (vip.entry, vip.exit, vip.minutes, vip.runs),
            (ROCK_SPIRIT_VIP_FLOOR, KERNING_SQUARE_7F, 30, 8)
        );
        assert!(vip.solo() && vip.starts_on(KERNING_SQUARE_7F));
        for map in [103040410, 103040420, 103040430] {
            assert!(rock.holds(map) && !vip.holds(map));
        }
        for map in [103040440, 103040450, 103040460] {
            assert!(vip.holds(map) && !rock.holds(map));
        }
        assert!(!event_map(KERNING_SQUARE_7F) && !event_map(103040300));
    }

    #[test]
    fn king_pepes_room_is_taken_alone_from_the_east_castle_tower() {
        let dollhouse = EventKind::Dollhouse.event();
        assert_eq!(
            (dollhouse.entry, dollhouse.exit, dollhouse.minutes),
            (DOLLHOUSE_ROOM, EOS_TOWER_100, 10)
        );
        assert!(dollhouse.solo() && dollhouse.items == [DOLLHOUSE_PENDULUM]);
        assert!(event_map(DOLLHOUSE_ROOM) && !event_map(EOS_TOWER_100));
        let parts = EventKind::MechanicalParts.event();
        assert_eq!(
            (parts.entry, parts.exit, parts.minutes),
            (PARTS_ROOM, PARTS_PASSAGE, 20)
        );
        assert!(parts.solo() && parts.items == [MACHINE_PART]);
        assert!(parts.team(1, &[at(1, 29, TOY_FACTORY)]).is_none());
        assert!(event_map(PARTS_ROOM) && !event_map(PARTS_PASSAGE));
        let pepe = EventKind::KingPepe.event();
        assert_eq!((pepe.entry, pepe.exit), (KING_PEPE_ROOM, EAST_CASTLE_TOWER));
        assert!(pepe.solo() && pepe.lasts_until_empty());
        assert_eq!((pepe.minutes, pepe.last_stage(), pepe.runs), (20, 0, 1));
        assert!(pepe.starts_on(EAST_CASTLE_TOWER));
        assert!(pepe.team(1, &[at(1, 30, EAST_CASTLE_TOWER)]).is_some());
        assert_eq!(pepe.team(1, &[at(1, 29, EAST_CASTLE_TOWER)]), None);
        assert!(event_map(KING_PEPE_ROOM) && !event_map(EAST_CASTLE_TOWER));
    }

    #[test]
    fn the_wedding_hall_is_taken_alone_from_its_door() {
        let hall = EventKind::WeddingHall.event();
        assert_eq!((hall.entry, hall.exit), (WEDDING_HALL, WEDDING_HALL_DOOR));
        assert!(hall.solo() && hall.lasts_until_empty());
        assert_eq!((hall.minutes, hall.last_stage(), hall.runs), (10, 0, 1));
        assert!(hall.starts_on(WEDDING_HALL_DOOR));
        assert!(hall.team(1, &[at(1, 30, WEDDING_HALL_DOOR)]).is_some());
        assert_eq!(hall.team(1, &[at(1, 29, WEDDING_HALL_DOOR)]), None);
        assert!(event_map(WEDDING_HALL) && !event_map(WEDDING_HALL_DOOR));
    }

    #[test]
    fn dojo_sections_have_their_own_clocks() {
        let floor = |n: u8| clock_on(DOJO.stage_map(n));
        let restarts: Vec<_> = (1..=DOJO_FLOORS)
            .filter_map(|n| match floor(n) {
                Some(ClockOn::Restart(minutes)) => Some((n, minutes)),
                _ => None,
            })
            .collect();
        assert_eq!(
            restarts,
            [(7, 6), (13, 7), (19, 8), (25, 9), (31, 10), (37, 15)]
        );
        for n in [6, 12, 18, 24, 30, 36] {
            assert_eq!(floor(n), Some(ClockOn::Stop));
        }
        assert_eq!(floor(1), None);
        assert_eq!(floor(38), None);
        assert_eq!(DOJO.minutes, 5);
        let bosses: std::collections::BTreeSet<_> = (0..38)
            .filter_map(|floor| dojo_boss(DOJO_FIRST + floor * 100))
            .collect();
        assert_eq!(bosses.len(), 32);
        // Beating the floor's boss clears it, and only it.
        assert_eq!(cleared_by_boss(DOJO_FIRST + 600, 9300189), Some(7));
        assert_eq!(cleared_by_boss(DOJO_FIRST + 600, 9300190), None);
        assert_eq!(DOJO.last_stage(), 38);
        assert!(DOJO.solo() && !HPQ.solo());
        assert_eq!(DOJO.runs, 15);
    }

    #[test]
    fn so_gongs_trial_is_won_by_beating_him_with_no_time_limit() {
        assert_eq!(DOJO_TRIAL.stage_on(DOJO_TRIAL_MAP), Some(1));
        assert_eq!(DOJO_TRIAL.last_stage(), 1);
        assert!(DOJO_TRIAL.solo() && DOJO_TRIAL.until_empty);
        assert_eq!(DOJO_TRIAL.minutes, 0);
        assert_eq!(cleared_by_boss(DOJO_TRIAL_MAP, DOJO_TRIAL_MOB), Some(1));
        assert_eq!(cleared_by_boss(DOJO_FIRST, DOJO_TRIAL_MOB), None);
        // The room lets out its time mob once So Gong is beaten.
        let data = crate::get_game_data().unwrap();
        let room = data.field(DOJO_TRIAL_MAP).unwrap();
        let (time_mob, message) = room.time_mob.clone().unwrap();
        assert!(!message.is_empty());
        let so_gong = crate::mobs::info(DOJO_TRIAL_MOB).unwrap();
        assert_eq!(so_gong.revive, [time_mob]);
        assert!(room.portal_by_name("out000").is_some());
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
    fn a_jail_draws_each_of_its_levers_by_a_coin() {
        let mut rng = rand::thread_rng();
        let mut masks = HashSet::new();
        for _ in 0..2000 {
            let mask = draw_jail(920_010_910, &mut rng);
            assert_eq!(mask & !0b1111_1110, 0);
            masks.insert(mask);
            assert_eq!(draw_jail(920_010_920, &mut rng) & !0b0111_1110, 0);
        }
        assert_eq!(masks.len(), 128);
        assert_eq!(jail_levers(920_010_930), Some(6));
        assert_eq!(jail_levers(920_010_900), None);
    }

    #[test]
    fn a_jail_counts_its_levers_standing_wrong() {
        // Levers 1, 3 and 5 of 6 up.
        let mask = 0b10_1010;
        let up = |names: &[usize]| {
            let mut up = [false; 6];
            for &name in names {
                up[name - 1] = true;
            }
            check_jail(mask, &up)
        };
        assert_eq!(up(&[1, 3, 5]), JailCheck::Solved);
        assert_eq!(up(&[]), JailCheck::Misplaced(3));
        assert_eq!(up(&[1, 3, 5, 6]), JailCheck::Misplaced(1));
        assert_eq!(up(&[2, 4, 6]), JailCheck::Misplaced(6));
        // A draw of none, or an opened jail's, lets anyone by.
        assert_eq!(check_jail(0, &[false; 7]), JailCheck::Open);
        let up = [true, true, false, false, true, false, true];
        assert_eq!(check_jail(0, &up), JailCheck::Open);
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
    fn the_guild_quest_waiting_room_opens_once_gathered() {
        use crate::guild_quest::{GATE_PORTAL, WAITING_ROOM};
        assert_eq!(GUILD_QUEST.gate_stage(WAITING_ROOM, GATE_PORTAL), Some(1));
        assert_eq!(GUILD_QUEST.gate_stage(WAITING_ROOM, "st00"), None);
        assert_eq!(GUILD_QUEST.gate_stage(990_000_100, GATE_PORTAL), None);
        // Other events' gated maps don't take the waiting room's portal.
        assert_eq!(KPQ.gate_stage(103_000_800, GATE_PORTAL), None);
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
    fn leaving_the_guild_quest_takes_the_earring_worn() {
        let mut player = crate::codec::stats::sample_player();
        let earring = crate::guild_quest::EARRING;
        let worn = game::Item::stack(earring, 1);
        let slot = game::inventory::equip_slot::EARRINGS;
        player
            .inventory
            .insert(game::InventoryType::Equipped, slot, worn);
        let kept = game::Item::stack(1_002_067, 1);
        player.inventory.insert(
            game::InventoryType::Equipped,
            game::inventory::equip_slot::CAP,
            kept,
        );
        let (changes, worn) = GUILD_QUEST.take_items(&mut player);
        assert!(worn);
        assert_eq!(changes.len(), 1);
        let left = player.inventory.items(game::InventoryType::Equipped);
        assert!(left.map(|(_, item)| item.item_id).eq([1_002_067]));
        assert_eq!(GUILD_QUEST.take_items(&mut player), (Vec::new(), false));
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
        assert_eq!(KPQ.take_items(&mut player).0.len(), 3);
        assert!(player.is_dirty());
        assert_eq!(
            player.inventory.count(COUPON) + player.inventory.count(PASS),
            0
        );
        assert_eq!(player.inventory.count(4000000), 3);
        assert!(KPQ.take_items(&mut player).0.is_empty());
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
        // Horntail's ends the same way: neither the leader's leaving nor
        // falling below six does.
        let mut team = Team::new(&HORNTAIL, 1, vec![1, 2, 3, 4, 5, 6]);
        assert!(!team.leave(1));
        assert!(!team.leave(2));
        assert!(team.rejoin(1));
        for member in [3, 4, 5, 6] {
            assert!(!team.leave(member));
        }
        assert!(team.leave(1));
        assert!(EventKind::Horntail.event().holds(HORNTAILS_CAVE));
        assert!(!HORNTAIL.holds(HORNTAIL.exit));
        assert!(!event_map(HORNTAIL.lobby));
        // Its trials and its cave are stages any member's kill clears (a
        // head's, then Horntail's); the cave is the last.
        let heads = [HORNTAIL_TRIAL_I, HORNTAIL_TRIAL_II, HORNTAILS_CAVE].map(trial_head);
        assert_eq!(
            heads[0].map(|(stage, head)| (stage, head.mob)),
            Some((1, 8_810_000))
        );
        assert_eq!(
            heads[1].map(|(stage, head)| (stage, head.mob)),
            Some((2, 8_810_001))
        );
        assert_eq!(heads[2], None);
        assert_eq!(
            HORNTAIL.stage_on(HORNTAILS_CAVE),
            Some(HORNTAIL.last_stage())
        );
        assert_eq!(HORNTAIL.member_clears, &[1, 2, HORNTAIL.last_stage()]);
    }

    #[test]
    fn horntails_party_quest_takes_six_and_ends_when_any_one_leaves() {
        let six: Vec<_> = (1..=6).map(|id| at(id, 120, HTPQ.lobby)).collect();
        assert!(HTPQ.team(1, &six).is_some());
        assert!(HTPQ.team(1, &six[..5]).is_none());
        let mut low = six.clone();
        low[5].level = 119;
        assert!(HTPQ.team(1, &low).is_none());
        // Any one member's leaving ends it, until it is won.
        let mut team = Team::new(&HTPQ, 1, vec![1, 2, 3, 4, 5, 6]);
        assert!(team.leave(6));
        // Its ten maps are all its own, the Cave Entrance and the Entrance
        // to Horntail's Cave outside them.
        for map in [HTPQ_ENTRY, HTPQ_ENTRY + 5, HTPQ_LAMP, HTPQ_LIGHT, HTPQ_DARK] {
            assert!(event_map(map), "{map}");
        }
        assert!(!event_map(HTPQ.lobby));
        assert!(!event_map(HTPQ.clear_map.unwrap()));
        assert!(!event_map(240_050_500));
        // The four rooms' seals, then the last room's, then the win.
        assert_eq!(HTPQ.stage_on(HTPQ_ENTRY + 1), Some(1));
        assert_eq!(HTPQ.stage_on(HTPQ_ENTRY + 5), Some(5));
        assert_eq!(HTPQ.stage_on(HTPQ_ENTRY), None);
        assert_eq!(HTPQ.last_stage(), 6);
        assert_eq!(HTPQ.items.len(), 7);
    }

    #[test]
    fn papulatus_ends_only_when_its_last_member_leaves() {
        let mut team = Team::new(&PAPULATUS, 1, vec![1, 2, 3]);
        // A hand-over to one outside ends nothing and keeps the lead; to
        // one inside, it moves it.
        assert!(!team.hand_over(9));
        assert_eq!(team.leader(), 1);
        assert!(!team.hand_over(2));
        assert_eq!(team.leader(), 2);
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
    fn delli_takes_one_or_two_and_protects_willi() {
        let two = [at(1, 120, DELLI_LOBBY), at(2, 200, DELLI_LOBBY)];
        assert!(DELLI.team(1, &two[..1]).is_some());
        assert!(DELLI.team(1, &two).is_some());
        let low = [at(1, 120, DELLI_LOBBY), at(2, 119, DELLI_LOBBY)];
        assert_eq!(DELLI.team(1, &low), Some(vec![1]));
        let three = [two[0], two[1], at(3, 120, DELLI_LOBBY)];
        assert!(DELLI.team(1, &three).is_none());
        // Only Save Delli! is its own; the room past it is not.
        assert!(event_map(SAVE_DELLI));
        assert!(!event_map(DELLI_LOBBY) && !event_map(NAUTILUS_HIDEOUT));
        assert_eq!(DELLI.duration(), scaled(Duration::from_secs(6 * 60)));
        assert_eq!(DELLI.stage_on(SAVE_DELLI), Some(DELLI.last_stage()));
        // Willi is the one mob any event protects.
        assert!(protected(WILLI));
        assert!(!protected(crate::reactors::MOON_BUNNY));
        // Its leader's leaving ends it, as for any party's.
        let mut team = Team::new(&DELLI, 1, vec![1, 2]);
        assert!(team.leave(1));
    }

    #[test]
    fn tylus_takes_one_to_four_and_drops_his_trust_every_hundred_seconds() {
        let four: Vec<_> = (1..=4).map(|id| at(id, 80, TYLUS_LOBBY)).collect();
        assert_eq!(TYLUS.team(1, &four[..1]), Some(vec![1]));
        assert_eq!(TYLUS.team(1, &four), Some(vec![1, 2, 3, 4]));
        let five = [&four[..], &[at(5, 80, TYLUS_LOBBY)]].concat();
        assert!(TYLUS.team(1, &five).is_none());
        let low = [four[0], at(2, 79, TYLUS_LOBBY)];
        assert_eq!(TYLUS.team(1, &low), Some(vec![1]));
        // Only the room is its own; the thanks past it and the town are not.
        assert!(event_map(TYLUS_ROOM));
        assert!(!event_map(TYLUS_LOBBY) && !event_map(TYLUS_THANKS) && !event_map(TYLUS_EXIT));
        assert_eq!(TYLUS.duration(), scaled(Duration::from_secs(10 * 60)));
        assert_eq!(
            friendly_drop_period(TYLUS_MOB, 0),
            Some(scaled(Duration::from_secs(100)))
        );
        assert_eq!(friendly_drop_period(TYLUS_MOB + 1, 0), None);
        // His going loses nothing.
        assert!(!protected(TYLUS_MOB));
    }

    #[test]
    fn the_forest_takes_four_to_six_of_44_to_55_and_clears_its_first_stage_by_killing() {
        let four: Vec<_> = (1..=4).map(|id| at(id, 50, ELLIN_LOBBY)).collect();
        assert_eq!(ELLIN.team(1, &four), Some(vec![1, 2, 3, 4]));
        assert!(ELLIN.team(1, &four[..3]).is_none());
        let seven: Vec<_> = (1..=7).map(|id| at(id, 50, ELLIN_LOBBY)).collect();
        assert!(ELLIN.team(1, &seven).is_none());
        for level in [43, 56] {
            let off = [&four[..3], &[at(4, level, ELLIN_LOBBY)]].concat();
            assert!(ELLIN.team(1, &off).is_none(), "{level}");
        }
        // Its entrance and stages are its own; the prize map is everyone's.
        assert!(event_map(ELLIN_ENTRY) && ELLIN_STAGES.iter().all(|&map| event_map(map)));
        assert!(!event_map(ELLIN_LOBBY) && !event_map(ELLIN_EXIT));
        assert_eq!(ELLIN.last_stage(), 6);
        assert_eq!(ELLIN.stage_on(ELLIN_STAGES[1]), Some(2));
        assert!(ELLIN.member_clears.contains(&2));
        assert_eq!(ELLIN.stage_on(ELLIN_STAGES[3]), Some(4));
        assert_eq!(ELLIN.duration(), scaled(Duration::from_secs(30 * 60)));
        assert_eq!(cleared_by_killing(ELLIN_STAGES[0]), Some(1));
        assert_eq!(cleared_by_killing(ELLIN_STAGES[1]), None);
        assert_eq!(cleared_by_boss(ELLIN_STAGES[5], ELLIN_GOLEM_LAST), Some(6));
        assert_eq!(cleared_by_boss(ELLIN_STAGES[5], ELLIN_GOLEM), None);
        assert_eq!(cleared_by_boss(ELLIN_STAGES[4], ELLIN_GOLEM_LAST), None);
    }

    #[test]
    fn romeo_and_juliet_take_exactly_four_of_71_to_85_and_end_when_any_one_leaves() {
        for (side, lobby, stages) in [
            (MagatiaSide::Zenumist, ZENUMIST_LOBBY, ZENUMIST_STAGES),
            (MagatiaSide::Alcadno, ALCADNO_LOBBY, ALCADNO_STAGES),
        ] {
            let event = EventKind::Magatia(side).event();
            assert_eq!(event.lobby, lobby);
            let four: Vec<_> = (1..=4).map(|id| at(id, 75, lobby)).collect();
            assert_eq!(event.team(1, &four), Some(vec![1, 2, 3, 4]));
            assert!(event.team(1, &four[..3]).is_none());
            let five: Vec<_> = (1..=5).map(|id| at(id, 75, lobby)).collect();
            assert!(event.team(1, &five).is_none());
            for level in [70, 86] {
                let off = [&four[..3], &[at(4, level, lobby)]].concat();
                assert!(event.team(1, &off).is_none(), "{level}");
            }
            let mut team = Team::new(event, 1, vec![1, 2, 3, 4]);
            assert!(team.leave(4));
            // Its maps are its own; the room it lets out on isn't.
            assert!(stages.iter().all(|&map| event_map(map)));
            assert_eq!(stages[0], event.entry);
            assert!(!event_map(lobby) && !event_map(event.exit));
            assert_eq!(event.exit, event.entry + 700);
            assert_eq!(event.last_stage(), 7);
            assert_eq!(event.stage_on(stages[1]), Some(2));
            assert_eq!(event.duration(), scaled(Duration::from_secs(45 * 60)));
            assert_eq!(event.items, MAGATIA_ITEMS);
        }
        assert_eq!(ZENUMIST_STAGES[0], 926_100_000);
        assert_eq!(ALCADNO_STAGES[6], 926_110_401);
    }

    #[test]
    fn a_magatia_library_hides_one_switch_and_one_letter_among_its_books() {
        let mut rng = rand::thread_rng();
        let draws: Vec<_> = (0..20).map(|_| draw_books(&mut rng)).collect();
        for books in &draws {
            let count = |find| books.iter().filter(|&&book| book == find).count();
            assert_eq!(count(Some(Find::Pass)), 1);
            assert_eq!(count(Some(Find::Letter)), 1);
            assert_eq!(count(Some(Find::Mesos)), 7);
            assert_eq!(count(Some(Find::Exp)), 7);
            assert_eq!(count(None), 10);
        }
        let first = |books: &[Option<Find>; MAGATIA_BOOKS]| {
            books.iter().position(|&b| b == Some(Find::Pass))
        };
        assert!(draws.iter().any(|books| first(books) != first(&draws[0])));
        for side in MagatiaSide::ALL {
            let event = EventKind::Magatia(side).event();
            assert_eq!(cleared_by_killing(side.stages()[1]), Some(2));
            assert_eq!(cleared_by_killing(side.stages()[0]), None);
            assert_eq!(MagatiaSide::of(event.exit), Some(side));
            assert_eq!(MagatiaSide::of(side.stages()[6]), Some(side));
            assert!(event.member_clears.contains(&1) && event.member_clears.contains(&2));
        }
        assert_eq!(MagatiaSide::of(ZENUMIST_LOBBY), None);
    }

    #[test]
    fn yulete_minds_who_is_near_and_whether_the_letter_was_read() {
        use game_data::server_content::keys;
        let talk = |mood, near, shocked| yulete_talk(mood, near, shocked);
        assert_eq!(
            talk(Yulete::Unmet, false, true),
            (keys::MAGATIA_YULETE_CAUGHT, Yulete::Caught)
        );
        assert_eq!(
            talk(Yulete::Caught, false, true),
            (keys::MAGATIA_YULETE_CAUGHT, Yulete::Caught)
        );
        assert_eq!(
            talk(Yulete::Caught, true, true),
            (keys::MAGATIA_YULETE_WATCHED, Yulete::Leaving)
        );
        assert_eq!(
            talk(Yulete::Unmet, false, false),
            (keys::MAGATIA_YULETE_SNEAKY, Yulete::Leaving)
        );
        for near in [false, true] {
            assert_eq!(
                talk(Yulete::Leaving, near, true),
                (keys::MAGATIA_YULETE_LEAVING, Yulete::Leaving)
            );
        }
        assert_eq!(yulete_leaves(true), keys::MAGATIA_YULETE_DISGUSTED);
        assert_eq!(yulete_leaves(false), keys::MAGATIA_YULETE_FOOLED);
        for side in MagatiaSide::ALL {
            let event = EventKind::Magatia(side).event();
            let stages = side.stages();
            assert_eq!(cleared_by_killing(stages[4]), Some(MAGATIA_OFFICE_STAGE));
            assert!(event.member_clears.contains(&MAGATIA_OFFICE_STAGE));
            assert!(event.member_clears.contains(&MAGATIA_ROPES_STAGE));
            assert_eq!(side.rope_room(stages[5]), None);
            assert_eq!(side.rope_room(stages[5] + 1), Some(0));
            assert_eq!(side.rope_room(stages[5] + 4), Some(3));
            assert_eq!(side.rope_room(stages[5] + 5), None);
        }
    }

    #[test]
    fn yulete_brings_out_his_frankenroid_whose_fall_wins_the_run() {
        use game_data::server_content::keys;
        assert_eq!(magatia_win_exp(false, false), 105_000);
        assert_eq!(magatia_win_exp(false, true), 140_000);
        assert_eq!(magatia_win_exp(true, false), 90_000);
        assert_eq!(magatia_win_exp(true, true), 90_000);
        assert_eq!(yulete_in_lab(true, true, true), keys::MAGATIA_YULETE_BEATEN);
        assert_eq!(
            yulete_in_lab(false, true, false),
            keys::MAGATIA_YULETE_BEHOLD
        );
        assert_eq!(
            yulete_in_lab(false, false, false),
            keys::MAGATIA_YULETE_PRESENTS
        );
        assert_eq!(
            yulete_in_lab(false, false, true),
            keys::MAGATIA_YULETE_MIGHTY
        );
        assert_eq!(frankenroid_called(false), keys::MAGATIA_FRANKENROID);
        assert_eq!(frankenroid_called(true), keys::MAGATIA_FRANKENROID_MIGHTY);
        assert_eq!(magatia_fainted(9300137), Some(keys::MAGATIA_JULIET_FAINTED));
        assert_eq!(magatia_fainted(9300138), Some(keys::MAGATIA_ROMEO_FAINTED));
        assert_eq!(magatia_fainted(9300139), None);
        let z = MagatiaSide::Zenumist;
        let a = MagatiaSide::Alcadno;
        assert_eq!(
            [z.frankenroid(false), z.frankenroid(true)],
            [9300139, 9300140]
        );
        assert_eq!(
            [a.frankenroid(false), a.frankenroid(true)],
            [9300151, 9300152]
        );
        for id in [9300139, 9300140, 9300151, 9300152] {
            assert!(is_frankenroid(id));
        }
        assert!(!is_frankenroid(9300150));
        assert_eq!([z.gift(), a.gift()], [4001159, 4001160]);
        for side in MagatiaSide::ALL {
            let event = EventKind::Magatia(side).event();
            assert_eq!(event.last_stage(), MAGATIA_BOSS_STAGE);
            assert!(event.member_clears.contains(&MAGATIA_BOSS_STAGE));
            assert_eq!(
                side.stages()[usize::from(MAGATIA_BOSS_STAGE) - 1] % 1000,
                401
            );
        }
    }

    #[test]
    fn a_rope_room_sends_a_row_up_or_back_down() {
        let room = MagatiaSide::Zenumist.stages()[5] + 1;
        let mut rng = rand::thread_rng();
        let combo = draw_climb(room, &mut rng);
        assert_eq!(combo.len(), MAGATIA_ROPE_ROWS);
        assert!(combo.iter().all(|&rope| rope < MAGATIA_ROPE_ROOMS));
        let combo = [2, 0, 1, 3, 0, 1, 2, 3, 0, 1];
        // `pt<row><rope>` is portal 15 + 4 row + rope.
        let step =
            |row: u8, rope: u8| climb(room, &combo, 15 + 4 * row + rope, &format!("pt{row}{rope}"));
        assert_eq!(step(0, 2), Some(21));
        assert_eq!(step(0, 1), Some(2));
        assert_eq!(step(8, 0), Some(51));
        assert_eq!(step(9, 1), Some(1));
        assert_eq!(step(9, 0), Some(2));
        assert_eq!(climb(room, &combo, 3, "np00"), None);
        let alcadno = MagatiaSide::Alcadno.stages()[5] + 4;
        assert_eq!(climb(alcadno, &combo, 19, "pt10"), Some(23));
    }

    #[test]
    fn the_ship_takes_three_to_six_of_55_to_100_and_gives_more_time_at_each_stage() {
        let three: Vec<_> = (1..=3).map(|id| at(id, 55, PIRATE_LOBBY)).collect();
        assert_eq!(PIRATE.team(1, &three), Some(vec![1, 2, 3]));
        assert!(PIRATE.team(1, &three[..2]).is_none());
        let seven: Vec<_> = (1..=7).map(|id| at(id, 70, PIRATE_LOBBY)).collect();
        assert!(PIRATE.team(1, &seven).is_none());
        for level in [54, 101] {
            let off = [&three[..2], &[at(3, level, PIRATE_LOBBY)]].concat();
            assert!(PIRATE.team(1, &off).is_none(), "{level}");
        }
        // The bonus rooms and the clear room are its own; the way out isn't.
        let rooms = [925_100_201, 925_100_202, 925_100_301, 925_100_302];
        assert!(PIRATE_STAGES
            .iter()
            .chain(&rooms)
            .all(|&map| event_map(map)));
        assert!(event_map(PIRATE_CLEAR));
        assert!(!event_map(PIRATE_LOBBY) && !event_map(PIRATE_EXIT));
        assert_eq!(PIRATE.last_stage(), 6);
        assert_eq!(PIRATE.stage_on(PIRATE_STAGES[5]), Some(6));
        assert_eq!(PIRATE.duration(), scaled(Duration::from_secs(4 * 60)));
        // Cosmic `changedMapInside`: 6 minutes from the second stage on,
        // 8 for the Lord Pirate's; none on the first or off the stages.
        let minutes = |map| match clock_on(map) {
            Some(ClockOn::Restart(minutes)) => Some(minutes),
            _ => None,
        };
        let clocks: Vec<_> = PIRATE_STAGES.iter().map(|&map| minutes(map)).collect();
        assert_eq!(clocks, [None, Some(6), Some(6), Some(6), Some(6), Some(8)]);
        assert!(rooms.iter().all(|&map| clock_on(map).is_none()));
        assert_eq!(clock_on(PIRATE_CLEAR), None);
        // Its first stage clears with the last monster on it, for whoever
        // is there.
        assert_eq!(cleared_by_killing(PIRATE_ENTRY), Some(1));
        assert_eq!(cleared_by_killing(PIRATE_STAGES[1]), None);
        assert_eq!(cleared_by_killing(PIRATE_STAGES[2]), Some(3));
        assert_eq!(cleared_by_killing(PIRATE_STAGES[3]), Some(4));
        assert_eq!(cleared_by_killing(PIRATE_STAGES[4]), Some(5));
        assert_eq!(seals(PIRATE_STAGES[4]).len(), 4);
        assert!(PIRATE_STAGES[..4].iter().all(|&map| seals(map).is_empty()));
        let bosses: Vec<_> = (0..=3).map(lord_pirate).collect();
        assert_eq!(bosses, [9_300_119, 9_300_105, 9_300_106, 9_300_106]);
        assert!(bosses.iter().all(|&boss| is_lord_pirate(boss)));
        assert!(!is_lord_pirate(9_300_124));
        let exp: Vec<_> = (0..=3).map(pirate_win_exp).collect();
        assert_eq!(exp, [28_000, 35_000, 42_000, 42_000]);
        // The decks open with twenty of the Lord Pirate's crew each, and
        // no other map with any.
        for map in &PIRATE_STAGES[2..4] {
            let crew: usize = opening_mobs(*map).iter().map(|&(_, _, n)| n).sum();
            assert_eq!(crew, 20, "{map}");
        }
        assert!(opening_mobs(PIRATE_STAGES[1]).is_empty());
        assert!(opening_mobs(PIRATE_STAGES[2] + 2).is_empty());
        // Their bellflower rooms with twenty Bellflowers more.
        for map in [PIRATE_STAGES[2] + 1, PIRATE_STAGES[3] + 1] {
            let more: Vec<_> = opening_mobs(map)
                .iter()
                .map(|&(id, _, n)| (id, n))
                .collect();
            assert_eq!(more, [(9_300_112, 10), (9_300_113, 10)], "{map}");
        }
        assert_eq!(PIRATE.member_clears, [1, 3, 4, 5]);
    }

    #[test]
    fn the_maze_takes_three_to_six_and_starts_each_on_one_of_its_first_rooms() {
        let four: Vec<_> = (1..=4).map(|id| at(id, 51, LUDIBRIUM)).collect();
        assert_eq!(LUDIMAZE.team(1, &four), Some(vec![1, 2, 3, 4]));
        assert!(LUDIMAZE.team(1, &four[..2]).is_none());
        let high = [four[0], four[1], at(3, 71, LUDIBRIUM)];
        assert!(LUDIMAZE.team(1, &high).is_none());
        let seven: Vec<_> = (1..=7).map(|id| at(id, 70, LUDIBRIUM)).collect();
        assert!(LUDIMAZE.team(1, &seven).is_none());
        // Its rooms and the clear room are its own; the way out is not.
        assert!(event_map(MAZE_ROOM) && event_map(MAZE_GOAL) && event_map(MAZE_CLEAR));
        assert!(!event_map(MAZE_EXIT) && !event_map(LUDIBRIUM));
        assert_eq!(
            (LUDIMAZE.stage_on(MAZE_GOAL), LUDIMAZE.last_stage()),
            (Some(1), 1)
        );
        assert_eq!(LUDIMAZE.duration(), scaled(Duration::from_secs(15 * 60)));
        // Each member starts on one of the first 15 rooms, the goal never;
        // any other event's on its entry.
        let mut rng = rand::thread_rng();
        let drawn: HashSet<i32> = (0..600).map(|_| LUDIMAZE.draw_entry(&mut rng)).collect();
        assert_eq!(drawn, (MAZE_ROOM..MAZE_GOAL).collect());
        assert_eq!(KPQ.draw_entry(&mut rng), KPQ.entry);
        // Dying in the maze lands on its way out, where Rolly sends
        // everyone home; only room 8 leads to Pietri's.
        let data = crate::game_data::get().unwrap();
        for room in MAZE_ROOM..=MAZE_GOAL {
            let field = data.field(room).unwrap();
            assert_eq!(field.forced_return, Some(MAZE_EXIT), "{room}");
            assert_eq!(field.return_map, Some(MAZE_EXIT), "{room}");
            let to_goal = field.portals().any(|p| p.to_map == Some(MAZE_GOAL));
            assert_eq!(to_goal, room == MAZE_ROOM + 8, "{room}");
        }
        let npcs = |map_id| -> Vec<i32> {
            let field = data.field(map_id).unwrap();
            field.map_npcs.iter().map(|npc| npc.npc_id).collect()
        };
        assert!(npcs(LUDIBRIUM).contains(&crate::npc_scripts::ROLLY));
        assert!(npcs(MAZE_GOAL).contains(&9103000));
        assert!(npcs(MAZE_EXIT).contains(&9103003));
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
