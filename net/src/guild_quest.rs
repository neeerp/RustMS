//! The Guild Quest to Sharenian: a guild's master or a Jr. Master
//! registers the guild with Shuang (9040000) at the Excavation Site's
//! camp, and each channel runs one guild's quest at a time, the rest
//! waiting their turn in a line of up to ten (Cosmic `EventManager`'s
//! guild queue, `EVENT_MAX_GUILD_QUEUE`). The guild whose turn it is has
//! three minutes in the waiting room to gather before the way into
//! Sharenian opens (`events::GUILD_QUEST`). Pure rules; the directory
//! keeps each channel's line.

use std::collections::VecDeque;
use std::time::Duration;

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
    fn the_waitlist_numbers_follow_the_client() {
        assert_eq!(waitlist_number(None), 1);
        assert_eq!(waitlist_number(Some(1)), 2);
        assert_eq!(waitlist_number(Some(3)), 4);
    }
}
