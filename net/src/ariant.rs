//! The Ariant Coliseum's Battle Arena (Cosmic's AriantPQ, `2101014`,
//! `2101017` and `AriantColiseum`): from its lobby a player of level 30 or
//! under opens one of three arenas for a few, or joins one still
//! gathering, and waits at its entrance; its host takes those there into
//! its battle map, where they catch scorpions for Spirit Jewels for ten
//! minutes (`EventKind::Ariant`). Pure rules; the directory keeps each
//! arena's roster, a squad (`crate::squads`).

use crate::squads::SquadView;
use crate::ClientId;
use std::ops::RangeInclusive;
use std::time::Duration;

/// The Coliseum's lobby, where Cesar (2101014) opens the arenas. Its
/// `out00` (portal 3, `aMatchMove2`) leads back to where the player came
/// from by the Dimensional Mirror.
pub const ARIANT_LOBBY: i32 = 980_010_000;
/// The mirror's way into the lobby lands by its way out.
pub const LOBBY_PORTAL: u8 = 3;
/// Where `aMatchMove2` goes when no way back was saved (Ariant).
pub const ARIANT_FALLBACK: i32 = 260_000_000;
/// The King's Room, where a battle's players go at its end, to Areda
/// (2101016) for their prizes.
pub const KINGS_ROOM: i32 = 980_010_010;
/// The room past the King's Room, on the way back to the lobby.
pub const ARIANT_EXIT: i32 = 980_010_020;
/// How many arenas there are.
pub const ARENAS: u8 = 3;
/// Who may go into a battle (Cosmic `ExpeditionType.ARIANT`).
pub const ARIANT_LEVELS: RangeInclusive<u8> = 20..=30;
/// How many an arena's host may let in, at least and at most: the
/// lobby's Cesar asks for 2 to 5, and the arena holds 7
/// (`ExpeditionType.ARIANT`).
pub const ARIANT_CAP: RangeInclusive<usize> = 2..=7;

/// The Spirit Jewel, the scorpions' prize and a player's score.
pub const SPIRIT_JEWEL: i32 = 4_031_868;
/// The Element Rock, which catches a weakened scorpion.
pub const ELEMENT_ROCK: i32 = 2_270_002;
/// The Bomb, which knocks the others' jewels out (`ItemInfo::req_map`).
pub const BOMB: i32 = 2_100_067;
/// What Cesar (2101017) gives each player in a battle, once a run.
pub const KIT: [(i32, i16); 2] = [(ELEMENT_ROCK, 50), (BOMB, 5)];
/// How long before a battle's ten minutes are up its results come
/// (Cosmic's `pqTimerBoard`, 9:50): the scoreboard, the mobs gone and
/// each player's points.
pub const RESULTS_BEFORE: Duration = Duration::from_secs(10);
/// How often the players see the scores change at most (Cosmic's
/// `broadcastAriantScoreUpdate`, every 500 ms while they have).
pub const SCORES_EVERY: Duration = Duration::from_millis(500);
/// The Palm Tree Beach Chair, which Abdullah VIII (2101015) trades for
/// `CHAIR_POINTS` Battle Arena points.
pub const CHAIR: i32 = 3_010_018;
pub const CHAIR_POINTS: i32 = 100;
/// Areda's (2101016) EXP for each point, times the player's EXP rate.
pub const EXP_PER_POINT: f64 = 92.7;

/// Arena `arena`'s entrance (980010n00), where its players wait for the
/// host and Cesar (2101017) stands.
pub const fn entrance(arena: u8) -> i32 {
    ARIANT_LOBBY + 100 * arena as i32
}

/// Arena `arena`'s battle map, past its entrance.
pub const fn battle(arena: u8) -> i32 {
    entrance(arena) + 1
}

/// The arena whose entrance is `map_id`.
pub fn arena_at(map_id: i32) -> Option<u8> {
    (1..=ARENAS).find(|&arena| entrance(arena) == map_id)
}

/// The arena whose battle map is `map_id`.
pub fn battle_of(map_id: i32) -> Option<u8> {
    (1..=ARENAS).find(|&arena| battle(arena) == map_id)
}

/// How many the host of a new arena lets in, from what they typed at
/// the lobby's Cesar: its leading number as JavaScript's `parseInt`
/// reads it, at least 2, and at most the arena's 7 (Cosmic takes any
/// number above 2).
pub fn cap(typed: &str) -> Result<usize, CapRefusal> {
    let typed = typed.trim_start();
    let (negative, digits) = match typed.as_bytes().first() {
        Some(b'-') => (true, &typed[1..]),
        Some(b'+') => (false, &typed[1..]),
        _ => (false, typed),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return Err(CapRefusal::NotANumber);
    }
    let value = digits[..end].parse::<usize>().unwrap_or(usize::MAX);
    if negative || value < *ARIANT_CAP.start() {
        return Err(CapRefusal::TooFew);
    }
    Ok(value.min(*ARIANT_CAP.end()))
}

