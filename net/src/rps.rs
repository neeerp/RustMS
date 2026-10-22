//! Rock, Paper, Scissors (RPS_ACTION, RPS_GAME; report b038): the Rock,
//! Paper, Scissors Admin's minigame. A game costs a fee; each win adds to a
//! streak the player may keep (a certificate for that many wins, on leaving)
//! or risk on another round. A loss ends the streak with nothing, a draw is
//! played again, and ten wins end the game with the last certificate. The
//! client never decides a round: it shows the NPC's hand and the streak the
//! server sends, and reads a negative streak as a loss.

/// The Rock, Paper, Scissors Admin (`info/rpsGame`).
pub const RPS_ADMIN: i32 = 9000019;
/// What a game costs, as the client's notice says.
pub const FEE: i32 = 1000;
/// Paid for losing a first round the client calls one (pool 0xe61).
pub const CONSOLATION: i32 = 500;
/// The streak that ends the game.
pub const MOST_WINS: i8 = 10;
/// The certificate for one straight win; `n` wins earn the `n`th from here.
pub const FIRST_CERTIFICATE: i32 = 4031332;
/// The hands, in the order the client draws them.
pub const HANDS: u8 = 3;

/// A request from the window, by RPS_ACTION mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpsRequest {
    /// 0: the Start button.
    Start,
    /// 1: a hand (0 rock, 1 paper, 2 scissors).
    Choose(u8),
    /// 2: the 30 seconds to choose ran out.
    TimeUp,
    /// 3: the Continue button, after a win.
    Continue,
    /// 4: the Exit button.
    Exit,
    /// 5: the Retry button, after a loss or the last win.
    Retry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// The window is open with Start.
    Ready,
    /// A round is on and the client waits for a hand.
    Choosing,
    /// The last round was won; Continue or Exit.
    Won,
    /// Lost, out of time, or all ten won; Retry or Exit.
    Over,
}

/// What the runtime does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpsStep {
    /// Nothing; the request doesn't fit the game.
    Ignore,
    /// Charge the fee and start a game, or refuse it.
    Charge,
    /// Show the round: the NPC's hand and the streak, negative on a loss,
    /// with `mesos` paid and `certificate` given.
    Round {
        hand: u8,
        streak: i8,
        mesos: i32,
        certificate: Option<i32>,
    },
    /// The round ran out of time.
    TimeOver,
    /// Play the next round of the streak.
    NextRound,
    /// Close the window, giving `certificate`.
    Close { certificate: Option<i32> },
}

/// The certificate for `wins` straight wins.
pub fn certificate(wins: i8) -> Option<i32> {
    (1..=MOST_WINS)
        .contains(&wins)
        .then(|| FIRST_CERTIFICATE + i32::from(wins) - 1)
}

/// Whether `choice` beats `hand`: rock (0) beats scissors (2), paper (1)
/// rock, scissors paper.
fn beats(choice: u8, hand: u8) -> bool {
    (choice + HANDS - hand) % HANDS == 1
}

/// The game a player has open with the admin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpsGame {
    stage: Stage,
    streak: i8,
    /// The streak the client last heard. It shows the consolation for a
    /// loss only when this is 0, and only a refusal or a new window sets it
    /// back, not a new game: mirrored so the 500 mesos match its words.
    shown: i8,
}

impl Default for RpsGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpsGame {
    /// The window just opened.
    pub fn new() -> Self {
        Self {
            stage: Stage::Ready,
            streak: 0,
            shown: 0,
        }
    }

    /// What to do for `request`; `hand` is the admin's hand should a round
    /// be played.
    pub fn request(&mut self, request: RpsRequest, hand: u8) -> RpsStep {
        match (self.stage, request) {
            (Stage::Ready, RpsRequest::Start) | (Stage::Over, RpsRequest::Retry) => RpsStep::Charge,
            (Stage::Choosing, RpsRequest::Choose(choice)) if choice < HANDS && hand < HANDS => {
                self.round(choice, hand)
            }
            (Stage::Choosing, RpsRequest::TimeUp) => {
                self.end();
                self.shown = -1;
                RpsStep::TimeOver
            }
            (Stage::Won, RpsRequest::Continue) => {
                self.stage = Stage::Choosing;
                RpsStep::NextRound
            }
            (Stage::Won, RpsRequest::Exit) => RpsStep::Close {
                certificate: certificate(self.streak),
            },
            (Stage::Ready | Stage::Over, RpsRequest::Exit) => RpsStep::Close { certificate: None },
            _ => RpsStep::Ignore,
        }
    }

    /// The fee was paid: the first round starts.
    pub fn charged(&mut self) {
        self.stage = Stage::Choosing;
        self.streak = 0;
    }

    /// The fee couldn't be paid: the window shows Start again.
    pub fn refused(&mut self) {
        *self = Self::new();
    }

    fn round(&mut self, choice: u8, hand: u8) -> RpsStep {
        if choice == hand {
            // A draw is played again; the client starts the round itself.
            return RpsStep::Round {
                hand,
                streak: self.streak,
                mesos: 0,
                certificate: None,
            };
        }
        if !beats(choice, hand) {
            let mesos = if self.shown == 0 { CONSOLATION } else { 0 };
            self.end();
            self.shown = -1;
            return RpsStep::Round {
                hand,
                streak: -1,
                mesos,
                certificate: None,
            };
        }
        self.streak += 1;
        let streak = self.streak;
        self.shown = streak;
        if streak < MOST_WINS {
            self.stage = Stage::Won;
            return RpsStep::Round {
                hand,
                streak,
                mesos: 0,
                certificate: None,
            };
        }
        self.end();
        RpsStep::Round {
            hand,
            streak,
            mesos: 0,
            certificate: certificate(streak),
        }
    }

