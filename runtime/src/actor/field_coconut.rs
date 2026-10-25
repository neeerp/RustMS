//! A Coconut Season game on one field (field type 4, report b167): the two
//! teams, the score, the nuts and the clock. Pure state; `FieldActor` sends
//! the packets.

use game_data::Coconut;
use net::ClientId;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

/// Team Maple, on the scoreboard's left.
pub const MAPLE: u8 = 0;
/// Team Story, on its right.
pub const STORY: u8 = 1;

/// Where the winners go when the game is over (Cosmic `EVENT_WINNER`).
pub const WINNERS_MAP: i32 = 109_050_000;
/// Where everyone else goes (the fields' forcedReturn).
pub const EXIT_MAP: i32 = 109_050_001;

/// How long after the start the nuts show (Cosmic's -1/5000/0).
pub const SHOW_DELAY: u16 = 5000;
/// The delay of every other nut's state.
pub const HIT_DELAY: u16 = 1000;
/// How long a nut that shook takes no more hits.
const SHAKE_TIME: Duration = Duration::from_millis(750);

/// What a hit did to a nut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    /// It shook, and takes more.
    Shake,
    /// It shook and stays up for good (`countStopped`).
    Stuck,
    /// It burst (`countBombing`), scoring nothing.
    Burst,
    /// It fell (`countFalling`), a point for `team`.
    Fell { team: u8 },
}

impl Shot {
    /// The state the client shows the nut in: s1 shakes, s2 bursts, s3
    /// falls (Map.nx `Obj/event.img/sandBeach/coconut`).
    pub fn state(self) -> u8 {
        match self {
            Shot::Shake | Shot::Stuck => 1,
            Shot::Burst => 2,
            Shot::Fell { .. } => 3,
        }
    }
}

/// What the time did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// A tie at the end plays on this long, once.
    Overtime(Duration),
    /// The game is over: `winner` won, or nobody did.
    Ended { winner: Option<u8> },
    /// Time to send everyone off: `winner`'s players to `WINNERS_MAP`.
    WarpOut { winner: Option<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Playing { ends: Instant, overtime: bool },
    Finishing { at: Instant, winner: Option<u8> },
}

#[derive(Clone, Copy, Debug, Default)]
struct Nut {
    hits: u8,
    /// How it came down, once it has.
    down: Option<Shot>,
    next_hit: Option<Instant>,
}

pub struct CoconutGame {
    rules: Coconut,
    teams: BTreeMap<ClientId, u8>,
    phase: Phase,
    scores: [u16; 2],
    nuts: Vec<Nut>,
    falling: u32,
    bombing: u32,
    stopped: u32,
}

impl CoconutGame {
    pub fn new(rules: Coconut) -> Self {
        Self {
            rules,
            teams: BTreeMap::new(),
            phase: Phase::Idle,
            scores: [0; 2],
            nuts: Vec::new(),
            falling: 0,
            bombing: 0,
            stopped: 0,
        }
    }

    pub fn rules(&self) -> &Coconut {
        &self.rules
    }

    /// Maple's and Story's points.
    pub fn scores(&self) -> [u16; 2] {
        self.scores
    }

    pub fn team(&self, client_id: ClientId) -> Option<u8> {
        self.teams.get(&client_id).copied()
    }

    /// A player arriving joins the team with fewer players, Maple on a tie,
    /// and keeps it until they leave.
    pub fn join(&mut self, client_id: ClientId) -> u8 {
        if let Some(team) = self.team(client_id) {
            return team;
        }
        let story = self.teams.values().filter(|&&team| team == STORY).count();
        let team = if story < self.teams.len() - story {
            STORY
        } else {
            MAPLE
        };
        self.teams.insert(client_id, team);
        team
    }

    pub fn leave(&mut self, client_id: ClientId) {
        self.teams.remove(&client_id);
    }

    pub fn playing(&self) -> bool {
        matches!(self.phase, Phase::Playing { .. })
    }

