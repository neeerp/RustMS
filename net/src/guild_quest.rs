//! The Guild Quest to Sharenian: a guild's master or a Jr. Master
//! registers the guild with Shuang (9040000) at the Excavation Site's
//! camp, and each channel runs one guild's quest at a time, the rest
//! waiting their turn in a line of up to ten (Cosmic `EventManager`'s
//! guild queue, `EVENT_MAX_GUILD_QUEUE`). The guild whose turn it is has
//! three minutes in the waiting room to gather before the way into
//! Sharenian opens (`events::GUILD_QUEST`). Pure rules; the directory
//! keeps each channel's line.

use game_data::Area;
use rand::seq::index::sample;
use rand::Rng;
use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Victoria Road: Excavation Site <Camp>, where Shuang registers guilds and
/// lets their members in.
pub const CAMP: i32 = 101_030_104;
/// Sharenian's waiting room, where the guild gathers before its quest.
pub const WAITING_ROOM: i32 = 990_000_000;
/// Sharenian: Returning Path, where everyone goes when the quest ends;
/// Nuris there takes them back to the camp.
pub const RETURNING_PATH: i32 = 990_001_100;
/// The waiting room's way into Sharenian, shut until the guild has had its
/// time to gather.
pub const GATE_PORTAL: &str = "join00";

pub const SHUANG: i32 = 9_040_000;
pub const NURIS: i32 = 9_040_001;
/// On every stage, offering the way out of the quest.
pub const EXIT_GUIDE: i32 = 9_040_005;

/// Sharenian's outskirts, the first map past the waiting room, where Shuang
/// gives the brief.
pub const OUTSKIRTS: i32 = 990_000_100;
/// The protector earring the outskirts' rocks drop, which keeps the air of
/// the fortress off its wearer.
pub const EARRING: i32 = 1_032_033;
/// The quest's own items, taken from whoever leaves it (Cosmic
/// `GuildQuest.js`'s `setEventExclusives`).
pub const ITEMS: &[i32] = &[
    EARRING, 4_001_024, 4_001_025, 4_001_026, 4_001_027, 4_001_028, 4_001_029, 4_001_030,
    4_001_031, 4_001_032, 4_001_033, 4_001_034, 4_001_035, 4_001_037,
];

/// The hall of statues, past the outskirts, where the statue keeper
/// (`STATUE_KEEPER`) tests the guild before the gate opens.
pub const STATUE_HALL: i32 = 990_000_300;
pub const STATUE_KEEPER: i32 = 9_040_009;
/// The hall's gate, which the test's last part opens (state 1), and the
/// way on behind it.
pub const STATUE_GATE: &str = "statuegate";
/// The statues: reactors 9208000-9208002, named "1" to "20".
pub const STATUES: std::ops::RangeInclusive<i32> = 9_208_000..=9_208_002;
pub const STATUE_NAMES: [&str; 20] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20",
];
/// The parts of the statue keeper's test.
pub const STATUE_PARTS: u8 = 3;
/// Guild points for passing the whole test (Cosmic `9040009`'s `gainGP`).
pub const STATUE_POINTS: i32 = 15;
/// When the first statue of a combo moves, and how far apart the rest do
/// (Cosmic `9040009`'s `delayedHitReactor`).
pub const FIRST_SHOWN: Duration = Duration::from_millis(5_000);
pub const SHOWN_EVERY: Duration = Duration::from_millis(3_500);

