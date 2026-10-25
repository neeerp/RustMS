//! The Monster Carnival: Spiegelmann's six rooms off the Carnival Field,
//! where parties race for Carnival Points (CP) by hunting each other's
//! monsters (`agent_docs/carnival.md`; Cosmic `MonsterCarnival.java`).
//! Pure rules; the event actor keeps a battle's score.

use crate::ClientId;
use game_data::skills::CarnivalSkill;
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

/// Room `room`'s revive room, where its battle field's dead go back to
/// life (the field's `returnMap`, fieldType 11).
pub const fn revive_map(room: u8) -> i32 {
    room_map(room) + 2
}

/// The room whose revive room `map_id` is.
pub fn revive_room(map_id: i32) -> Option<u8> {
    (1..=CARNIVAL_ROOMS).find(|&room| revive_map(room) == map_id)
}

/// The most CP a death on room `room`'s battle field costs (its
/// `deathCP`).
pub fn death_cp(room: u8) -> i32 {
    crate::get_game_data()
        .ok()
        .and_then(|data| data.field(battle_map(room))?.carnival.as_ref())
        .map_or(0, |rules| rules.death_cp.max(0))
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

/// The Monster Carnival skill `id` (`MCSkill.img`).
pub fn carnival_skill(id: i32) -> Option<CarnivalSkill> {
    carnival_skills().get(usize::try_from(id).ok()?).copied()
}

fn carnival_skills() -> &'static [CarnivalSkill] {
    static SKILLS: std::sync::OnceLock<Vec<CarnivalSkill>> = std::sync::OnceLock::new();
    SKILLS.get_or_init(|| {
        crate::skills::data()
            .ok()
            .and_then(|data| data.carnival_skills().ok())
            .unwrap_or_default()
    })
}

/// How many guardians a side may have standing at once where the field
/// doesn't say (`guardianGenMax`; Cosmic's default).
pub const GUARDIAN_GEN_MAX: i32 = 16;

/// What room `room`'s battle field sells at `index` of its guardian list:
/// the guardian, and how many of a side's may stand at once.
pub fn guardian(room: u8, index: u8) -> Option<(CarnivalSkill, usize)> {
    static GUARDIANS: std::sync::OnceLock<Vec<CarnivalSkill>> = std::sync::OnceLock::new();
    let guardians = GUARDIANS.get_or_init(|| {
        crate::skills::data()
            .ok()
            .and_then(|data| data.carnival_guardians().ok())
            .unwrap_or_default()
    });
    let data = crate::get_game_data().ok()?;
    let rules = data.field(battle_map(room))?.carnival.as_ref()?;
    let id = *rules.guardians.get(usize::from(index))?;
    let guardian = *guardians.get(usize::try_from(id).ok()?)?;
    let cap = rules.guardian_gen_max.unwrap_or(GUARDIAN_GEN_MAX).max(0) as usize;
    Some((guardian, cap))
}

/// The state a guardian stands in when placed: its reactor's first state
/// waits for nothing, so Cosmic's `spawnGuardian` hits it once. Four more
/// hits break it.
pub const GUARDIAN_STATE: u8 = 1;

/// The `guardianGenPos` spot, of `spots`, where side `side` places a
/// guardian with mob skill `mob_skill`, `standing` being the guardians
/// that stand (side, mob skill, spot): the `roll`th free one of those for
/// the side or for either. None if the side has `cap` standing (code 2)
/// or that one (4), or no spot is free (3) (Cosmic `spawnGuardian`, whose
/// spots stay taken after a guardian breaks; here they are freed).
pub fn guardian_spot(
    spots: &[game_data::CarnivalSpot],
    standing: &[(u8, i32, usize)],
    side: u8,
    mob_skill: i32,
    cap: usize,
    roll: usize,
) -> Result<usize, SpendRefusal> {
    let ours: Vec<_> = standing.iter().filter(|&&(of, _, _)| of == side).collect();
    if ours.len() >= cap {
        return Err(SpendRefusal::Limit);
    }
    if ours.iter().any(|&&(_, skill, _)| skill == mob_skill) {
        return Err(SpendRefusal::Active);
    }
    let free: Vec<usize> = (0..spots.len())
        .filter(|&spot| spots[spot].team.is_none_or(|team| team == side))
        .filter(|&spot| !standing.iter().any(|&(_, _, taken)| taken == spot))
        .collect();
    free.get(roll % free.len().max(1))
        .copied()
        .ok_or(SpendRefusal::NoSpot)
}

/// The guardian reactor a side places: Maple Red's 9980000, Maple Blue's
/// 9980001 (Reactor.nx, "Monster Carnival Artifact").
pub const fn guardian_reactor(side: u8) -> i32 {
    9_980_000 + side as i32
}

/// What room `room`'s battle field sells at `index` of its skill list.
pub fn skill(room: u8, index: u8) -> Option<CarnivalSkill> {
    let data = crate::get_game_data().ok()?;
    let rules = data.field(battle_map(room))?.carnival.as_ref()?;
    carnival_skill(*rules.skills.get(usize::from(index))?)
}

