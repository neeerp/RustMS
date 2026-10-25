//! An OX Quiz on its field (109020001, field type 5, report b171): ten
//! questions, each answered by standing in the O room or the X room when
//! its time is up. Pure state; `FieldActor` sends the packets and the
//! players off.

use game_data::ox_quiz::{OxQuestion, OxQuiz};
use game_data::Area;
use net::ClientId;
use rand::seq::SliceRandom;
use rand::Rng;
use tokio::time::{Duration, Instant};

/// How many questions a game asks (the map's help).
pub const QUESTIONS: usize = 10;
/// How long a question is up: the client's own clock for it.
pub const QUESTION_TIME: Duration = Duration::from_secs(30);
/// How long the answer shows before the wrong are sent off.
pub const ANSWER_TIME: Duration = Duration::from_secs(5);
/// The pause before the next question.
pub const PAUSE_TIME: Duration = Duration::from_secs(3);

/// Where everyone who answered all ten goes (as after Coconut Season).
pub const WINNERS_MAP: i32 = 109_050_000;
/// Where the wrong go: the field's forcedReturn.
pub const EXIT_MAP: i32 = 109_050_001;

/// What the time did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Turn {
    /// The next question goes up.
    Ask(OxQuestion),
    /// Its time is up: the answer shows, and the field judges everyone.
    Answer(OxQuestion),
    /// Those judged wrong go; on the `last` question the game is over, and
    /// those judged right win.
    SendOff {
        set: u8,
        wrong: Vec<ClientId>,
        right: Vec<ClientId>,
        last: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Asking,
    Answering,
    Pausing,
}

#[derive(Debug)]
pub struct OxGame {
    times: [Duration; 3],
    questions: Vec<OxQuestion>,
    round: usize,
    phase: Phase,
    until: Instant,
    wrong: Vec<ClientId>,
    right: Vec<ClientId>,
    on: bool,
}