/// The knights' hall, past the hall of statues: its gates open on the
/// spears (`SPEAR_GATE`), a stone and a piece of metal, and the maps off it
/// (`KNIGHT_MAPS`) share its stage.
pub const KNIGHT_HALL: i32 = 990_000_400;
/// The knights' hall's stage (`crate::events::GUILD_QUEST`'s, by map).
pub const KNIGHT_STAGE: u8 = 5;
/// The knights' hall and the maps off it, where the stage's clear shows
/// (Cosmic `9208007`'s `showEffect` on each), but not the way on past it.
pub const KNIGHT_MAPS: [i32; 6] = [
    KNIGHT_HALL,
    990_000_410,
    990_000_420,
    990_000_430,
    990_000_431,
    990_000_440,
];
/// The spears on the room of the spears (990000440), each set by throwing
/// a spear (4001025) on it.
pub const SPEAR: i32 = 9_208_007;
/// The knights' hall's gate, which moves a state for each spear set, and
/// the way on past it once all are.
pub const SPEAR_GATE: &str = "speargate";
pub const SPEARS: u8 = 4;
/// Guild points for setting the last spear (Cosmic `9208007`'s `gainGP`).
pub const SPEAR_POINTS: i32 = 20;
/// The hall's gate to the metal room (990000430), opened by a stone
/// (4001026), and the metal room's gate to the room past it (990000431),
/// opened by a piece of metal (4001037).
pub const STONE_GATE: &str = "stonegate";
pub const METAL_GATE: &str = "metalgate";

/// The Fountain of the Wiseman, past the knights' hall, where the vassal
/// keeper (`VASSAL_KEEPER`) judges the offerings the guild sets out before
/// its four vassals. Two of the kinds come from its knights, and two from
/// the dining hall and the cellar off it (990000501 and 990000502).
pub const FOUNTAIN: i32 = 990_000_500;
pub const VASSAL_KEEPER: i32 = 9_040_006;
/// The fountain's gate, which the right offerings open (state 1), and the
/// way on behind it.
pub const WATER_GATE: &str = "watergate";
/// The offerings, one kind for each digit of the vassals' wish, from 0:
/// the knights' two items, the spoiled food and the drink.
pub const OFFERINGS: std::ops::RangeInclusive<i32> = 4_001_027..=4_001_030;
/// The vassals, one for each of the fountain's areas, from 0.
pub const VASSALS: usize = 4;
/// How many offerings the guild may make before the wish changes.
pub const OFFERING_TRIES: u8 = 7;
/// Guild points for the right offerings (Cosmic `9040006`'s `gainGP`).
pub const VASSAL_POINTS: i32 = 25;
/// The knights a wrong offering brings out, one of each where it says, and
/// how many of each the last wrong one brings out anywhere along
/// `KNIGHTS_X`.
pub const KNIGHTS: [(i32, (i16, i16)); 2] = [(9_300_036, (-350, 150)), (9_300_037, (400, 150))];
pub const KNIGHTS_ON_FAILING: usize = 6;
pub const KNIGHTS_X: std::ops::Range<i16> = -350..400;

