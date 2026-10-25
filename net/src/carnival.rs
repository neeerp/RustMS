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
/// How long a party waits in its room for a challenge before it is sent
/// back to the Carnival Field (Cosmic's lobby timer).
pub const WAITING: Duration = Duration::from_secs(180);
/// How long both parties stand in the waiting room once a challenge is
/// accepted, before the battle (Cosmic `startCPQ`'s clock).
pub const READY: Duration = Duration::from_secs(10);
/// How long a challenge waits for the waiting leader's answer.
pub const CHALLENGE: Duration = Duration::from_secs(30);

/// Room `room`'s waiting room (980000n00). The battle is the next map,
/// then the revive room, the winners' room and the losers'.
pub const fn room_map(room: u8) -> i32 {
    CARNIVAL_LOBBY + room as i32 * 100
}

/// Room `room`'s battle field (980000n01).
pub const fn battle_map(room: u8) -> i32 {
    room_map(room) + 1
}

/// The room whose waiting room `map_id` is.
pub fn waiting_room(map_id: i32) -> Option<u8> {
    (1..=CARNIVAL_ROOMS).find(|&room| room_map(room) == map_id)
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

/// What Spiegelmann shows of a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Room {
    Free,
    /// A party of `size`, of average level `level`, waits for a challenge.
    Waiting {
        size: usize,
        level: u8,
    },
    /// Two parties are in battle, or about to be.
    Battle,
}

/// Why a party's leader may not take or challenge a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoParty,
    NotLeader,
    /// Too few or too many of the party on the Carnival Field at the right
    /// levels.
    NotEligible,
    /// The party isn't the size of the one waiting.
    Unequal,
    /// The party waiting is already answering another challenge.
    Facing,
    /// The room is in battle.
    Full,
}

/// Where a room's event stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// One party waits in the room for a challenge.
    #[default]
    Waiting,
    /// A challenge was accepted: both parties gather before the battle.
    Ready,
    Battle,
}

/// One of a challenging party, as the waiting leader is shown them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Challenger {
    pub name: String,
    pub level: u8,
}

/// How many mobs room `room`'s battle field lists to summon.
pub fn summonable(room: u8) -> usize {
    crate::get_game_data()
        .ok()
        .and_then(|data| Some(data.field(battle_map(room))?.carnival.as_ref()?.mobs.len()))
        .unwrap_or(0)
}

/// How many of its summons a side may have out at once where the field
/// doesn't say (`mobGenMax`; Cosmic's default).
pub const MOB_GEN_MAX: i32 = 20;

/// What room `room`'s battle field sells at `index` of its mob list: the
/// mob, its CP price, and how many of a side's summons may be out at once.
pub fn summon(room: u8, index: u8) -> Option<(i32, i32, usize)> {
    let data = crate::get_game_data().ok()?;
    let rules = data.field(battle_map(room))?.carnival.as_ref()?;
    let &(mob_id, cost) = rules.mobs.get(usize::from(index))?;
    let cap = rules.mob_gen_max.unwrap_or(MOB_GEN_MAX).max(0) as usize;
    Some((mob_id, cost, cap))
}

/// Why CP couldn't be spent, MONSTER_CARNIVAL_MESSAGE's code (v83ref
/// b163): the client says so in its chat log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpendRefusal {
    /// Not enough CP, or nothing at that place in the list.
    NotEnoughCp = 1,
    /// The side has as many summons out as it may.
    Limit = 2,
    /// No spot is free for a guardian.
    NoSpot = 3,
    /// That guardian is already out.
    Active = 4,
    /// Anything else.
    Failed = 5,
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

    /// Spending takes from what is left, never from the total.
    fn spend(&mut self, cp: i32) {
        self.cp = self.cp.saturating_sub(cp);
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

    /// Whether both sides still have one of `members` playing.
    pub fn both_sides(&self, members: &[ClientId]) -> bool {
        let plays = |side| members.iter().any(|&m| self.side(m) == Some(side));
        plays(0) && plays(1)
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

    /// `member` spends `cp`, and so does their side; the totals stay
    /// (Cosmic `gainCP(-cost)`). None for one not playing or without as
    /// much left.
    pub fn spend(&mut self, member: ClientId, cp: i32) -> Option<Gain> {
        let side = self.side(member)?;
        let mine = self.members.entry(member).or_default();
        if mine.cp < cp {
            return None;
        }
        mine.spend(cp);
        let mine = *mine;
        let ours = &mut self.teams[usize::from(side)];
        ours.spend(cp);
        Some(Gain {
            side,
            mine,
            ours: *ours,
        })
    }

    /// `member` gets back `cp` they spent on what didn't come about.
    pub fn refund(&mut self, member: ClientId, cp: i32) {
        let Some(side) = self.side(member) else {
            return;
        };
        let mine = self.members.entry(member).or_default();
        mine.cp = mine.cp.saturating_add(cp);
        let ours = &mut self.teams[usize::from(side)];
        ours.cp = ours.cp.saturating_add(cp);
    }

    /// `member`'s points and their side's now. None for one not playing.
    pub fn standing(&self, member: ClientId) -> Option<Gain> {
        let side = self.side(member)?;
        Some(Gain {
            side,
            mine: self.members.get(&member).copied().unwrap_or_default(),
            ours: self.teams[usize::from(side)],
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
        assert_eq!(waiting_room(980000300), Some(3));
        assert_eq!(waiting_room(980000301), None);
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

        // Spending takes from what is left, the member's and their side's.
        assert_eq!(score.spend(2, 4), None);
        let spent = score.spend(2, 3).unwrap();
        assert_eq!(spent.mine, Points { cp: 0, total: 3 });
        assert_eq!(spent.ours, Points { cp: 2, total: 5 });
        assert_eq!(score.spend(9, 0), None);
        score.refund(2, 3);
        let back = score.standing(2).unwrap();
        assert_eq!(back.mine, Points { cp: 3, total: 3 });
        assert_eq!(back.ours, Points { cp: 5, total: 5 });
        assert!(score.both_sides(&[1, 3]));
        assert!(!score.both_sides(&[1, 2]));
        assert!(!score.both_sides(&[3, 9]));
    }
}
