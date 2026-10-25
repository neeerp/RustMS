//! A Snowball game on one field (field type 1, report b168). Each team
//! pushes its ball along its lane, and can bring down the snowman in its
//! lane to stop the other team's ball for a while. Pure state;
//! `FieldActor` sends the packets.

pub use crate::actor::field_coconut::{EXIT_MAP, WINNERS_MAP};
use game_data::Snowball;
use net::packet::build::world::snowball::BallState;
use net::ClientId;
use std::collections::{BTreeMap, HashMap, VecDeque};
use tokio::time::{Duration, Instant};

/// Team Story, whose ball rolls along the lower lane (the client's names
/// for team 0 and 1).
pub const STORY: u8 = 0;
/// Team Maple, on the upper lane.
pub const MAPLE: u8 = 1;

/// How long a game runs (Cosmic's clock; the fields give no length).
pub const LENGTH: Duration = Duration::from_secs(600);
/// How long the result shows before everyone is sent off.
pub const FINISH: Duration = Duration::from_secs(10);

/// The hits that roll a ball one step (Cosmic).
const HITS_PER_STEP: u8 = 3;
/// How often a player's hits count (Cosmic).
const HIT_COOLDOWN: Duration = Duration::from_millis(500);
/// How often a player walking into a ball is thrown back. The client asks
/// on every frame it is inside one.
const KNOCK_COOLDOWN: Duration = Duration::from_millis(500);
/// How often a snowman that is up regains `recoveryAmount` (QUESTIONS
/// b168 #3).
const RECOVERY_EVERY: Duration = Duration::from_secs(10);

/// The client's milliseconds a ball takes for each step at each stage
/// (v83 0xbd8ba4). At these the client rolls the ball on, whatever it was
/// last sent: a step sooner when behind, later when ahead, never back.
const STEP_TIMES: [u64; 8] = [150, 200, 250, 300, 350, 400, 450, 500];
/// The next entry of the table, 0: the ball only catches up with the step
/// it was last sent, a step per 500 ms, and goes back one step when ahead.
pub const CATCH_UP: u8 = 8;
/// The longest a hit may wait to show: the hitter's swing.
pub const MAX_HIT_DELAY: u16 = 2000;
/// The span a ball's pace is measured over, and the most steps.
const PACE_WINDOW: Duration = Duration::from_secs(3);
const PACE_STEPS: usize = 3;

/// The client's state: a game runs, and lets the players hit.
const RUNNING: u8 = 1;
/// Ball 0 fades out; ball 1 fades out at 3.
const FADE: u8 = 2;

/// What a hit did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hit {
    /// What was hit: ball 0, ball 1, snowman 0, snowman 1.
    pub what: u8,
    /// What the hit shows, 0 for a miss.
    pub damage: u16,
    /// Whether a ball rolled or a snowman lost HP: the field's state is
    /// sent again.
    pub changed: bool,
    /// The client's lines it brought on: (team, message).
    pub messages: Vec<(u8, u8)>,
}

/// What the time did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Turn {
    /// A ball's pace or a snowman's HP changed.
    Changed,
    /// The client's line `message` about `team`'s ball.
    Message { team: u8, message: u8 },
    /// The game is over: `winner` won, or nobody did.
    Ended { winner: Option<u8> },
    /// Time to send everyone off: `winner`'s players to `WINNERS_MAP`.
    WarpOut { winner: Option<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Playing { ends: Instant },
    Finishing { at: Instant, winner: Option<u8> },
}

#[derive(Clone, Debug, Default)]
struct Ball {
    position: i32,
    hits: u8,
    /// When it rolled its last steps, for its pace.
    steps: VecDeque<Instant>,
    stage: u8,
}

#[derive(Clone, Copy, Debug)]
struct Snowman {
    hp: u32,
    /// When it is back up, once it is down.
    up_at: Option<Instant>,
}

pub struct SnowballGame {
    rules: Snowball,
    /// How long a snowman stays down.
    wait: Duration,
    teams: BTreeMap<ClientId, u8>,
    phase: Phase,
    balls: [Ball; 2],
    snowmen: [Snowman; 2],
    next_recovery: Option<Instant>,
    last_hit: HashMap<ClientId, Instant>,
    last_knock: HashMap<ClientId, Instant>,
}