/// The Underground Waterway, past the fountain: a maze of portals to four
/// rooms, three behind a door (`SECRET_GATES`) that opens on a stack of
/// Marks of Evil, and one (990000640) only those of `UNDER_30_LEVEL` or
/// lower may enter. Past each lies a room where a piece of Sharen III's
/// clothes drops, and the way on to the grave (990000700).
pub const WATERWAY: i32 = 990_000_600;
/// Each door's reactor, its room and the room it opens on (whose portal 1
/// it leads to).
pub static SECRET_GATES: [(&str, i32, i32); 3] = [
    ("secretgate1", 990_000_610, 990_000_611),
    ("secretgate2", 990_000_630, 990_000_631),
    ("secretgate3", 990_000_640, 990_000_641),
];
pub const UNDER_30_LEVEL: u8 = 30;
/// The altar on the gargoyles' room's ledge (990000630), which takes the
/// Gargoyle's drop and brings out `JR_GARGOYLES` of `JR_GARGOYLE` on the
/// ground below `JR_GARGOYLES_AT`, whose Marks open its door.
pub const GARGOYLE_ALTAR: i32 = 9_201_000;
pub const JR_GARGOYLE: i32 = 9_300_033;
pub const JR_GARGOYLES: usize = 8;
pub const JR_GARGOYLES_AT: (i16, i16) = (-100, 50);
/// Sharen III's Grave (990000700), whose gate (forced open once the
/// guild has dressed Sharen III's soul) leads on to the King's Corridor.
pub const GRAVE: i32 = 990_000_700;
pub const GHOST_GATE: &str = "ghostgate";
/// The altar on the grave (9201001), which takes Sharen III's shoes,
/// pants, top and crown (4001032, 4001031, 4001033, 4001034) in turn and
/// brings out Sharen III's soul (`SHAREN_SOUL`).
pub const CLOTHES_ALTAR: i32 = 9_201_001;
pub const SHAREN_SOUL: i32 = 9_040_003;
/// Guild points for opening the grave's gate (Cosmic `9040003`'s `gainGP`).
pub const SOUL_POINTS: i32 = 30;
/// The end rooms' ways to the grave, each recording the room it was taken
/// from (Cosmic `guild1F01` to `4`'s `gridInsert`), and the grave's way
/// back, to the waterway's portal beside the room last recorded
/// (`guild1F00`'s `backPortals`).
pub const GRID_WAYS: [&str; 4] = ["guild1F01", "guild1F02", "guild1F04", "guild1F03"];
pub const GRID_BACK: &str = "guild1F00";
pub const BACK_PORTALS: [u8; 4] = [6, 8, 9, 11];
/// The grave's `st00`, where the end rooms' ways come out.
pub const GRAVE_ARRIVAL: u8 = 1;
/// The waterway's `st00`, the way back for one who never came through a
/// room (Cosmic has no such case).
pub const WATERWAY_ARRIVAL: u8 = 1;
/// The King's Corridor (990000800), whose door (9208009, `kinggate`) takes
/// Sharen III's earring (`KING_GATE_ITEM`) and stays open, letting the
/// guild on to Ergoth's room by its two ways, `KING_WAYS` (Cosmic
/// `kinggate_open`, `kinggate2_open`). Before it opens, a member who dies
/// in the quest revives on its way out and leaves it; from then on, on its
/// maps (Cosmic `9208009`'s `canRevive`).
pub const KING_CORRIDOR: i32 = 990_000_800;
pub const KING_GATE: &str = "kinggate";
pub const KING_GATE_REACTOR: i32 = 9_208_009;
pub const KING_GATE_ITEM: i32 = 1_032_033;
pub const KING_ROOM: i32 = 990_000_900;
pub const KING_WAYS: [(&str, u8); 2] = [("kinggate_open", 1), ("kinggate2_open", 2)];

/// Where the grid's way `script` leads a member who last came from `room`,
/// and the room it records: `None` for any other portal.
pub fn grid_way(script: &str, room: Option<u8>) -> Option<((i32, u8), Option<u8>)> {
    if script == GRID_BACK {
        let back = room.and_then(|room| BACK_PORTALS.get(usize::from(room)));
        return Some(((WATERWAY, *back.unwrap_or(&WATERWAY_ARRIVAL)), room));
    }
    let way = GRID_WAYS.iter().position(|way| *way == script)?;
    Some(((GRAVE, GRAVE_ARRIVAL), u8::try_from(way).ok()))
}

/// How many guilds may wait on a channel, besides the one playing.
pub const QUEUE_MAX: usize = 10;
/// How long the guild whose turn it is has to gather (Cosmic
/// `GuildQuest.js`'s `waitTime`).
pub const GATHERING: Duration = Duration::from_secs(3 * 60);
/// How long after the client's notice that the quest can't go on it ends
/// (0x4A, 0x4B: "will end in 5 seconds").
pub const CLOSING: Duration = Duration::from_secs(5);
/// The guild ranks that may register: the master (1) and the Jr. Masters
/// (2) (Cosmic `isGuildLeader`).
pub const REGISTRANT_RANKS: std::ops::RangeInclusive<u8> = 1..=2;

/// What a member asks of Shuang.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Put the guild in line.
    Register,
    /// Go into the guild's waiting room while it gathers.
    Join,
}

/// What Shuang knows of the channel's line, for a member asking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// How many guilds wait.
    pub waiting: usize,
    pub full: bool,
    /// The member's guild is in line or playing, here or elsewhere.
    pub listed: bool,
    /// The member's guild is gathering on this channel, so they may go in.
    pub gathering: bool,
}

/// A guild waiting its turn, and who registered it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiting {
    pub guild_id: i32,
    pub registrant: i32,
}