/// The Battle Arena points each player of a battle earns, from how many
/// Spirit Jewels each holds (`scores`, in the order they first held some)
/// and how many the bombs knocked out or lie on the floor (`lost`): one
/// for every ten, but the winner's are 1 if the others were too far
/// behind (Cosmic `distributeAriantPoints` and `isUnfairMatch`). The
/// winner is the first with the most. Cosmic drops the runner-up whose
/// index is the winner's score (`List.remove(int)`), not the winner; here
/// the others are everyone but the winner (`v83ref/lanes/QUESTIONS.md`
/// b303).
pub fn points(scores: &[(ClientId, i32)], lost: i32) -> Vec<(ClientId, i32)> {
    let mut points: Vec<_> = (scores.iter())
        .map(|&(who, jewels)| (who, jewels.max(0) / 10))
        .collect();
    let mut winner: Option<(usize, i32)> = None;
    for (index, &(_, jewels)) in scores.iter().enumerate() {
        if winner.is_none_or(|(_, most)| jewels > most) {
            winner = Some((index, jewels));
        }
    }
    let Some((index, first)) = winner else {
        return points;
    };
    let others = (scores.iter().enumerate())
        .filter(|&(at, _)| at != index)
        .map(|(_, &(_, jewels))| jewels);
    let second = others.clone().max().unwrap_or(-1);
    let rest: i32 = others.sum();
    if unfair(first, second, lost, rest) {
        points[index].1 = 1;
    }
    points
}

/// Whether the winner, with `first` jewels, left the others too far
/// behind: the next best with `second`, all of them with `rest`, and
/// `lost` counted to both (Cosmic `isUnfairMatch`).
fn unfair(first: i32, second: i32, lost: i32, rest: i32) -> bool {
    if first <= 0 {
        return false;
    }
    let rest = f64::from(rest) + f64::from(lost);
    let second = f64::from(second + lost);
    let first = f64::from(first);
    rest / first < 0.817_707_268_919_801_2 && (rest < 7.0 || second / first < 0.5929)
}

/// A battle's scores: the Spirit Jewels each player last held, in the
/// order they first told, and how many the bombs knocked out of bags
/// (Cosmic's `score` and `lostShards`). A player who leaves keeps their
/// last score in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scores {
    held: Vec<(ClientId, String, i32)>,
    lost: i32,
}

impl Scores {
    /// `who`, named `name`, holds `jewels`, after the bombs knocked `lost`
    /// out. Whether the board changed.
    pub fn hold(&mut self, who: ClientId, name: &str, jewels: i32, lost: i32) -> bool {
        self.lost = self.lost.saturating_add(lost.max(0));
        match self.held.iter_mut().find(|(id, _, _)| *id == who) {
            Some((_, _, held)) if *held == jewels => false,
            Some((_, _, held)) => {
                *held = jewels;
                true
            }
            None => {
                self.held.push((who, name.to_string(), jewels));
                true
            }
        }
    }

    /// Each player's name and score, as the board shows them.
    pub fn board(&self) -> Vec<(String, i32)> {
        (self.held.iter())
            .map(|(_, name, jewels)| (name.clone(), *jewels))
            .collect()
    }

    /// Each player's Battle Arena points, with `on_floor` jewels lying on
    /// the battle map (`points`).
    pub fn points(&self, on_floor: i32) -> Vec<(ClientId, i32)> {
        let held: Vec<_> = (self.held.iter())
            .map(|&(who, _, jewels)| (who, jewels))
            .collect();
        points(&held, self.lost.saturating_add(on_floor))
    }
}

/// Areda's EXP for `points` Battle Arena points at EXP rate `rate`.
pub fn prize_exp(points: i32, rate: f64) -> i32 {
    (EXP_PER_POINT * rate * f64::from(points)).clamp(0.0, f64::from(i32::MAX)) as i32
}

/// Why the lobby's Cesar won't open an arena for the number typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapRefusal {
    NotANumber,
    TooFew,
}

/// What the lobby's Cesar shows of an arena.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arena {
    /// Nobody holds it: the player may open it.
    Empty,
    /// A host gathers players there: its roster, the host first, and
    /// what joining would find (`SquadView::Joinable`, `Full` or
    /// `Banned`).
    Gathering { names: Vec<String>, view: SquadView },
    /// A battle is on in it: not shown.
    Busy,
}

/// What the lobby's Cesar finds on the player's channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lobby {
    /// Arenas 1 to 3.
    pub arenas: Vec<Arena>,
    /// Whether the player is on an arena's roster already.
    pub registered: bool,
}