impl SnowballGame {
    pub fn new(rules: Snowball, wait: Duration) -> Self {
        let snowman = Snowman {
            hp: rules.snowman_hp,
            up_at: None,
        };
        Self {
            rules,
            wait,
            teams: BTreeMap::new(),
            phase: Phase::Idle,
            balls: [ball(), ball()],
            snowmen: [snowman; 2],
            next_recovery: None,
            last_hit: HashMap::new(),
            last_knock: HashMap::new(),
        }
    }

    pub fn team(&self, client_id: ClientId) -> Option<u8> {
        self.teams.get(&client_id).copied()
    }

    /// A player arriving joins the team with fewer players, Story on a tie,
    /// and keeps it until they leave.
    pub fn join(&mut self, client_id: ClientId) -> u8 {
        if let Some(team) = self.team(client_id) {
            return team;
        }
        let maple = self.teams.values().filter(|&&team| team == MAPLE).count();
        let team = if maple < self.teams.len() - maple {
            MAPLE
        } else {
            STORY
        };
        self.teams.insert(client_id, team);
        team
    }

    pub fn leave(&mut self, client_id: ClientId) {
        self.teams.remove(&client_id);
        self.last_hit.remove(&client_id);
        self.last_knock.remove(&client_id);
    }

    /// The client's state: 1 while the game runs, 2 or 3 fading out the
    /// losing ball once it is over, 0 otherwise.
    pub fn state(&self) -> u8 {
        match self.phase {
            Phase::Playing { .. } => RUNNING,
            Phase::Finishing {
                winner: Some(winner),
                ..
            } => FADE + (1 - winner),
            _ => 0,
        }
    }

    /// Each snowman's HP, in percent.
    pub fn snowmen(&self) -> [i32; 2] {
        self.snowmen.map(|snowman| {
            let max = u64::from(self.rules.snowman_hp.max(1));
            (u64::from(snowman.hp) * 100 / max) as i32
        })
    }

    pub fn balls(&self) -> [BallState; 2] {
        [0, 1].map(|b| BallState {
            position: self.balls[b].position as i16,
            stage: self.balls[b].stage,
        })
    }

    /// What the client shows for a hit on a ball, on a snowman, and a
    /// strong one on a snowman.
    pub fn damages(&self) -> [u16; 3] {
        [
            self.rules.damage_ball,
            self.rules.damage_snowman,
            self.rules.damage_snowman_strong,
        ]
    }

    /// The seconds left on the clock while the game runs.
    pub fn remaining(&self, now: Instant) -> Option<u32> {
        match self.phase {
            Phase::Playing { ends } => Some(seconds(ends.saturating_duration_since(now))),
            _ => None,
        }
    }

    /// A GM starts a game, for `length`; not while one runs or is ending.
    pub fn start(&mut self, now: Instant, length: Duration) -> bool {
        if self.phase != Phase::Idle {
            return false;
        }
        self.set_up();
        self.phase = Phase::Playing { ends: now + length };
        self.next_recovery = Some(now + RECOVERY_EVERY);
        true
    }

    /// A GM calls the game off and puts the balls back; whether one was on.
    /// Not once the game is over: the losing ball has faded out, and the
    /// client has no state that brings it back.
    pub fn reset(&mut self) -> bool {
        if !matches!(self.phase, Phase::Playing { .. }) {
            return false;
        }
        self.phase = Phase::Idle;
        self.set_up();
        true
    }

    fn set_up(&mut self) {
        self.balls = [ball(), ball()];
        for snowman in &mut self.snowmen {
            *snowman = Snowman {
                hp: self.rules.snowman_hp,
                up_at: None,
            };
        }
        self.last_hit.clear();
    }

    /// Whether ball `b` is stopped: the snowman in the other lane is down.
    fn stopped(&self, b: usize) -> bool {
        self.snowmen[1 - b].up_at.is_some()
    }