/// Why a guild can't be put in line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Ten guilds wait already.
    Full,
    /// The guild is in line or playing on this channel or another.
    Listed,
}

/// One channel's guilds: the one playing, if any, and those waiting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    playing: Option<Waiting>,
    waiting: VecDeque<Waiting>,
}

impl Line {
    /// The guild playing, if any.
    pub fn playing(&self) -> Option<Waiting> {
        self.playing
    }

    /// How many guilds wait.
    pub fn waiting(&self) -> usize {
        self.waiting.len()
    }

    pub fn full(&self) -> bool {
        self.waiting.len() >= QUEUE_MAX
    }

    /// Whether `guild_id` plays or waits here.
    pub fn has(&self, guild_id: i32) -> bool {
        self.playing.is_some_and(|guild| guild.guild_id == guild_id)
            || self.waiting.iter().any(|guild| guild.guild_id == guild_id)
    }

    /// Put `guild` at the end of the line. Returns its place, from 1.
    pub fn register(&mut self, guild: Waiting) -> Result<usize, Refusal> {
        if self.has(guild.guild_id) {
            return Err(Refusal::Listed);
        }
        if self.full() {
            return Err(Refusal::Full);
        }
        self.waiting.push_back(guild);
        Ok(self.waiting.len())
    }

    /// The first guild in line whose registrant `present` finds takes its
    /// turn, if nobody plays; those passed over leave the line (Cosmic
    /// `attemptStartGuildInstance`). Returns the guild now playing and
    /// those passed over.
    pub fn next(&mut self, present: impl Fn(i32) -> bool) -> (Option<Waiting>, Vec<Waiting>) {
        let mut passed = Vec::new();
        if self.playing.is_some() {
            return (None, passed);
        }
        while let Some(guild) = self.waiting.pop_front() {
            if present(guild.registrant) {
                self.playing = Some(guild);
                return (Some(guild), passed);
            }
            passed.push(guild);
        }
        (None, passed)
    }

    /// `guild_id`'s turn is over.
    pub fn done(&mut self, guild_id: i32) {
        if self.playing.is_some_and(|guild| guild.guild_id == guild_id) {
            self.playing = None;
        }
    }

    /// Those waiting, first in line first.
    pub fn line(&self) -> impl Iterator<Item = &Waiting> {
        self.waiting.iter()
    }
}

/// What the statue keeper tells the registrant (Cosmic `9040009`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatueTalk {
    /// The statues are shown to the registrant alone.
    NotLeader,
    /// The gate is open.
    Cleared,
    /// What part `n` (from 1) asks; when they're ready the statues move.
    Intro(u8),
    /// The statues are still showing their combo.
    Showing,
    /// The guess was right, and part `n` is passed; the next is harder.
    Passed(u8),
    /// The guess was wrong: the test starts over from the first part.
    Failed,
    /// The last part is passed: the gate opens.
    Won,
}

/// The statue keeper's test (Cosmic `9040009` and the statues' scripts):
/// part `n` shows `n + 3` statues in turn, which the guild then strikes in
/// that order before the registrant talks to the keeper again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statues {
    /// The part under way, from 1.
    part: u8,
    /// The statues to strike, from 1, empty until the part is shown.
    combo: Vec<u8>,
    /// When the combo's last statue moves, after which strikes count.
    shown_at: Option<Instant>,
    guess: Vec<u8>,
}

impl Default for Statues {
    fn default() -> Self {
        Statues {
            part: 1,
            combo: Vec::new(),
            shown_at: None,
            guess: Vec::new(),
        }
    }
}

impl Statues {
    /// How many statues part `part` shows.
    pub fn combo_len(part: u8) -> usize {
        usize::from(part) + 3
    }

    /// The registrant talks to the keeper at `now`, the gate not yet
    /// open: the part's intro until it is shown, or a wait while it is,
    /// or the guess judged.
    pub fn talk(&mut self, now: Instant) -> StatueTalk {
        let Some(shown_at) = self.shown_at else {
            return StatueTalk::Intro(self.part);
        };
        if now < shown_at {
            return StatueTalk::Showing;
        }
        let right = self.guess == self.combo;
        self.combo.clear();
        self.guess.clear();
        self.shown_at = None;
        if !right {
            self.part = 1;
            StatueTalk::Failed
        } else if self.part >= STATUE_PARTS {
            self.part = 1;
            StatueTalk::Won
        } else {
            self.part += 1;
            StatueTalk::Passed(self.part - 1)
        }
    }

