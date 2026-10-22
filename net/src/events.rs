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
    /// is played on `entry + (n - 1) * stride`.
    pub exp_per_stage: &'static [i32],
    /// How far apart the stage maps' ids are; the maps between them are
    /// rooms off the stages.
    pub stride: i32,
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
}

/// A stage that `npc` clears once the leader hands it `count` of `item`
/// (Ludibrium's balloons, Cosmic `2040036`-`2040044`).
#[derive(Debug, PartialEq, Eq)]
pub struct HandIn {
    pub stage: u8,
    pub npc: i32,
    pub item: i32,
    pub count: i16,
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

/// Kerning City's party quest (Cosmic `KerningPQ`, Server.NX
/// `FieldSet.img/Party1`: maps 103000800-805, 30 minutes). Stage EXP is
/// Cosmic's; `next00` leads on from 103000800-803, and 800-804 each have a
/// `gate` object (Map.nx). Stages 2 to 4 are Cloto's ropes, platforms and
/// barrels; stage 5's monsters drop the 10 passes that clear it (Cosmic
/// `9020001`, `clearPQ`).
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
    gated: &[103_000_800, 103_000_801, 103_000_802, 103_000_803],
    puzzles: 2..=4,
    areas_taken: 3,
    items: &[COUPON, PASS],
    passes: 10,
    hand_ins: &[],
    gates: true,
    clear_map: None,
    bonus: None,
};

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
    }
}

/// What the Moon Bunny makes on Primrose Hill, and Growlie wants.
pub const RICE_CAKE: i32 = 4001101;

/// What Tory gives each member of a party that won Henesys's party quest,
/// once (Cosmic `setEventRewards`; the default in
/// `v83ref/lanes/QUESTIONS.md`, 2026-10-22, row 180).
pub const HPQ_REWARD: i32 = 4001158;

impl EventKind {
    pub fn event(self) -> &'static Event {
        match self {
            EventKind::Kpq => &KPQ,
            EventKind::Hpq => &HPQ,
            EventKind::Lpq => &LPQ,
        }
    }
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

    /// Whether `map_id` is one of the event's maps; its exit never is.
    pub fn holds(&self, map_id: i32) -> bool {
        self.maps.contains(&map_id) && map_id != self.exit
    }

    /// The map stage `stage` (from 1) is played on.
    pub fn stage_map(&self, stage: u8) -> i32 {
        self.entry + (i32::from(stage) - 1) * self.stride
    }

    /// The stage played on `map_id`, if any; none is played in its rooms.
    pub fn stage_on(&self, map_id: i32) -> Option<u8> {
        let offset = map_id.checked_sub(self.entry)?;
        if offset.checked_rem(self.stride)? != 0 {
            return None;
        }
        let stage = u8::try_from(offset / self.stride + 1).ok()?;
        (1..=self.exp_per_stage.len())
            .contains(&usize::from(stage))
            .then_some(stage)
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
}

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
/// `isEventTeamLackingNow`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    leader: ClientId,
    members: Vec<ClientId>,
    least: usize,
    won: bool,
}

impl Team {
    pub fn new(event: &Event, leader: ClientId, members: Vec<ClientId>) -> Self {
        Self {
            leader,
            members,
            least: *event.team.start(),
            won: false,
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
        if self.won {
            return self.members.is_empty();
        }
        member == self.leader || self.members.len() < self.least
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