    /// Whether a game is on or ending, its nuts out.
    pub fn on(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// The nuts that burst or fell, with the state they show: what a player
    /// arriving mid-game is sent to see them as the others do. A stuck nut
    /// looks like any other.
    pub fn downed(&self) -> impl Iterator<Item = (u16, u8)> + '_ {
        self.nuts.iter().enumerate().filter_map(|(index, nut)| {
            let shot = nut.down.filter(|shot| *shot != Shot::Stuck)?;
            Some((u16::try_from(index).ok()?, shot.state()))
        })
    }

    /// The seconds left on the clock while the game is on.
    pub fn remaining(&self, now: Instant) -> Option<u32> {
        match self.phase {
            Phase::Playing { ends, .. } => Some(seconds(ends.saturating_duration_since(now))),
            _ => None,
        }
    }

    /// A GM starts the game, for `length`; not while one is on or ending.
    pub fn start(&mut self, now: Instant, length: Duration) -> bool {
        if self.phase != Phase::Idle {
            return false;
        }
        self.phase = Phase::Playing {
            ends: now + length,
            overtime: false,
        };
        self.scores = [0; 2];
        self.nuts = vec![Nut::default(); self.rules.nuts() as usize];
        self.falling = self.rules.count_falling;
        self.bombing = self.rules.count_bombing;
        self.stopped = self.rules.count_stopped;
        true
    }

    /// A GM calls the game off; whether one was on.
    pub fn reset(&mut self) -> bool {
        let was = self.phase != Phase::Idle;
        self.phase = Phase::Idle;
        self.scores = [0; 2];
        was
    }

    /// `client_id` hits nut `index` (Cosmic's rules, QUESTIONS b169 #4):
    /// after more than two hits, 40% of hits bring it down, which sticks it
    /// 1% of the time, bursts it 5% and drops it otherwise, while each kind
    /// is left. `roll` draws from [0, 1).
    pub fn hit(
        &mut self,
        client_id: ClientId,
        index: u16,
        now: Instant,
        roll: &mut dyn FnMut() -> f64,
    ) -> Option<Shot> {
        let team = self.team(client_id)?;
        if !matches!(self.phase, Phase::Playing { ends, .. } if now < ends) {
            return None;
        }
        let nut = self.nuts.get_mut(usize::from(index))?;
        if nut.down.is_some() || nut.next_hit.is_some_and(|at| now < at) {
            return None;
        }
        if nut.hits <= 2 || roll() >= 0.4 {
            nut.hits = nut.hits.saturating_add(1);
            nut.next_hit = Some(now + SHAKE_TIME);
            return Some(Shot::Shake);
        }
        // Every nut ends one of the three ways: the counts add up to them.
        let shot = if self.stopped > 0 && roll() < 0.01 {
            Shot::Stuck
        } else if self.bombing > 0 && roll() < 0.05 {
            Shot::Burst
        } else if self.falling > 0 {
            Shot::Fell { team }
        } else if self.bombing > 0 {
            Shot::Burst
        } else {
            Shot::Stuck
        };
        nut.down = Some(shot);
        match shot {
            Shot::Stuck => self.stopped = self.stopped.saturating_sub(1),
            Shot::Burst => self.bombing -= 1,
            Shot::Fell { team } => {
                self.falling -= 1;
                let score = &mut self.scores[usize::from(team)];
                *score = score.saturating_add(1);
            }
            Shot::Shake => {}
        }
        Some(shot)
    }