    /// The registrant is ready at `now`: the part's statues are drawn, to
    /// be shown in turn from `now + FIRST_SHOWN`. Returns them, or None if
    /// a combo is out already.
    pub fn show(&mut self, now: Instant, rng: &mut impl Rng) -> Option<Vec<u8>> {
        if self.shown_at.is_some() {
            return None;
        }
        let len = Self::combo_len(self.part);
        self.combo = (sample(rng, STATUE_NAMES.len(), len).into_iter())
            .map(|index| index as u8 + 1)
            .collect();
        self.shown_at = Some(now + shown_after(len - 1));
        self.guess.clear();
        Some(self.combo.clone())
    }

    /// Statue `statue` (from 1) was struck at `now`: once its combo is
    /// shown it counts toward the guess, up to the combo's length.
    pub fn struck(&mut self, statue: u8, now: Instant) {
        if self.shown_at.is_some_and(|shown_at| now >= shown_at)
            && self.guess.len() < self.combo.len()
        {
            self.guess.push(statue);
        }
    }
}

/// What Sharen III's soul tells a member (Cosmic `9040003`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoulTalk {
    NotLeader,
    Cleared,
    /// The registrant hears the soul out: the stage clears and the gate
    /// opens.
    Opened,
}

/// What the vassal keeper tells a member (Cosmic `9040006`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VassalTalk {
    NotLeader,
    Cleared,
    /// The vassals have a new wish; the registrant hears the test.
    Intro,
    /// Not one offering of the kinds in each area, and four in all.
    NoOffering,
    Right,
    Wrong(Hint),
    /// The last try was wrong: the wish is forgotten until the next talk.
    Failed,
}

/// How a wrong offering went down (Cosmic `9040006`): `pleased` vassals got
/// what they wished for; of the rest, `unknown` offerings are kinds beyond
/// what any vassal wished for, and `wrong` ones are wished for elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    pub pleased: u8,
    pub wrong: u8,
    pub unknown: u8,
    /// Which try this was, from 1.
    pub attempt: u8,
}

/// The vassals' wish, drawn when the registrant first asks, and the tries
/// made at it.
#[derive(Clone, Debug, Default)]
pub struct Vassals {
    wish: Option<[u8; VASSALS]>,
    attempt: u8,
}

impl Vassals {
    /// The registrant talks to the keeper with `offering` set out (see
    /// `offering`).
    pub fn talk(&mut self, offering: Option<[u8; VASSALS]>, rng: &mut impl Rng) -> VassalTalk {
        let Some(wish) = self.wish else {
            self.wish = Some(draw_wish(rng));
            self.attempt = 1;
            return VassalTalk::Intro;
        };
        let Some(offering) = offering else {
            return VassalTalk::NoOffering;
        };
        if offering == wish {
            return VassalTalk::Right;
        }
        if self.attempt >= OFFERING_TRIES {
            self.wish = None;
            return VassalTalk::Failed;
        }
        let hint = hint(wish, offering, self.attempt);
        self.attempt += 1;
        VassalTalk::Wrong(hint)
    }
}

/// A new wish: each vassal wishes for any of the four kinds. For testing,
/// `RUSTMS_GQ_VASSALS` (four digits 0-3) pins it.
fn draw_wish(rng: &mut impl Rng) -> [u8; VASSALS] {
    static PIN: OnceLock<Option<[u8; VASSALS]>> = OnceLock::new();
    if let Some(pin) = PIN.get_or_init(|| crate::events::pinned_digits("RUSTMS_GQ_VASSALS", 0..=3))
    {
        return *pin;
    }
    std::array::from_fn(|_| rng.gen_range(0, 4))
}

