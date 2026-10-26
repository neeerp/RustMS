//! The mini games (PLAYER_INTERACTION, mini room types 1 and 2): Omok and
//! match cards, for two players, opened with a game set from the Etc bag
//! (report 164-shops-games §3–5). The room lives in the field; these are
//! the requests and the rules of play.

use rand::seq::SliceRandom;
use rand::Rng;

/// The room types.
pub const OMOK: u8 = 1;
pub const MATCH_CARDS: u8 = 2;
/// A game seats its owner and one visitor.
pub const GAME_USERS: u8 = 2;

/// What a player asked of a game. Visiting, chatting and leaving are shared
/// with the trading room (`trade::TradeRequest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MiniGameRequest {
    /// Open a game of `kind` named `title`. `piece` is the Omok set's
    /// stones, or the size of the match cards' board.
    Create {
        kind: u8,
        title: String,
        password: Option<String>,
        piece: u8,
    },
    /// The visitor is ready, or not.
    Ready(bool),
    /// The owner starts the game.
    Start,
    AskTie,
    AnswerTie(bool),
    GiveUp,
    /// Omok: take back my last stone.
    AskUndo,
    AnswerUndo(bool),
    /// The owner turns the visitor out.
    Expel,
    /// Leave once this game ends, or not.
    LeaveAfter(bool),
    /// The turn's time ran out.
    Skip,
    /// Omok: a stone at column `x`, row `y`.
    Move {
        x: i32,
        y: i32,
        color: u8,
    },
    /// Match cards: turn up card `index`, first or second of the turn.
    Flip {
        first: bool,
        index: u8,
    },
}

/// The game set `kind` needs for `piece`: an Omok set of those stones, or
/// the match cards (Etc/0408.img; (id / 100) % 10 picks the game).
pub fn game_set(kind: u8, piece: u8) -> Option<i32> {
    match kind {
        OMOK if piece < OMOK_SETS => Some(4080000 + i32::from(piece)),
        MATCH_CARDS if piece < 3 => Some(4080100),
        _ => None,
    }
}

/// Whether a game may be opened in `map_id`: not where its fieldLimit
/// has 0x80 (v83 FUN_00a1d4d8, "You can't start the game here.").
pub fn may_play_here(map_id: i32) -> bool {
    crate::get_game_data()
        .ok()
        .and_then(|data| data.field(map_id))
        .is_none_or(|field| field.may_play_games())
}

/// The Omok sets, 4080000 to 4080011.
const OMOK_SETS: u8 = 12;

/// How many cards a board of `size` holds: 4 x 3, 5 x 4 or 6 x 5.
pub fn card_count(size: u8) -> usize {
    match size {
        0 => 12,
        1 => 20,
        _ => 30,
    }
}

/// The cards of a board of `size`, shuffled: each of the faces 0, 1, ...
/// twice.
pub fn deal(size: u8, rng: &mut impl Rng) -> Vec<i32> {
    let pairs = (card_count(size) / 2) as i32;
    let mut deck: Vec<i32> = (0..pairs).chain(0..pairs).collect();
    deck.shuffle(rng);
    deck
}

/// The Omok board's side (inferred: the 15 x 15 of the board art).
pub const BOARD: i32 = 15;

/// Why a stone may not go where asked (Omok's 0x41).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    /// Off the board or on a stone; the client checks these too.
    Taken,
    /// It would make two open threes at once ("You have double-3's.").
    DoubleThree,
}

impl Rejected {
    /// The code 0x41 carries: 'C' for double threes, anything else for the
    /// rest.
    pub fn code(self) -> u8 {
        match self {
            Rejected::Taken => 0,
            Rejected::DoubleThree => b'C',
        }
    }
}

/// What a stone did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placed {
    Played,
    /// Five or more in a row.
    Won,
    /// The board is full and nobody won.
    Full,
}

/// An Omok board: stones of color 1 or 2, and the order they went down in.
#[derive(Clone, Debug)]
pub struct Omok {
    cells: [[u8; BOARD as usize]; BOARD as usize],
    moves: Vec<(i32, i32, u8)>,
}

impl Default for Omok {
    fn default() -> Self {
        Self {
            cells: [[0; BOARD as usize]; BOARD as usize],
            moves: Vec::new(),
        }
    }
}

const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

