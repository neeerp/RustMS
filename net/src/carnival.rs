//! The Monster Carnival: Spiegelmann's six rooms off the Carnival Field,
//! where parties race for Carnival Points (CP) by hunting each other's
//! monsters (`agent_docs/carnival.md`; Cosmic `MonsterCarnival.java`).
//! Pure rules; the event actor keeps a battle's score.

use crate::ClientId;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::time::Duration;

/// The Carnival Field, where Spiegelmann (2042000) sends parties to the
/// rooms. Its `out00` (`mc_out`) leads back to where the player came from.
pub const CARNIVAL_LOBBY: i32 = 980000000;
/// The room everyone goes to once a battle is over, with 2042002.
pub const CARNIVAL_EXIT: i32 = 980000010;
/// The rooms, 1 to 4 for two to four a side, 5 and 6 for three to six.
pub const CARNIVAL_ROOMS: u8 = 6;
/// Who may go to the Carnival Field and play.
pub const CARNIVAL_LEVELS: RangeInclusive<u8> = 30..=50;
/// Where `mc_out` goes when no way back was saved (Kerning City).
pub const CARNIVAL_FALLBACK: i32 = 102000000;
/// A battle's length when its field's data has none (`timeDefault`).
const BATTLE_SECONDS: u64 = 610;

/// Room `room`'s waiting room (980000n00). The battle is the next map,
/// then the revive room, the winners' room and the losers'.
pub const fn room_map(room: u8) -> i32 {
    CARNIVAL_LOBBY + room as i32 * 100
}

/// Room `room`'s battle field (980000n01).
pub const fn battle_map(room: u8) -> i32 {
    room_map(room) + 1
}

/// The room whose battle field `map_id` is.
pub fn battle_room(map_id: i32) -> Option<u8> {
    (1..=CARNIVAL_ROOMS).find(|&room| battle_map(room) == map_id)
}

/// How long room `room`'s battle lasts: its field's `timeDefault`,
/// shortened like the events' time (`crate::events::scaled`).
pub fn battle_length(room: u8) -> Duration {
    let seconds = crate::get_game_data()
        .ok()
        .and_then(|data| data.field(battle_map(room))?.carnival.as_ref())
        .and_then(|rules| u64::try_from(rules.time_default).ok())
        .filter(|&seconds| seconds > 0)
        .unwrap_or(BATTLE_SECONDS);
    crate::events::scaled(Duration::from_secs(seconds))
}

/// How many mobs room `room`'s battle field lists to summon.
pub fn summonable(room: u8) -> usize {
    crate::get_game_data()
        .ok()
        .and_then(|data| Some(data.field(battle_map(room))?.carnival.as_ref()?.mobs.len()))
        .unwrap_or(0)
}

/// Carnival Points: those left to spend, and all ever earned, which
/// decide the battle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Points {
    pub cp: i32,
    pub total: i32,
}

impl Points {
    fn gain(&mut self, cp: i32) {
        self.cp = self.cp.saturating_add(cp);
        self.total = self.total.saturating_add(cp);
    }
}

/// What a member's screen shows as they arrive on the battle field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Start {
    /// 0 for Maple Red, 1 for Maple Blue.
    pub side: u8,
    pub mine: Points,
    pub ours: Points,
    pub theirs: Points,
}

/// A kill's CP, as the screens show it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gain {
    pub side: u8,
    pub mine: Points,
    pub ours: Points,
}

/// A battle's score: each member's side and points, and each side's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Score {
    sides: HashMap<ClientId, u8>,
    members: HashMap<ClientId, Points>,
    teams: [Points; 2],
}

impl Score {
    /// `member` plays for `side` (0 or 1).
    pub fn join(&mut self, member: ClientId, side: u8) {
        self.sides.insert(member, side.min(1));
        self.members.entry(member).or_default();
    }

    pub fn side(&self, member: ClientId) -> Option<u8> {
        self.sides.get(&member).copied()
    }

    /// `member` earned `cp` with a kill, and so did their side (Cosmic
    /// `gainCP`). None for one not playing.
    pub fn gain(&mut self, member: ClientId, cp: i32) -> Option<Gain> {
        let side = self.side(member)?;
        let mine = self.members.entry(member).or_default();
        mine.gain(cp);
        let ours = &mut self.teams[usize::from(side)];
        ours.gain(cp);
        Some(Gain {
            side,
            mine: *mine,
            ours: *ours,
        })
    }

    /// What `member`'s screen shows on arrival. None for one not playing.
    pub fn start(&self, member: ClientId) -> Option<Start> {
        let side = self.side(member)?;
        Some(Start {
            side,
            mine: self.members.get(&member).copied().unwrap_or_default(),
            ours: self.teams[usize::from(side)],
            theirs: self.teams[usize::from(1 - side)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rooms_battle_fields_are_their_second_maps() {
        assert_eq!(room_map(1), 980000100);
        assert_eq!(battle_map(1), 980000101);
        assert_eq!(battle_map(6), 980000601);
        assert_eq!(battle_room(980000401), Some(4));
        assert_eq!(battle_room(980000400), None);
        assert_eq!(battle_room(980000701), None);
        assert_eq!(battle_room(CARNIVAL_LOBBY), None);
    }

    #[test]
    fn a_kill_earns_cp_for_the_killer_and_their_side() {
        let mut score = Score::default();
        score.join(1, 0);
        score.join(2, 0);
        score.join(3, 1);
        assert_eq!(score.gain(4, 5), None);
        let gain = score.gain(1, 2).unwrap();
        assert_eq!(gain.side, 0);
        assert_eq!(gain.mine, Points { cp: 2, total: 2 });
        assert_eq!(gain.ours, Points { cp: 2, total: 2 });
        score.gain(2, 3);
        score.gain(3, 1);
        let start = score.start(2).unwrap();
        assert_eq!(start.mine, Points { cp: 3, total: 3 });
        assert_eq!(start.ours, Points { cp: 5, total: 5 });
        assert_eq!(start.theirs, Points { cp: 1, total: 1 });
        assert_eq!(score.start(3).unwrap().theirs, start.ours);
        assert_eq!(score.start(9), None);
    }
}