/// The skill `skill` casts: itself, or for a Random one (no mob skill),
/// the `roll`th of the others that reach as many (Cosmic
/// `MCSkill.getSkill`'s `randomizeSkill`).
pub fn cast(skill: CarnivalSkill, roll: usize) -> Option<CarnivalSkill> {
    if skill.mob_skill != 0 {
        return Some(skill);
    }
    let pool: Vec<_> = carnival_skills()
        .iter()
        .filter(|other| other.mob_skill != 0 && other.all == skill.all)
        .collect();
    pool.get(roll % pool.len().max(1)).copied().copied()
}

/// Whether mob skill `mob_skill` cast with a Carnival skill takes hold,
/// for `roll` in 0..1: darkness, weakness, poison and slow only do 80% of
/// the time, as their descriptions say (Cosmic `MonsterCarnivalHandler`).
pub fn takes_hold(mob_skill: i32, roll: f64) -> bool {
    !matches!(mob_skill, 121 | 122 | 125 | 126) || roll < 0.8
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

    /// `member` died, and loses up to `most` of their CP, as does their
    /// side; the totals stay (Cosmic `playerDead`'s `gainCP(-losing)`).
    /// The CP lost, and the points now. None for one not playing.
    pub fn lose(&mut self, member: ClientId, most: i32) -> Option<(i32, Gain)> {
        let side = self.side(member)?;
        let mine = self.members.entry(member).or_default();
        let lost = most.clamp(0, mine.cp.max(0));
        mine.spend(lost);
        let mine = *mine;
        let ours = &mut self.teams[usize::from(side)];
        ours.spend(lost);
        Some((
            lost,
            Gain {
                side,
                mine,
                ours: *ours,
            },
        ))
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
    fn a_guardian_takes_a_free_spot_of_its_side() {
        use game_data::CarnivalSpot;
        let spot = |team| CarnivalSpot {
            team,
            ..CarnivalSpot::default()
        };
        let spots = [spot(Some(1)), spot(None), spot(Some(0)), spot(Some(0))];
        // Red's spots are the 2nd to 4th; the 3rd is taken.
        let standing = [(1, 150, 2)];
        assert_eq!(guardian_spot(&spots, &standing, 0, 150, 8, 0), Ok(1));
        assert_eq!(guardian_spot(&spots, &standing, 0, 150, 8, 1), Ok(3));
        assert_eq!(guardian_spot(&spots, &standing, 0, 150, 8, 2), Ok(1));
        // Blue has Power UP already, and as many as it may.
        assert_eq!(
            guardian_spot(&spots, &standing, 1, 150, 8, 0),
            Err(SpendRefusal::Active)
        );
        assert_eq!(
            guardian_spot(&spots, &standing, 1, 151, 1, 0),
            Err(SpendRefusal::Limit)
        );
        // Blue's own spot and the shared one are taken.
        let standing = [(1, 150, 0), (0, 151, 1)];
        assert_eq!(
            guardian_spot(&spots, &standing, 1, 152, 8, 0),
            Err(SpendRefusal::NoSpot)
        );
        assert!(guardian(1, 6).is_some_and(|(guardian, cap)| {
            (guardian.cost, guardian.mob_skill, cap) == (12, 156, 8)
        }));
    }

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
        assert_eq!(revive_map(4), 980000402);
        assert_eq!(revive_room(980000602), Some(6));
        assert_eq!(revive_room(980000601), None);
        assert_eq!(death_cp(1), 10);
    }

    #[test]
    fn a_death_costs_cp_the_member_has() {
        let mut score = Score::default();
        score.join(1, 0);
        score.join(2, 0);
        assert_eq!(score.lose(3, 10), None);
        score.gain(1, 12);
        score.gain(2, 4);
        let (lost, gain) = score.lose(1, 10).unwrap();
        assert_eq!(lost, 10);
        assert_eq!(gain.mine, Points { cp: 2, total: 12 });
        assert_eq!(gain.ours, Points { cp: 6, total: 16 });
        // One with less loses what they have; one with none, nothing.
        assert_eq!(score.lose(1, 10).unwrap().0, 2);
        let (lost, gain) = score.lose(1, 10).unwrap();
        assert_eq!(lost, 0);
        assert_eq!(gain.ours, Points { cp: 4, total: 16 });
    }

    #[test]
    fn some_skills_take_hold_four_times_in_five() {
        assert!(takes_hold(124, 0.99) && takes_hold(127, 0.99));
        assert!(takes_hold(121, 0.79) && !takes_hold(121, 0.8));
        assert!(!takes_hold(126, 0.9));
        let curse = CarnivalSkill {
            cost: 12,
            mob_skill: 124,
            level: 7,
            all: true,
        };
        assert_eq!(cast(curse, 5), Some(curse));
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
