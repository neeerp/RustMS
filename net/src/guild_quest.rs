//! The Guild Quest to Sharenian: a guild's master or a Jr. Master
//! registers the guild with Shuang (9040000) at the Excavation Site's
//! camp, and each channel runs one guild's quest at a time, the rest
//! waiting their turn in a line of up to ten (Cosmic `EventManager`'s
//! guild queue, `EVENT_MAX_GUILD_QUEUE`). The guild whose turn it is has
//! three minutes in the waiting room to gather before the way into
//! Sharenian opens (`events::GUILD_QUEST`). Pure rules; the directory
//! keeps each channel's line.

use rand::seq::index::sample;
use rand::Rng;
use std::collections::VecDeque;
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
    fn the_waitlist_numbers_follow_the_client() {
        assert_eq!(waitlist_number(None), 1);
        assert_eq!(waitlist_number(Some(1)), 2);
        assert_eq!(waitlist_number(Some(3)), 4);
    }
}