impl Omok {
    fn at(&self, x: i32, y: i32) -> Option<u8> {
        let inside = (0..BOARD).contains(&x) && (0..BOARD).contains(&y);
        inside.then(|| self.cells[y as usize][x as usize])
    }

    /// The stones of `color` in a row through (x, y) along (dx, dy), (x, y)
    /// counted as one of them.
    fn run(&self, x: i32, y: i32, (dx, dy): (i32, i32), color: u8) -> i32 {
        let mut count = 1;
        for sign in [1, -1] {
            let (mut cx, mut cy) = (x + dx * sign, y + dy * sign);
            while self.at(cx, cy) == Some(color) {
                count += 1;
                cx += dx * sign;
                cy += dy * sign;
            }
        }
        count
    }

    /// Whether a stone of `color` at the empty (x, y) makes an open four
    /// along `direction` that holds the stone `with` cells on from it: four
    /// in a row with both ends free.
    fn open_four(&mut self, x: i32, y: i32, direction: (i32, i32), color: u8, with: i32) -> bool {
        self.cells[y as usize][x as usize] = color;
        let (dx, dy) = direction;
        let mut open = false;
        // Every window of four through both, with a free cell either side.
        for start in (-3..=0).filter(|start| (*start..start + 4).contains(&with)) {
            let (sx, sy) = (x + dx * start, y + dy * start);
            let four = (0..4).all(|i| self.at(sx + dx * i, sy + dy * i) == Some(color));
            let before = self.at(sx - dx, sy - dy) == Some(0);
            let after = self.at(sx + dx * 4, sy + dy * 4) == Some(0);
            if four && before && after {
                open = true;
                break;
            }
        }
        self.cells[y as usize][x as usize] = 0;
        open
    }

    /// Whether a stone of `color` at (x, y) makes an open three along
    /// `direction`: one more stone on that line would make an open four
    /// with it.
    fn open_three(&mut self, x: i32, y: i32, direction: (i32, i32), color: u8) -> bool {
        self.cells[y as usize][x as usize] = color;
        let (dx, dy) = direction;
        let three = (-3..=3).filter(|&i| i != 0).any(|i| {
            let (cx, cy) = (x + dx * i, y + dy * i);
            self.at(cx, cy) == Some(0) && self.open_four(cx, cy, direction, color, -i)
        });
        self.cells[y as usize][x as usize] = 0;
        three
    }

    /// Put a stone of `color` at (x, y). A stone that wins may make double
    /// threes (inferred: the usual Omok rule).
    pub fn place(&mut self, x: i32, y: i32, color: u8) -> Result<Placed, Rejected> {
        if self.at(x, y) != Some(0) {
            return Err(Rejected::Taken);
        }
        let wins = DIRECTIONS
            .iter()
            .any(|&direction| self.run(x, y, direction, color) >= 5);
        if !wins {
            let threes = DIRECTIONS
                .iter()
                .filter(|&&direction| self.open_three(x, y, direction, color))
                .count();
            if threes >= 2 {
                return Err(Rejected::DoubleThree);
            }
        }
        self.cells[y as usize][x as usize] = color;
        self.moves.push((x, y, color));
        Ok(if wins {
            Placed::Won
        } else if self.moves.len() == (BOARD * BOARD) as usize {
            Placed::Full
        } else {
            Placed::Played
        })
    }

    /// Whether `color` has a stone down.
    pub fn has_stones(&self, color: u8) -> bool {
        self.moves.iter().any(|&(_, _, c)| c == color)
    }