fn hint(wish: [u8; VASSALS], offering: [u8; VASSALS], attempt: u8) -> Hint {
    let (mut wished, mut offered) = ([0u8; VASSALS], [0u8; VASSALS]);
    let mut pleased = 0;
    for (&w, &o) in wish.iter().zip(&offering) {
        if w == o {
            pleased += 1;
        } else {
            wished[usize::from(w)] += 1;
            offered[usize::from(o)] += 1;
        }
    }
    let unknown = (offered.iter().zip(&wished))
        .map(|(o, w)| o.saturating_sub(*w))
        .sum::<u8>();
    Hint {
        pleased,
        wrong: VASSALS as u8 - pleased - unknown,
        unknown,
        attempt,
    }
}

/// The offering the items on the fountain's floor make, each `(item id,
/// where it lies)`: exactly four of the kinds, one in each of the first
/// four `areas`; None otherwise. Items of other kinds don't count.
pub fn offering(items: &[(i32, (i32, i32))], areas: &[Area]) -> Option<[u8; VASSALS]> {
    let offered: Vec<_> = (items.iter())
        .filter(|(id, _)| OFFERINGS.contains(id))
        .collect();
    if offered.len() != VASSALS {
        return None;
    }
    let mut set = [None; VASSALS];
    for &&(id, (x, y)) in &offered {
        let area = (areas.iter().take(VASSALS)).position(|area| area.contains(x, y));
        if let Some(area) = area {
            set[area] = Some((id - OFFERINGS.start()) as u8);
        }
    }
    let mut offering = [0; VASSALS];
    for (kind, set) in offering.iter_mut().zip(set) {
        *kind = set?;
    }
    Some(offering)
}

/// How long after the statues start moving the `n`th (from 0) of a combo
/// is shown; `RUSTMS_EVENT_SCALE` shortens it.
pub fn shown_after(n: usize) -> Duration {
    crate::events::scaled(FIRST_SHOWN + SHOWN_EVERY * n as u32)
}

/// The statue named `name`'s number, from 1.
pub fn statue_number(name: &str) -> Option<u8> {
    let number = name.parse().ok()?;
    (1..=STATUE_NAMES.len() as u8)
        .contains(&number)
        .then_some(number)
}