/// What Cesar finds at an arena's entrance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Host {
    /// The arena has no roster.
    None,
    /// Its roster's names, the host first, to another.
    Member(Vec<String>),
    /// To its host: the roster, the host first, and whether they may
    /// take it in now.
    Leading {
        members: Vec<(ClientId, String)>,
        start: Result<(), StartRefusal>,
    },
}

/// Why the host may not take the arena's players into battle, in the
/// order Cosmic's `startAriantBattle` finds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartRefusal {
    /// The host is in a party.
    HostInParty,
    /// Too few or too many: between `least` and `most` must go.
    Count { least: usize, most: usize },
    /// One of them isn't at the entrance.
    Away,
    /// One of them is in a party.
    InParty,
    /// One of them isn't of `ARIANT_LEVELS`.
    Levels,
    /// A battle is on in the arena already.
    Busy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arenas_maps_follow_the_lobby() {
        assert_eq!(entrance(1), 980_010_100);
        assert_eq!(battle(3), 980_010_301);
        assert_eq!(arena_at(980_010_200), Some(2));
        assert_eq!(arena_at(980_010_201), None);
        assert_eq!(battle_of(980_010_101), Some(1));
        assert_eq!(battle_of(ARIANT_LOBBY), None);
    }

    #[test]
    fn a_point_for_every_ten_jewels_but_one_for_a_winner_far_ahead() {
        // Close enough: everyone's tens.
        assert_eq!(
            points(&[(1, 30), (2, 25), (3, 9)], 0),
            [(1, 3), (2, 2), (3, 0)]
        );
        // The others too far behind: the winner gets 1.
        assert_eq!(points(&[(1, 5), (2, 40)], 0), [(1, 0), (2, 1)]);
        // What was lost counts for the others.
        assert_eq!(points(&[(1, 5), (2, 40)], 30), [(1, 0), (2, 4)]);
        // A tie goes to the first with the most.
        assert_eq!(points(&[(1, 20), (2, 20)], 0), [(1, 2), (2, 2)]);
        // Nobody with any: nothing to judge.
        assert_eq!(points(&[(1, 0), (2, 0)], 0), [(1, 0), (2, 0)]);
        assert!(points(&[], 3).is_empty());
        // Alone, all the winner's are theirs only if lost ones were many.
        assert_eq!(points(&[(1, 30)], 0), [(1, 1)]);
        assert_eq!(points(&[(1, 30)], 25), [(1, 3)]);
    }

    #[test]
    fn the_scores_keep_who_told_first_first_and_count_what_was_lost() {
        let mut scores = Scores::default();
        assert!(scores.hold(7, "Seven", 3, 0));
        assert!(scores.hold(2, "Two", 0, 0));
        assert!(!scores.hold(7, "Seven", 3, 0));
        assert!(scores.hold(7, "Seven", 1, 2));
        assert_eq!(
            scores.board(),
            [("Seven".to_string(), 1), ("Two".to_string(), 0)]
        );
        // Lost twice over, and the floor's: 1 against 9 lost is fair.
        assert!(!scores.hold(2, "Two", 0, 3));
        assert_eq!(scores.points(4), [(7, 0), (2, 0)]);
    }

    #[test]
    fn unfair_needs_both_a_small_rest_and_a_far_second() {
        // Two thirds of the winner's, under 7 in all.
        assert!(unfair(6, 3, 0, 4));
        // Over 7 and the next best over 0.5929 of the winner's.
        assert!(!unfair(10, 6, 0, 8));
        // Over 7 but the next best far behind.
        assert!(unfair(20, 5, 0, 10));
        // The rest at least 0.8177 of the winner's.
        assert!(!unfair(10, 2, 0, 9));
    }

    #[test]
    fn aredas_exp_is_92_7_a_point_at_the_rate() {
        assert_eq!(prize_exp(3, 1.0), 278);
        assert_eq!(prize_exp(2, 2.0), 370);
        assert_eq!(prize_exp(0, 1.0), 0);
    }

    #[test]
    fn the_cap_reads_like_parse_int_from_two_to_seven() {
        assert_eq!(cap("3"), Ok(3));
        assert_eq!(cap(" 4 people"), Ok(4));
        assert_eq!(cap("+5"), Ok(5));
        assert_eq!(cap("12"), Ok(7));
        assert_eq!(cap("99999999999999999999999"), Ok(7));
        assert_eq!(cap("1"), Err(CapRefusal::TooFew));
        assert_eq!(cap("-3"), Err(CapRefusal::TooFew));
        assert_eq!(cap("two"), Err(CapRefusal::NotANumber));
        assert_eq!(cap(""), Err(CapRefusal::NotANumber));
        assert_eq!(cap("-"), Err(CapRefusal::NotANumber));
    }
}