    /// `client_id` hits `what`: their team's ball (`what` 0 or 1) or the
    /// snowman in their lane (2 or 3). Three hits roll a ball one step; a
    /// stopped ball shows a miss. A hit on a snowman misses 20% of the time
    /// and is strong 10% of it, as the client rolls; one brought down stops
    /// the other team's ball for `snowManWait`. `roll` draws from [0, 1).
    pub fn hit(
        &mut self,
        client_id: ClientId,
        what: u8,
        now: Instant,
        roll: &mut dyn FnMut() -> f64,
    ) -> Option<Hit> {
        let team = self.team(client_id)?;
        if !matches!(self.phase, Phase::Playing { ends } if now < ends)
            || what > 3
            || what % 2 != team
        {
            return None;
        }
        if self
            .last_hit
            .get(&client_id)
            .is_some_and(|at| now < *at + HIT_COOLDOWN)
        {
            return None;
        }
        if what >= 2 && self.snowmen[usize::from(what - 2)].up_at.is_some() {
            return None;
        }
        self.last_hit.insert(client_id, now);
        let mut hit = Hit {
            what,
            ..Hit::default()
        };
        if what < 2 {
            let b = usize::from(what);
            if self.stopped(b) {
                return Some(hit);
            }
            hit.damage = self.rules.damage_ball;
            let ball = &mut self.balls[b];
            ball.hits += 1;
            if ball.hits >= HITS_PER_STEP && ball.position < self.rules.x_max {
                ball.hits = 0;
                ball.position += 1;
                ball.steps.push_back(now);
                ball.stage = pace(ball, now);
                hit.changed = true;
                if let Some(section) = self.rules.sections.iter().position(|&s| s == ball.position)
                {
                    hit.messages.push((what, section as u8 + 1));
                }
            }
            return Some(hit);
        }
        let roll = roll();
        hit.damage = if roll < 0.2 {
            0
        } else if roll >= 0.9 {
            self.rules.damage_snowman_strong
        } else {
            self.rules.damage_snowman
        };
        let snowman = &mut self.snowmen[usize::from(what - 2)];
        snowman.hp = snowman.hp.saturating_sub(u32::from(hit.damage));
        hit.changed = hit.damage > 0;
        if snowman.hp == 0 {
            snowman.up_at = Some(now + self.wait);
            // Its ball must stop on the client too.
            self.balls[usize::from(1 - team)].stage = CATCH_UP;
            // "Team <team> is attacking the snowman, stopping the progress
            // of Team <the other>'s snowball", of the team stopped.
            hit.messages.push((1 - team, 4));
        }
        Some(hit)
    }

    /// Lets everyone hit again at once, for the field's tests.
    #[cfg(test)]
    pub fn forget_hits(&mut self) {
        self.last_hit.clear();
    }

    /// Whether `client_id`, inside a ball, is thrown back now.
    pub fn knock(&mut self, client_id: ClientId, now: Instant) -> bool {
        if !self.teams.contains_key(&client_id)
            || self
                .last_knock
                .get(&client_id)
                .is_some_and(|at| now < *at + KNOCK_COOLDOWN)
        {
            return false;
        }
        self.last_knock.insert(client_id, now);
        true
    }

