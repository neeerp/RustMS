//! Party events played in a copy of their maps of their own (an instance):
//! who may start one, where it is played, and when it ends. Pure rules;
//! the runtime's event actor keeps the time and moves the players.

use crate::ClientId;
use std::ops::RangeInclusive;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Kerning City's party quest, started at Lakelis.
    Kpq,
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
    /// Where everyone goes when they leave or the event ends.
    pub exit: i32,
    pub levels: RangeInclusive<u8>,
    /// How many may play, the leader included.
    pub team: RangeInclusive<usize>,
    pub minutes: u64,
}

/// Kerning City's party quest (Cosmic `KerningPQ`, Server.NX
/// `FieldSet.img/Party1`: maps 103000800-805, 30 minutes).
pub const KPQ: Event = Event {
    lobby: 103_000_000,
    maps: 103_000_800..=103_000_805,
    entry: 103_000_800,
    exit: 103_000_890,
    levels: 21..=30,
    team: 3..=4,
    minutes: 30,
};

impl EventKind {
    pub fn event(self) -> &'static Event {
        match self {
            EventKind::Kpq => &KPQ,
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
        Duration::from_secs(self.minutes * 60) / scale()
    }

    pub fn holds(&self, map_id: i32) -> bool {
        self.maps.contains(&map_id)
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
/// remain (Cosmic `isEventTeamLackingNow`, before the event is cleared).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    leader: ClientId,
    members: Vec<ClientId>,
    least: usize,
}

impl Team {
    pub fn new(event: &Event, leader: ClientId, members: Vec<ClientId>) -> Self {
        Self {
            leader,
            members,
            least: *event.team.start(),
        }
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
        member == self.leader || self.members.len() < self.least
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(id: ClientId, level: u8, map_id: i32) -> Candidate {
        Candidate { id, level, map_id }
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
    fn kpq_lasts_half_an_hour_on_its_own_maps() {
        assert_eq!(KPQ.duration(), Duration::from_secs(30 * 60));
        assert!(KPQ.holds(103_000_800) && KPQ.holds(103_000_805));
        assert!(!KPQ.holds(KPQ.exit) && !KPQ.holds(KPQ.lobby));
    }
}