impl OxGame {
    /// A game whose question, answer and pause last `times`.
    pub fn new(times: [Duration; 3]) -> Self {
        Self {
            times,
            questions: Vec::new(),
            round: 0,
            phase: Phase::Pausing,
            until: Instant::now(),
            wrong: Vec::new(),
            right: Vec::new(),
            on: false,
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    /// Starts on `questions`, unless a game is on or there are none, and
    /// gives the first.
    pub fn start(&mut self, questions: Vec<OxQuestion>, now: Instant) -> Option<OxQuestion> {
        let first = *questions.first().filter(|_| !self.on)?;
        *self = Self {
            questions,
            on: true,
            phase: Phase::Asking,
            until: now + self.times[0],
            ..Self::new(self.times)
        };
        Some(first)
    }

    /// Stops the game, if one is on, and gives its set.
    pub fn reset(&mut self) -> Option<u8> {
        let set = self.questions.first().filter(|_| self.on)?.set;
        self.on = false;
        Some(set)
    }

    /// The question up and how long it has left, or the one whose answer
    /// shows, for a player coming in.
    pub fn showing(&self, now: Instant) -> Option<(OxQuestion, Option<Duration>)> {
        let question = *self.questions.get(self.round).filter(|_| self.on)?;
        match self.phase {
            Phase::Asking => Some((question, Some(self.until.saturating_duration_since(now)))),
            Phase::Answering => Some((question, None)),
            Phase::Pausing => None,
        }
    }

    /// The field's judgement of the question whose answer shows: who
    /// answered it wrong and who right.
    pub fn judged(&mut self, wrong: Vec<ClientId>, right: Vec<ClientId>) {
        self.wrong = wrong;
        self.right = right;
    }

    pub fn tick(&mut self, now: Instant) -> Option<Turn> {
        if !self.on || now < self.until {
            return None;
        }
        let question = self.questions[self.round];
        Some(match self.phase {
            Phase::Asking => {
                self.phase = Phase::Answering;
                self.until = now + self.times[1];
                Turn::Answer(question)
            }
            Phase::Answering => {
                self.round += 1;
                let last = self.round == self.questions.len();
                self.on = !last;
                self.phase = Phase::Pausing;
                self.until = now + self.times[2];
                Turn::SendOff {
                    set: question.set,
                    wrong: std::mem::take(&mut self.wrong),
                    right: std::mem::take(&mut self.right),
                    last,
                }
            }
            Phase::Pausing => {
                let question = self.questions[self.round];
                self.phase = Phase::Asking;
                self.until = now + self.times[0];
                Turn::Ask(question)
            }
        })
    }
}

/// Ten questions from one set, each different, all of them English: the
/// client draws Korean as `?`.
pub fn draw(quiz: &OxQuiz, rng: &mut impl Rng) -> Vec<OxQuestion> {
    let sets: Vec<Vec<OxQuestion>> = quiz
        .sets()
        .map(|set| set.iter().copied().filter(|q| q.english).collect())
        .filter(|set: &Vec<OxQuestion>| set.len() >= QUESTIONS)
        .collect();
    let Some(set) = sets.choose(rng) else {
        return Vec::new();
    };
    let mut questions: Vec<_> = set.choose_multiple(rng, QUESTIONS).copied().collect();
    questions.shuffle(rng);
    questions
}

/// The answer a player gives standing at (`x`, `y`) on `foothold`: O in
/// the field's first area (`area/o`, the left room), X in its second
/// (`area/x`, the right), none elsewhere or on a ladder or rope, whose
/// foothold is below 0.
pub fn answer(areas: &[Area], x: i16, y: i16, foothold: i16) -> Option<bool> {
    let (x, y) = (x.into(), y.into());
    match areas {
        [o, x_room, ..] if foothold >= 0 => {
            if o.contains(x, y) {
                Some(true)
            } else if x_room.contains(x, y) {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn question(number: u16, o: bool) -> OxQuestion {
        OxQuestion {
            set: 4,
            number,
            o,
            english: true,
        }
    }

    #[test]
    fn each_question_is_asked_answered_and_judged_in_turn() {
        let now = Instant::now();
        let mut game = OxGame::new([30 * SECOND, 5 * SECOND, 3 * SECOND]);
        let questions = vec![question(1, true), question(100, false)];
        assert_eq!(game.start(questions.clone(), now), Some(questions[0]));
        assert_eq!(game.start(questions.clone(), now), None);
        assert_eq!(
            game.showing(now + 10 * SECOND),
            Some((questions[0], Some(20 * SECOND)))
        );
        assert_eq!(game.tick(now + 29 * SECOND), None);
        let t = now + 30 * SECOND;
        assert_eq!(game.tick(t), Some(Turn::Answer(questions[0])));
        assert_eq!(game.showing(t), Some((questions[0], None)));
        game.judged(vec![7], vec![8]);
        assert_eq!(game.tick(t + 4 * SECOND), None);
        let t = t + 5 * SECOND;
        assert_eq!(
            game.tick(t),
            Some(Turn::SendOff {
                set: 4,
                wrong: vec![7],
                right: vec![8],
                last: false
            })
        );
        assert_eq!(game.showing(t), None);
        let t = t + 3 * SECOND;
        assert_eq!(game.tick(t), Some(Turn::Ask(questions[1])));
        let t = t + 30 * SECOND;
        assert_eq!(game.tick(t), Some(Turn::Answer(questions[1])));
        let t = t + 5 * SECOND;
        assert_eq!(
            game.tick(t),
            Some(Turn::SendOff {
                set: 4,
                wrong: vec![],
                right: vec![],
                last: true
            })
        );
        assert!(!game.on());
        assert_eq!(game.tick(t + 60 * SECOND), None);
        assert_eq!(game.start(questions.clone(), t), Some(questions[0]));
    }

    #[test]
    fn a_reset_stops_the_game() {
        let now = Instant::now();
        let mut game = OxGame::new([SECOND; 3]);
        assert_eq!(game.reset(), None);
        assert_eq!(game.start(Vec::new(), now), None);
        game.start(vec![question(1, true)], now);
        assert_eq!(game.reset(), Some(4));
        assert_eq!(game.tick(now + 60 * SECOND), None);
        assert_eq!(game.showing(now), None);
    }

    #[test]
    fn the_left_room_is_o_the_right_x_and_the_rest_no_answer() {
        // Map.nx 109020001 `area/o` and `area/x`.
        let areas = [
            Area {
                x1: -1030,
                x2: -304,
                y1: -150,
                y2: 347,
            },
            Area {
                x1: -146,
                x2: 570,
                y1: -134,
                y2: 360,
            },
        ];
        assert_eq!(answer(&areas, -600, 334, 12), Some(true));
        assert_eq!(answer(&areas, 200, 334, 40), Some(false));
        // Jumping in a room still counts.
        assert_eq!(answer(&areas, 200, 300, 0), Some(false));
        // The top platform, the middle, and the ladder down to the X room.
        assert_eq!(answer(&areas, 0, -206, 3), None);
        assert_eq!(answer(&areas, -230, 274, 90), None);
        assert_eq!(answer(&areas, 268, -100, -4), None);
        assert_eq!(answer(&[], -600, 334, 12), None);
    }

    #[test]
    fn ten_different_english_questions_of_one_set() {
        let Ok(quiz) = OxQuiz::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Etc.nx"),
        ) else {
            return;
        };
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let questions = draw(&quiz, &mut rng);
            assert_eq!(questions.len(), QUESTIONS);
            assert!(questions
                .iter()
                .all(|q| q.english && q.set == questions[0].set));
            let mut numbers: Vec<_> = questions.iter().map(|q| q.number).collect();
            numbers.sort();
            numbers.dedup();
            assert_eq!(numbers.len(), QUESTIONS);
        }
        assert!(draw(&OxQuiz::default(), &mut rng).is_empty());
    }
}