    /// What the time has done by `now`: snowmen back up, their recovery,
    /// the balls' pace, the end (time out, or a ball at the end of its
    /// lane), and the warp out `finish` after it.
    pub fn tick(&mut self, now: Instant, finish: Duration) -> Vec<Turn> {
        let mut turns = Vec::new();
        match self.phase {
            Phase::Idle => {}
            Phase::Playing { ends } => {
                let mut changed = false;
                for s in 0..2 {
                    let snowman = &mut self.snowmen[s];
                    if snowman.up_at.is_some_and(|at| now >= at) {
                        *snowman = Snowman {
                            hp: self.rules.snowman_hp,
                            up_at: None,
                        };
                        changed = true;
                        turns.push(Turn::Message {
                            team: 1 - s as u8,
                            message: 5,
                        });
                    }
                }
                if self.next_recovery.is_some_and(|at| now >= at) {
                    self.next_recovery = Some(now + RECOVERY_EVERY);
                    for snowman in &mut self.snowmen {
                        if snowman.up_at.is_none() && snowman.hp < self.rules.snowman_hp {
                            snowman.hp = snowman
                                .hp
                                .saturating_add(self.rules.recovery)
                                .min(self.rules.snowman_hp);
                            changed = true;
                        }
                    }
                }
                for b in 0..2 {
                    let stopped = self.stopped(b);
                    let ball = &mut self.balls[b];
                    let stage = if stopped { CATCH_UP } else { pace(ball, now) };
                    if stage != ball.stage {
                        ball.stage = stage;
                        changed = true;
                    }
                }
                let at_end = self
                    .balls
                    .iter()
                    .any(|ball| ball.position >= self.rules.x_max);
                if now >= ends || at_end {
                    let [story, maple] = [&self.balls[0], &self.balls[1]].map(|ball| ball.position);
                    let winner = match story.cmp(&maple) {
                        std::cmp::Ordering::Greater => Some(STORY),
                        std::cmp::Ordering::Less => Some(MAPLE),
                        std::cmp::Ordering::Equal => None,
                    };
                    for ball in &mut self.balls {
                        ball.stage = CATCH_UP;
                    }
                    self.phase = Phase::Finishing {
                        at: now + finish,
                        winner,
                    };
                    turns.push(Turn::Ended { winner });
                } else if changed {
                    turns.push(Turn::Changed);
                }
            }
            Phase::Finishing { at, winner } if now >= at => {
                // The next to come find the balls back at the start.
                self.phase = Phase::Idle;
                self.set_up();
                turns.push(Turn::WarpOut { winner });
            }
            Phase::Finishing { .. } => {}
        }
        turns
    }
}

fn ball() -> Ball {
    Ball {
        stage: CATCH_UP,
        ..Ball::default()
    }
}

/// The stage the client should roll the ball at: the fastest no faster
/// than it has been pushed over the last few seconds, so the client trails
/// it and catches up at each step it is sent. Once the next step is late,
/// it only catches up, so that it is never more than a step ahead, which
/// that stage takes back.
fn pace(ball: &mut Ball, now: Instant) -> u8 {
    while ball.steps.len() > PACE_STEPS
        || ball
            .steps
            .front()
            .is_some_and(|at| now.saturating_duration_since(*at) > PACE_WINDOW)
    {
        ball.steps.pop_front();
    }
    let (Some(first), Some(last)) = (ball.steps.front(), ball.steps.back()) else {
        return CATCH_UP;
    };
    let steps = ball.steps.len() as u32;
    if steps < 2 {
        return CATCH_UP;
    }
    let per_step = (*last - *first).as_millis() as u64 / u64::from(steps - 1);
    let Some(stage) = STEP_TIMES.iter().position(|&time| time >= per_step) else {
        return CATCH_UP;
    };
    let since = now.saturating_duration_since(*last).as_millis() as u64;
    if since >= STEP_TIMES[stage] {
        return CATCH_UP;
    }
    stage as u8
}