    /// Take back `color`'s last stone and any played after it; how many
    /// went.
    pub fn undo(&mut self, color: u8) -> usize {
        let Some(at) = self.moves.iter().rposition(|&(_, _, c)| c == color) else {
            return 0;
        };
        let taken = self.moves.split_off(at);
        for &(x, y, _) in &taken {
            self.cells[y as usize][x as usize] = 0;
        }
        taken.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn game_sets_pick_the_game_and_its_pieces() {
        assert_eq!(game_set(OMOK, 0), Some(4080000));
        assert_eq!(game_set(OMOK, 11), Some(4080011));
        assert_eq!(game_set(OMOK, 12), None);
        assert_eq!(game_set(MATCH_CARDS, 2), Some(4080100));
        assert_eq!(game_set(MATCH_CARDS, 3), None);
        assert_eq!(game_set(3, 0), None);
    }

    #[test]
    fn no_game_opens_where_the_map_forbids_them() {
        let data = crate::get_game_data().unwrap();
        let forbidden = data
            .field_ids()
            .filter(|&id| !data.field(id).unwrap().may_play_games())
            .min()
            .unwrap();
        assert!(!may_play_here(forbidden), "{forbidden}");
        assert!(may_play_here(100_000_000), "Henesys");
    }

    #[test]
    fn a_deck_holds_each_face_twice() {
        let mut rng = StdRng::seed_from_u64(7);
        for (size, count) in [(0, 12), (1, 20), (2, 30)] {
            let mut deck = deal(size, &mut rng);
            assert_eq!(deck.len(), count);
            deck.sort_unstable();
            let pairs: Vec<i32> = (0..count as i32 / 2).flat_map(|v| [v, v]).collect();
            assert_eq!(deck, pairs);
        }
    }

    #[test]
    fn five_in_a_row_wins_in_any_direction() {
        for (dx, dy) in DIRECTIONS {
            let mut board = Omok::default();
            let (x0, y0) = (5, 5);
            for i in 0..4 {
                assert_eq!(board.place(x0 + dx * i, y0 + dy * i, 1), Ok(Placed::Played));
            }
            assert_eq!(board.place(x0 + dx * 4, y0 + dy * 4, 1), Ok(Placed::Won));
        }
    }

    #[test]
    fn a_stone_needs_a_free_cell_on_the_board() {
        let mut board = Omok::default();
        board.place(0, 0, 1).unwrap();
        assert_eq!(board.place(0, 0, 2), Err(Rejected::Taken));
        assert_eq!(board.place(-1, 0, 2), Err(Rejected::Taken));
        assert_eq!(board.place(0, BOARD, 2), Err(Rejected::Taken));
    }

    #[test]
    fn two_open_threes_at_once_are_refused() {
        let mut board = Omok::default();
        // . X X ? across and down from (7, 7).
        for (x, y) in [(5, 7), (6, 7), (7, 5), (7, 6)] {
            board.place(x, y, 1).unwrap();
        }
        assert_eq!(board.place(7, 7, 1), Err(Rejected::DoubleThree));
        assert_eq!(Rejected::DoubleThree.code(), b'C');
        // The other color may play there, and a single three is fine.
        let mut other = board.clone();
        assert_eq!(other.place(7, 7, 2), Ok(Placed::Played));
        let mut single = Omok::default();
        for (x, y) in [(5, 7), (6, 7)] {
            single.place(x, y, 1).unwrap();
        }
        assert_eq!(single.place(7, 7, 1), Ok(Placed::Played));
    }

    #[test]
    fn a_blocked_three_is_not_open() {
        let mut board = Omok::default();
        // Across: O X X ?, closed on the left, so no open four can come of
        // it; down: an open three.
        board.place(4, 7, 2).unwrap();
        for (x, y) in [(5, 7), (6, 7), (7, 5), (7, 6)] {
            board.place(x, y, 1).unwrap();
        }
        assert_eq!(board.place(7, 7, 1), Ok(Placed::Played));
    }

    #[test]
    fn an_old_three_does_not_count_as_a_new_one() {
        let mut board = Omok::default();
        // An open three across at (2..=4, 7), and . X X ? down to (7, 7).
        for (x, y) in [(2, 7), (3, 7), (4, 7), (7, 5), (7, 6)] {
            board.place(x, y, 1).unwrap();
        }
        assert_eq!(board.place(7, 7, 1), Ok(Placed::Played));
    }

    #[test]
    fn undo_takes_back_the_last_stone_and_any_after_it() {
        let mut board = Omok::default();
        board.place(0, 0, 1).unwrap();
        board.place(1, 0, 2).unwrap();
        board.place(2, 0, 1).unwrap();
        assert_eq!(board.undo(1), 1);
        assert_eq!(board.undo(2), 1);
        assert!(board.has_stones(1));
        assert!(!board.has_stones(2));
        board.place(1, 1, 2).unwrap();
        // Black's last stone and white's answer to it.
        assert_eq!(board.undo(1), 2);
        assert!(!board.has_stones(1));
        assert_eq!(board.undo(1), 0);
        assert_eq!(board.place(0, 0, 2), Ok(Placed::Played));
    }
}
