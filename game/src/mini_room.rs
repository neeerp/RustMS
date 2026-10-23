//! What mini rooms hold besides their members: the rows a personal shop or
//! a hired merchant lists, and the records players keep at the games.

use crate::Item;

/// What a shop lists in one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    /// One bundle.
    pub item: Item,
    /// Bundles left; 0 once sold out.
    pub bundles: i16,
    /// The price of a bundle.
    pub price: i32,
}

impl Listing {
    /// `bundles` bundles as one item.
    pub fn take(&self, bundles: i16) -> Item {
        Item {
            quantity: self.item.quantity * bundles,
            ..self.item.clone()
        }
    }
}

/// A player's record at one of the games, Omok or match cards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameRecord {
    pub wins: i32,
    pub ties: i32,
    pub losses: i32,
}

/// How a game ended for one player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    Win,
    Tie,
    Loss,
}

impl GameRecord {
    pub fn add(&mut self, outcome: GameOutcome) {
        match outcome {
            GameOutcome::Win => self.wins += 1,
            GameOutcome::Tie => self.ties += 1,
            GameOutcome::Loss => self.losses += 1,
        }
    }
}