/// Whole seconds, rounded up, as the clock shows them.
pub fn seconds(time: Duration) -> u32 {
    time.as_millis().div_ceil(1000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Snowball {
        Snowball {
            dx: 3,
            x_min: -60,
            x_max: 900,
            damage_ball: 10,
            damage_snowman: 15,
            damage_snowman_strong: 45,
            snowman_hp: 30,
            snowman_wait: 10_000,
            speed: 150,
            recovery: 10,
            sections: [1, 2, 3],
        }
    }

    fn game() -> (SnowballGame, Instant) {
        let mut game = SnowballGame::new(rules(), Duration::from_secs(10));
        assert_eq!(game.join(1), STORY);
        assert_eq!(game.join(2), MAPLE);
        assert_eq!(game.join(3), STORY);
        let now = Instant::now();
        assert!(game.start(now, LENGTH));
        (game, now)
    }

    fn at(now: Instant, ms: u64) -> Instant {
        now + Duration::from_millis(ms)
    }

    #[test]
    fn players_join_the_smaller_team_and_hit_their_own_lane() {
        let (mut game, now) = game();
        let mut roll = || 0.5;
        // Story hits ball 0 and snowman 0; ball 1 and snowman 1 are Maple's.
        assert!(game.hit(1, 1, now, &mut roll).is_none());
        assert!(game.hit(1, 3, now, &mut roll).is_none());
        let hit = game.hit(1, 0, now, &mut roll).expect("its ball");
        assert_eq!((hit.damage, hit.changed), (10, false));
        // Once per 500 ms.
        assert!(game.hit(1, 0, at(now, 499), &mut roll).is_none());
        assert!(game.hit(2, 3, now, &mut roll).is_some());
        assert!(game.hit(9, 0, now, &mut roll).is_none(), "not on a team");
    }

    #[test]
    fn three_hits_roll_a_ball_a_step_and_its_sections_are_told() {
        let (mut game, now) = game();
        let mut roll = || 0.5;
        let mut messages = Vec::new();
        for i in 0..9 {
            let hit = game.hit(1, 0, at(now, 500 * i), &mut roll).unwrap();
            assert_eq!(hit.changed, i % 3 == 2);
            messages.extend(hit.messages);
        }
        assert_eq!(game.balls()[0].position, 3);
        assert_eq!(game.balls()[1].position, 0);
        assert_eq!(messages, [(0, 1), (0, 2), (0, 3)]);
    }

    #[test]
    fn a_snowman_brought_down_stops_the_other_ball_for_a_while() {
        let (mut game, now) = game();
        let mut strong = || 0.95;
        let hit = game.hit(2, 3, now, &mut strong).unwrap();
        assert_eq!(hit.damage, 45);
        // Maple's snowman down: Story's ball stops, as Story is told.
        assert_eq!(hit.messages, [(STORY, 4)]);
        assert_eq!(game.snowmen(), [100, 0]);
        assert!(game.hit(2, 3, at(now, 600), &mut strong).is_none(), "down");
        let stopped = game.hit(1, 0, now, &mut strong).unwrap();
        assert_eq!(stopped.damage, 0);
        assert!(game.hit(2, 1, at(now, 600), &mut strong).unwrap().damage > 0);
        let turns = game.tick(at(now, 10_000), FINISH);
        assert!(turns.contains(&Turn::Message {
            team: STORY,
            message: 5
        }));
        assert_eq!(game.snowmen(), [100, 100]);
        assert_eq!(
            game.hit(1, 0, at(now, 10_000), &mut strong).unwrap().damage,
            10
        );
    }

    #[test]
    fn snowman_hits_miss_or_are_strong_as_the_client_rolls() {
        let (mut game, now) = game();
        assert_eq!(game.hit(2, 3, now, &mut || 0.1).unwrap().damage, 0);
        assert_eq!(
            game.hit(2, 3, at(now, 500), &mut || 0.2).unwrap().damage,
            15
        );
        assert_eq!(game.snowmen()[1], 50);
        // A snowman that is up regains HP every 10 s.
        game.tick(at(now, 10_000), FINISH);
        assert_eq!(game.snowmen()[1], 83);
    }

    /// Ten more of Story's players.
    fn story(game: &mut SnowballGame) -> Vec<ClientId> {
        (10..40)
            .filter(|&id| game.join(id) == STORY)
            .take(10)
            .collect()
    }

    #[test]
    fn a_pushed_ball_rolls_at_its_pace() {
        let (mut game, now) = game();
        let mut roll = || 0.5;
        // One player: a step every 1.5 s, slower than the client catches up.
        for i in 0..6 {
            game.hit(1, 0, at(now, 500 * i), &mut roll);
        }
        assert_eq!(game.balls()[0].position, 2);
        assert_eq!(game.balls()[0].stage, CATCH_UP);
        assert!(game.tick(at(now, 2600), FINISH).is_empty());

        // Many: a hit every 50 ms is a step every 150 ms, stage 0, as soon
        // as the second step shows the pace.
        let (mut game, now) = self::game();
        let story = story(&mut game);
        let mut t = 0;
        let mut stages = Vec::new();
        for _ in 0..2 {
            for &id in &story {
                let hit = game.hit(id, 0, at(now, t), &mut roll).unwrap();
                if hit.changed {
                    stages.push(game.balls()[0].stage);
                }
                t += 50;
            }
        }
        assert_eq!(stages, [CATCH_UP, 0, 0, 0, 0, 0]);
        // It keeps it until the next step is late (the last was at 850 ms),
        // then only catches up.
        assert!(game.tick(at(now, 950), FINISH).is_empty());
        assert_eq!(game.tick(at(now, 1000), FINISH), [Turn::Changed]);
        assert_eq!(game.balls()[0].stage, CATCH_UP);
        // A step every 400 ms is stage 5 (400 ms). Three players a step, as
        // each may hit only once per 500 ms.
        let mut t = 10_000;
        for round in story.chunks(3).take(3) {
            for &id in round {
                game.hit(id, 0, at(now, t), &mut roll);
            }
            t += 400;
        }
        assert_eq!(game.balls()[0].stage, 5);
    }

    #[test]
    fn a_stopped_ball_stops_rolling_at_once() {
        let (mut game, now) = game();
        let story = story(&mut game);
        for (i, &id) in story.iter().take(6).enumerate() {
            game.hit(id, 0, at(now, 50 * i as u64), &mut || 0.5);
        }
        assert_eq!(game.balls()[0].stage, 0);
        // Maple brings its snowman down.
        game.hit(2, 3, at(now, 300), &mut || 0.95);
        assert_eq!(game.balls()[0].stage, CATCH_UP);
        game.tick(at(now, 350), FINISH);
        assert_eq!(game.balls()[0].stage, CATCH_UP);
    }

    #[test]
    fn the_ball_further_on_wins_and_everyone_goes() {
        let (mut game, now) = game();
        let mut roll = || 0.5;
        for i in 0..3 {
            game.hit(2, 1, at(now, 500 * i), &mut roll);
        }
        assert!(game.tick(at(now, 599_000), FINISH).is_empty());
        let ended = at(now, 600_000);
        assert_eq!(
            game.tick(ended, FINISH),
            [Turn::Ended {
                winner: Some(MAPLE)
            }]
        );
        // Story's ball fades out.
        assert_eq!(game.state(), 2);
        assert!(game.hit(2, 1, ended, &mut roll).is_none());
        assert!(game
            .tick(ended + FINISH - Duration::from_millis(1), FINISH)
            .is_empty());
        assert_eq!(
            game.tick(ended + FINISH, FINISH),
            [Turn::WarpOut {
                winner: Some(MAPLE)
            }]
        );
        assert_eq!(game.state(), 0);
        assert_eq!(game.balls()[1].position, 0);
        assert!(game.start(ended + FINISH, LENGTH));
        assert_eq!(game.balls()[1].position, 0);
    }

    #[test]
    fn a_ball_at_the_end_of_its_lane_wins_at_once() {
        let mut rules = rules();
        rules.x_max = 1;
        let mut game = SnowballGame::new(rules, Duration::from_secs(10));
        game.join(1);
        let now = Instant::now();
        game.start(now, LENGTH);
        for i in 0..4 {
            game.hit(1, 0, at(now, 500 * i), &mut || 0.5);
        }
        assert_eq!(game.balls()[0].position, 1, "no further than xMax");
        assert_eq!(
            game.tick(at(now, 2000), FINISH),
            [Turn::Ended {
                winner: Some(STORY)
            }]
        );
        assert_eq!(game.state(), 3);
    }

    #[test]
    fn a_reset_puts_everything_back_and_a_tie_has_no_winner() {
        let (mut game, now) = game();
        game.hit(2, 3, now, &mut || 0.5);
        assert!(game.reset());
        assert!(!game.reset());
        // Nor once a game is over.
        game.start(now, LENGTH);
        game.tick(at(now, 600_000), FINISH);
        assert!(!game.reset());
        game.tick(at(now, 600_000) + FINISH, FINISH);
        assert_eq!(game.snowmen(), [100, 100]);
        assert!(game.hit(2, 3, at(now, 600), &mut || 0.5).is_none());
        let now = at(now, 700_000);
        game.start(now, LENGTH);
        assert_eq!(
            game.tick(at(now, 600_000), FINISH),
            [Turn::Ended { winner: None }]
        );
        assert_eq!(game.state(), 0);
    }

    #[test]
    fn a_player_in_a_ball_is_thrown_back_twice_a_second() {
        let (mut game, now) = game();
        assert!(game.knock(1, now));
        assert!(!game.knock(1, at(now, 300)));
        assert!(game.knock(1, at(now, 500)));
        assert!(game.knock(2, at(now, 300)));
        assert!(!game.knock(9, now), "not on the field");
    }
}