/// GUILD_OPERATION 0x4C's number for a guild at `place` in line, from 1,
/// or None for the guild whose turn it is: 1 calls it in, 2 says it is
/// next, and more says it is `n - 1`th (the 2025 client's 0xddb-0xddd).
pub fn waitlist_number(place: Option<usize>) -> i32 {
    match place {
        None => 1,
        Some(place) => i32::try_from(place)
            .unwrap_or(i32::MAX - 1)
            .saturating_add(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guild(guild_id: i32) -> Waiting {
        Waiting {
            guild_id,
            registrant: guild_id * 10,
        }
    }

    #[test]
    fn guilds_wait_in_line_up_to_ten() {
        let mut line = Line::default();
        for n in 1..=10 {
            assert_eq!(line.register(guild(n)), Ok(n as usize));
        }
        assert!(line.full());
        assert_eq!(line.register(guild(11)), Err(Refusal::Full));
        assert_eq!(line.register(guild(3)), Err(Refusal::Listed));
    }

    #[test]
    fn the_next_guild_with_its_registrant_present_plays() {
        let mut line = Line::default();
        for n in 1..=3 {
            line.register(guild(n)).unwrap();
        }
        let (playing, passed) = line.next(|registrant| registrant != 10);
        assert_eq!(playing, Some(guild(2)));
        assert_eq!(passed, vec![guild(1)]);
        assert!(line.has(2));
        assert_eq!(line.register(guild(2)), Err(Refusal::Listed));
        // Nobody else plays while it does.
        assert_eq!(line.next(|_| true), (None, vec![]));
        line.done(2);
        assert!(!line.has(2));
        assert_eq!(line.next(|_| true), (Some(guild(3)), vec![]));
        line.done(3);
        assert_eq!(line.next(|_| true), (None, vec![]));
    }

    fn area(x1: i32, x2: i32) -> Area {
        Area {
            x1,
            x2,
            y1: 100,
            y2: 200,
        }
    }

    #[test]
    fn the_vassals_hint_counts_pleased_wrong_and_unknown_offerings() {
        let hint = |wish, offering| {
            let h = super::hint(wish, offering, 1);
            (h.pleased, h.wrong, h.unknown)
        };
        assert_eq!(hint([0, 1, 2, 3], [0, 1, 3, 2]), (2, 2, 0));
        // Two of kind 3 offered where only one is wished for: one unknown.
        assert_eq!(hint([0, 1, 2, 3], [3, 1, 2, 3]), (3, 0, 1));
        assert_eq!(hint([0, 0, 0, 0], [1, 2, 3, 1]), (0, 0, 4));
        assert_eq!(hint([0, 1, 2, 3], [1, 2, 3, 0]), (0, 4, 0));
    }

    #[test]
    fn the_vassals_take_seven_tries_then_wish_anew() {
        let mut rng = rand::rngs::mock::StepRng::new(0, 0);
        let mut vassals = Vassals::default();
        assert_eq!(vassals.talk(Some([0; 4]), &mut rng), VassalTalk::Intro);
        let wish = vassals.wish.unwrap();
        let wrong = wish.map(|kind| (kind + 1) % 4);
        assert_eq!(vassals.talk(None, &mut rng), VassalTalk::NoOffering);
        for attempt in 1..OFFERING_TRIES {
            let VassalTalk::Wrong(hint) = vassals.talk(Some(wrong), &mut rng) else {
                panic!("try {attempt}");
            };
            assert_eq!(hint.attempt, attempt);
        }
        assert_eq!(vassals.talk(Some(wrong), &mut rng), VassalTalk::Failed);
        assert_eq!(vassals.talk(Some(wrong), &mut rng), VassalTalk::Intro);
        let wish = vassals.wish.unwrap();
        assert_eq!(vassals.talk(Some(wish), &mut rng), VassalTalk::Right);
    }

    #[test]
    fn an_offering_is_four_kinds_one_in_each_area() {
        let areas = [area(0, 10), area(10, 20), area(20, 30), area(30, 40)];
        let at = |x| (x, 150);
        let four = [
            (4_001_030, at(5)),
            (4_001_027, at(15)),
            (4_001_028, at(25)),
            (4_001_029, at(35)),
        ];
        assert_eq!(offering(&four, &areas), Some([3, 0, 1, 2]));
        // Other items don't count.
        let mut more = four.to_vec();
        more.push((4_001_025, at(5)));
        assert_eq!(offering(&more, &areas), Some([3, 0, 1, 2]));
        assert_eq!(offering(&four[..3], &areas), None);
        more.push((4_001_027, at(5)));
        assert_eq!(offering(&more, &areas), None);
        // Two in one area leave another empty; one outside every area.
        let mut doubled = four;
        doubled[1].1 = at(6);
        assert_eq!(offering(&doubled, &areas), None);
        let mut outside = four;
        outside[3].1 = (35, 250);
        assert_eq!(offering(&outside, &areas), None);
    }

    #[test]
    fn the_statues_test_three_parts_of_growing_combos() {
        let mut rng = rand::thread_rng();
        let mut statues = Statues::default();
        let start = Instant::now();
        let mut now = start;
        for part in 1..=STATUE_PARTS {
            assert_eq!(statues.talk(now), StatueTalk::Intro(part));
            let combo = statues.show(now, &mut rng).unwrap();
            assert_eq!(combo.len(), Statues::combo_len(part));
            assert!(combo.iter().all(|n| (1..=20).contains(n)));
            let mut distinct = combo.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), combo.len());
            // Another word while showing draws nothing new.
            assert_eq!(statues.show(now, &mut rng), None);
            assert_eq!(statues.talk(now), StatueTalk::Showing);
            // Strikes before the last statue moves don't count.
            statues.struck(combo[0], now);
            now += shown_after(combo.len() - 1);
            for &statue in &combo {
                statues.struck(statue, now);
            }
            // Nor do those past the combo's length.
            statues.struck(combo[0], now);
            let told = statues.talk(now);
            if part < STATUE_PARTS {
                assert_eq!(told, StatueTalk::Passed(part));
            } else {
                assert_eq!(told, StatueTalk::Won);
            }
        }
    }

    #[test]
    fn a_wrong_guess_starts_the_statues_over() {
        let mut rng = rand::thread_rng();
        let mut statues = Statues::default();
        let mut now = Instant::now();
        for _ in 0..2 {
            let combo = statues.show(now, &mut rng).unwrap();
            now += Duration::from_secs(60);
            for &statue in &combo {
                statues.struck(statue, now);
            }
            statues.talk(now);
        }
        assert_eq!(statues.talk(now), StatueTalk::Intro(3));
        let combo = statues.show(now, &mut rng).unwrap();
        now += Duration::from_secs(60);
        for &statue in combo.iter().rev() {
            statues.struck(statue, now);
        }
        assert_eq!(statues.talk(now), StatueTalk::Failed);
        assert_eq!(statues.talk(now), StatueTalk::Intro(1));
        // No strike at all is wrong too.
        statues.show(now, &mut rng).unwrap();
        now += Duration::from_secs(60);
        assert_eq!(statues.talk(now), StatueTalk::Failed);
    }

    #[test]
    fn statues_are_numbered_by_name() {
        assert_eq!(statue_number("1"), Some(1));
        assert_eq!(statue_number("20"), Some(20));
        assert_eq!(statue_number("21"), None);
        assert_eq!(statue_number("0"), None);
        assert_eq!(statue_number("statuegate"), None);
    }

    #[test]
    fn the_knights_hall_plays_its_stage_and_the_maps_off_it_are_rooms() {
        let event = &crate::events::GUILD_QUEST;
        assert_eq!(event.stage_on(KNIGHT_HALL), Some(KNIGHT_STAGE));
        assert_eq!(event.stage_map(KNIGHT_STAGE), KNIGHT_HALL);
        for map_id in &KNIGHT_MAPS[1..] {
            assert_eq!(event.stage_on(*map_id), None, "{map_id}");
            assert!(event.holds(*map_id), "{map_id}");
        }
        assert_eq!(event.stage_on(STATUE_HALL), Some(KNIGHT_STAGE - 1));
    }

    #[test]
    fn the_grid_takes_each_end_room_to_the_grave_and_back_beside_it() {
        let data = crate::get_game_data().unwrap();
        // Each end room's way and the room it records, in Cosmic's order.
        for (end, room) in [(611, 0), (620, 1), (641, 2), (631, 3)] {
            let field = data.field(990_000_000 + end).unwrap();
            let way = field
                .portals()
                .find_map(|p| p.script.as_deref().filter(|s| s.starts_with("guild1F")))
                .unwrap();
            assert_eq!(
                grid_way(way, None),
                Some(((GRAVE, GRAVE_ARRIVAL), Some(room))),
                "{end}"
            );
            let back = BACK_PORTALS[usize::from(room)];
            assert_eq!(
                grid_way(GRID_BACK, Some(room)),
                Some(((WATERWAY, back), Some(room)))
            );
            assert!(data
                .field(WATERWAY)
                .unwrap()
                .portal_by_id(back.into())
                .is_some());
        }
        let grave = data.field(GRAVE).unwrap();
        assert_eq!(
            grave.portal_by_name("out00").unwrap().script.as_deref(),
            Some(GRID_BACK)
        );
        assert!(grave.portal_by_id(GRAVE_ARRIVAL.into()).is_some());
        assert_eq!(
            grid_way(GRID_BACK, None),
            Some(((WATERWAY, WATERWAY_ARRIVAL), None))
        );
        assert_eq!(grid_way("guild1F05", Some(1)), None);
        let event = &crate::events::GUILD_QUEST;
        assert_eq!(event.stage_on(WATERWAY), Some(7));
        assert_eq!(event.stage_on(GRAVE), Some(8));
    }

    #[test]
    fn the_waitlist_numbers_follow_the_client() {
        assert_eq!(waitlist_number(None), 1);
        assert_eq!(waitlist_number(Some(1)), 2);
        assert_eq!(waitlist_number(Some(3)), 4);
    }
}