    fn end(&mut self) {
        self.stage = Stage::Over;
        self.streak = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROCK: u8 = 0;
    const PAPER: u8 = 1;
    const SCISSORS: u8 = 2;

    fn started() -> RpsGame {
        let mut game = RpsGame::new();
        assert_eq!(game.request(RpsRequest::Start, 0), RpsStep::Charge);
        game.charged();
        game
    }

    fn win(game: &mut RpsGame) -> RpsStep {
        game.request(RpsRequest::Choose(PAPER), ROCK)
    }

    #[test]
    fn each_hand_beats_one_other() {
        assert!(beats(ROCK, SCISSORS));
        assert!(beats(PAPER, ROCK));
        assert!(beats(SCISSORS, PAPER));
        assert!(!beats(ROCK, PAPER));
        assert!(!beats(PAPER, SCISSORS));
        assert!(!beats(SCISSORS, ROCK));
    }

    #[test]
    fn a_win_counts_and_leaving_takes_its_certificate() {
        let mut game = started();
        assert_eq!(
            win(&mut game),
            RpsStep::Round {
                hand: ROCK,
                streak: 1,
                mesos: 0,
                certificate: None
            }
        );
        assert_eq!(game.request(RpsRequest::Continue, 0), RpsStep::NextRound);
        assert!(matches!(win(&mut game), RpsStep::Round { streak: 2, .. }));
        assert_eq!(
            game.request(RpsRequest::Exit, 0),
            RpsStep::Close {
                certificate: Some(4031333)
            }
        );
    }

    #[test]
    fn a_draw_plays_again_without_a_word() {
        let mut game = started();
        assert_eq!(
            game.request(RpsRequest::Choose(SCISSORS), SCISSORS),
            RpsStep::Round {
                hand: SCISSORS,
                streak: 0,
                mesos: 0,
                certificate: None
            }
        );
        assert!(matches!(win(&mut game), RpsStep::Round { streak: 1, .. }));
    }

    #[test]
    fn only_the_first_loss_in_the_window_pays_the_consolation() {
        let mut game = started();
        assert_eq!(
            game.request(RpsRequest::Choose(ROCK), PAPER),
            RpsStep::Round {
                hand: PAPER,
                streak: -1,
                mesos: CONSOLATION,
                certificate: None
            }
        );
        // The client still holds the lost streak, so a new game's first
        // loss isn't consoled.
        assert_eq!(game.request(RpsRequest::Retry, 0), RpsStep::Charge);
        game.charged();
        assert!(matches!(
            game.request(RpsRequest::Choose(ROCK), PAPER),
            RpsStep::Round { mesos: 0, .. }
        ));
        game.request(RpsRequest::Retry, 0);
        game.charged();
        win(&mut game);
        game.request(RpsRequest::Continue, 0);
        assert_eq!(
            game.request(RpsRequest::Choose(ROCK), PAPER),
            RpsStep::Round {
                hand: PAPER,
                streak: -1,
                mesos: 0,
                certificate: None
            }
        );
        // Nothing is left to take on leaving.
        assert_eq!(
            game.request(RpsRequest::Exit, 0),
            RpsStep::Close { certificate: None }
        );
    }

    #[test]
    fn ten_wins_end_the_game_with_the_last_certificate() {
        let mut game = started();
        for _ in 1..MOST_WINS {
            win(&mut game);
            assert_eq!(game.request(RpsRequest::Continue, 0), RpsStep::NextRound);
        }
        assert_eq!(
            win(&mut game),
            RpsStep::Round {
                hand: ROCK,
                streak: 10,
                mesos: 0,
                certificate: Some(4031341)
            }
        );
        assert_eq!(game.request(RpsRequest::Continue, 0), RpsStep::Ignore);
        assert_eq!(
            game.request(RpsRequest::Exit, 0),
            RpsStep::Close { certificate: None }
        );
    }

    #[test]
    fn running_out_of_time_loses_without_consolation() {
        let mut game = started();
        assert_eq!(game.request(RpsRequest::TimeUp, 0), RpsStep::TimeOver);
        assert_eq!(
            game.request(RpsRequest::Choose(ROCK), ROCK),
            RpsStep::Ignore
        );
        assert_eq!(game.request(RpsRequest::Retry, 0), RpsStep::Charge);
    }

    #[test]
    fn requests_out_of_turn_are_ignored() {
        let mut game = RpsGame::new();
        for request in [
            RpsRequest::Choose(ROCK),
            RpsRequest::TimeUp,
            RpsRequest::Continue,
            RpsRequest::Retry,
        ] {
            assert_eq!(game.request(request, 0), RpsStep::Ignore);
        }
        let mut game = started();
        assert_eq!(game.request(RpsRequest::Start, 0), RpsStep::Ignore);
        assert_eq!(game.request(RpsRequest::Exit, 0), RpsStep::Ignore);
        assert_eq!(game.request(RpsRequest::Choose(3), 0), RpsStep::Ignore);
    }

    #[test]
    fn a_refused_fee_shows_start_again_and_resets_the_shown_streak() {
        let mut game = started();
        game.request(RpsRequest::Choose(ROCK), PAPER);
        game.request(RpsRequest::Retry, 0);
        game.refused();
        assert_eq!(game.request(RpsRequest::Choose(ROCK), 0), RpsStep::Ignore);
        assert_eq!(game.request(RpsRequest::Start, 0), RpsStep::Charge);
        game.charged();
        assert!(matches!(
            game.request(RpsRequest::Choose(ROCK), PAPER),
            RpsStep::Round {
                mesos: CONSOLATION,
                ..
            }
        ));
    }
}
