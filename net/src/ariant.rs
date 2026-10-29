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
