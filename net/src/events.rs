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
    /// The EXP every member gets for clearing each stage, in order. Stage n
    /// is played on `entry + n - 1`.
    pub exp_per_stage: &'static [i32],
    /// The stage maps whose `next00` portal stays shut until their stage is
    /// cleared. Every stage map has a `gate` object, shown once its stage is
    /// cleared.
    pub gated: RangeInclusive<i32>,
    /// The stages cleared by standing on their maps' areas (`on_areas`).
    pub puzzles: RangeInclusive<u8>,
    /// Items of use only in the event, taken from whoever leaves it.
    pub items: &'static [i32],
}

/// Kerning City's party quest (Cosmic `KerningPQ`, Server.NX
/// `FieldSet.img/Party1`: maps 103000800-805, 30 minutes). Stage EXP is
/// Cosmic's; `next00` leads on from 103000800-803, and 800-804 each have a
/// `gate` object (Map.nx). Stages 2 to 4 are Cloto's ropes, platforms and
/// barrels.
pub const KPQ: Event = Event {
    lobby: 103_000_000,
    maps: 103_000_800..=103_000_805,
    entry: 103_000_800,
    exit: 103_000_890,
    levels: 21..=30,
    team: 3..=4,
    minutes: 30,
    exp_per_stage: &[100, 200, 400, 800, 1500],
    gated: 103_000_800..=103_000_803,
    puzzles: 2..=4,
    items: &[COUPON, PASS],
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

    /// The map stage `stage` (from 1) is played on.
    pub fn stage_map(&self, stage: u8) -> i32 {
        self.entry + i32::from(stage) - 1
    }

    /// The stage played on `map_id`, if any.
    pub fn stage_on(&self, map_id: i32) -> Option<u8> {
        let stage = u8::try_from(map_id.checked_sub(self.entry)? + 1).ok()?;
        (1..=self.exp_per_stage.len())
            .contains(&usize::from(stage))
            .then_some(stage)
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
    /// A member other than the leader on a puzzle stage, which is the
    /// leader's to answer.
    Follow,
    /// The leader, on first talking to her on puzzle stage `stage`: she sets
    /// the puzzle.
    Intro { stage: u8 },
    /// The leader, answering puzzle stage `stage`: the team must stand as
    /// `combo` asks (`on_areas`).
    Puzzle { stage: u8, combo: u32 },
    /// The leader, who needs a pass from each of the others.
    Leader { needed: i32 },
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

/// A combo for a puzzle stage with `areas` areas: random, or the one
/// `RUSTMS_KPQ_COMBO` (from 0, in `combos`' order) names for testing.
/// `None` for a map without enough areas.
pub fn draw_combo(areas: usize) -> Option<u32> {
    static PINNED: OnceLock<Option<usize>> = OnceLock::new();
    let pinned = *PINNED.get_or_init(|| {
        std::env::var("RUSTMS_KPQ_COMBO")
            .ok()
            .and_then(|combo| combo.parse::<usize>().ok())
    });
    let count = combos(areas).count();
    if count == 0 {
        return None;
    }
    let index = pinned
        .filter(|index| *index < count)
        .unwrap_or_else(|| rand::random::<usize>() % count);
    combos(areas).nth(index)
}

/// How many of a stage's areas the team must stand on, on Kerning City's
/// party quest's stages 2 to 4 (Cosmic `9020001`): one member on each.
pub const AREAS_TAKEN: u32 = 3;

/// Each way of choosing `AREAS_TAKEN` of `areas` areas, as a bit per area
/// (bit 0 for the first), in a fixed order: 4 for stage 2's ropes, 10 for
/// stage 3's platforms and 20 for stage 4's barrels.
pub fn combos(areas: usize) -> impl Iterator<Item = u32> {
    let all = 1u32.checked_shl(areas as u32).unwrap_or(0);
    (0..all).filter(|combo| combo.count_ones() == AREAS_TAKEN)
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
        assert_eq!(draw_combo(2), None);
        assert!(draw_combo(4).is_some_and(|combo| combos(4).any(|c| c == combo)));
    }

    #[test]
    fn each_puzzle_stage_has_a_combo_for_every_three_of_its_areas() {
        for (areas, count) in [(4, 4), (5, 10), (6, 20)] {
            let all: Vec<u32> = combos(areas).collect();
            assert_eq!(all.len(), count);
            assert!(all.iter().all(|c| c.count_ones() == 3 && *c < 1 << areas));
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
        assert_eq!(stage.talk(1, &team, || 0), Cloto::Leader { needed: 3 });
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