    /// What the time has done by `now`. A tie at the end plays `overtime`
    /// once; the warp out comes `finish` after the end.
    pub fn tick(&mut self, now: Instant, overtime: Duration, finish: Duration) -> Option<Turn> {
        match self.phase {
            Phase::Playing {
                ends,
                overtime: over,
            } if now >= ends => {
                let [maple, story] = self.scores;
                if maple == story && !over {
                    self.phase = Phase::Playing {
                        ends: now + overtime,
                        overtime: true,
                    };
                    return Some(Turn::Overtime(overtime));
                }
                let winner = match maple.cmp(&story) {
                    std::cmp::Ordering::Greater => Some(MAPLE),
                    std::cmp::Ordering::Less => Some(STORY),
                    std::cmp::Ordering::Equal => None,
                };
                self.phase = Phase::Finishing {
                    at: now + finish,
                    winner,
                };
                Some(Turn::Ended { winner })
            }
            Phase::Finishing { at, winner } if now >= at => {
                self.phase = Phase::Idle;
                self.scores = [0; 2];
                Some(Turn::WarpOut { winner })
            }
            _ => None,
        }
    }
}

/// Whole seconds, rounded up, so the clock never shows 0 early.
pub fn seconds(time: Duration) -> u32 {
    let millis = time.as_millis();
    u32::try_from(millis.div_ceil(1000)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Coconut {
        Coconut {
            count_falling: 2,
            count_bombing: 1,
            count_stopped: 1,
            time_default: 300,
            time_expand: 120,
            time_finish: 12,
            ..Coconut::default()
        }
    }

    const MINUTE: Duration = Duration::from_secs(60);

    fn started() -> (CoconutGame, Instant) {
        let mut game = CoconutGame::new(rules());
        game.join(1);
        game.join(2);
        let now = Instant::now();
        assert!(game.start(now, 5 * MINUTE));
        (game, now)
    }

    /// Hits nut `index` until it comes down, the rolls for what it does
    /// giving `value`.
    fn bring_down(
        game: &mut CoconutGame,
        who: ClientId,
        index: u16,
        at: Instant,
        value: f64,
    ) -> Shot {
        let mut now = at;
        loop {
            let mut rolls = [0.0, value, value].into_iter();
            let shot = game
                .hit(who, index, now, &mut || rolls.next().unwrap())
                .expect("hit");
            if shot != Shot::Shake {
                return shot;
            }
            now += SHAKE_TIME;
        }
    }

    #[test]
    fn players_split_into_the_smaller_team() {
        let mut game = CoconutGame::new(rules());
        assert_eq!(game.join(1), MAPLE);
        assert_eq!(game.join(2), STORY);
        assert_eq!(game.join(3), MAPLE);
        // A rejoin keeps the team.
        assert_eq!(game.join(1), MAPLE);
        game.leave(1);
        game.leave(3);
        assert_eq!(game.join(4), MAPLE);
        assert_eq!(game.join(5), MAPLE);
        assert_eq!(game.join(6), STORY);
    }

    #[test]
    fn a_nut_shakes_three_times_then_falls_for_the_hitter() {
        let (mut game, now) = started();
        let mut at = now;
        for _ in 0..3 {
            assert_eq!(game.hit(2, 3, at, &mut || 0.0), Some(Shot::Shake));
            // It takes no hit while it shakes.
            assert_eq!(game.hit(1, 3, at + SHAKE_TIME / 2, &mut || 0.0), None);
            at += SHAKE_TIME;
        }
        // More than two hits: 0.5 still shakes it; 0.3 brings it down,
        // neither stuck nor burst.
        assert_eq!(game.hit(2, 3, at, &mut || 0.5), Some(Shot::Shake));
        assert_eq!(
            game.hit(2, 3, at + SHAKE_TIME, &mut || 0.3),
            Some(Shot::Fell { team: STORY })
        );
        assert_eq!(game.scores(), [0, 1]);
        // A fallen nut is gone.
        assert_eq!(game.hit(1, 3, at + MINUTE, &mut || 0.0), None);
    }

    #[test]
    fn the_counts_run_out_in_turn() {
        let (mut game, now) = started();
        // 0.0 sticks while a stuck nut is left, then bursts, then falls.
        assert_eq!(bring_down(&mut game, 1, 0, now, 0.0), Shot::Stuck);
        assert_eq!(bring_down(&mut game, 1, 1, now, 0.0), Shot::Burst);
        assert_eq!(
            bring_down(&mut game, 1, 2, now, 0.0),
            Shot::Fell { team: MAPLE }
        );
        assert_eq!(
            bring_down(&mut game, 2, 3, now, 0.0),
            Shot::Fell { team: STORY }
        );
        assert_eq!(game.scores(), [1, 1]);
        assert_eq!(game.downed().collect::<Vec<_>>(), [(1, 2), (2, 3), (3, 3)]);
        // Four nuts, all down; there is no fifth.
        assert_eq!(game.hit(1, 4, now, &mut || 0.0), None);
    }

    #[test]
    fn with_no_falls_left_a_nut_bursts_or_sticks() {
        let mut game = CoconutGame::new(Coconut {
            count_falling: 0,
            ..rules()
        });
        game.join(1);
        let now = Instant::now();
        game.start(now, MINUTE);
        assert_eq!(bring_down(&mut game, 1, 0, now, 0.9), Shot::Burst);
        assert_eq!(bring_down(&mut game, 1, 1, now, 0.9), Shot::Stuck);
    }

    #[test]
    fn nothing_counts_out_of_the_game() {
        let mut game = CoconutGame::new(rules());
        game.join(1);
        let now = Instant::now();
        // Before the start.
        assert_eq!(game.hit(1, 0, now, &mut || 0.0), None);
        game.start(now, MINUTE);
        // Someone not on a team, a nut that isn't there, and after the end.
        assert_eq!(game.hit(9, 0, now, &mut || 0.0), None);
        assert_eq!(game.hit(1, 500, now, &mut || 0.0), None);
        assert_eq!(game.hit(1, 0, now + MINUTE, &mut || 0.0), None);
        assert_eq!(game.hit(1, 0, now, &mut || 0.0), Some(Shot::Shake));
    }

    #[test]
    fn a_tie_plays_on_once_then_is_a_draw() {
        let (mut game, now) = started();
        let (over, finish) = (2 * MINUTE, Duration::from_secs(12));
        assert_eq!(game.tick(now, over, finish), None);
        assert_eq!(game.remaining(now + MINUTE), Some(240));
        let end = now + 5 * MINUTE;
        assert_eq!(game.tick(end, over, finish), Some(Turn::Overtime(over)));
        assert_eq!(game.remaining(end), Some(120));
        assert!(!game.start(end, MINUTE));
        let end = end + over;
        assert_eq!(
            game.tick(end, over, finish),
            Some(Turn::Ended { winner: None })
        );
        assert!(!game.playing());
        assert_eq!(game.tick(end + finish / 2, over, finish), None);
        assert_eq!(
            game.tick(end + finish, over, finish),
            Some(Turn::WarpOut { winner: None })
        );
        // It may be played again.
        assert!(game.start(end + finish, MINUTE));
    }

    #[test]
    fn the_side_ahead_wins() {
        let (mut game, now) = started();
        bring_down(&mut game, 2, 0, now, 0.9);
        let (over, finish) = (2 * MINUTE, Duration::from_secs(12));
        assert_eq!(
            game.tick(now + 5 * MINUTE, over, finish),
            Some(Turn::Ended {
                winner: Some(STORY)
            })
        );
    }

    #[test]
    fn a_reset_calls_the_game_off() {
        let (mut game, now) = started();
        bring_down(&mut game, 1, 0, now, 0.9);
        assert!(game.reset());
        assert_eq!(game.scores(), [0, 0]);
        assert!(!game.playing() && !game.reset());
        assert_eq!(game.tick(now + 10 * MINUTE, MINUTE, MINUTE), None);
        assert_eq!(game.team(1), Some(MAPLE));
    }

    #[test]
    fn the_clock_rounds_up() {
        assert_eq!(seconds(Duration::from_millis(1)), 1);
        assert_eq!(seconds(Duration::from_millis(1000)), 1);
        assert_eq!(seconds(Duration::ZERO), 0);
    }
}
