use std::collections::BTreeMap;

/// A card fills its page after this many copies; later copies are wasted.
pub const MAX_CARD_LEVEL: u8 = 5;

/// Whether `item_id` is a Monster Book card (`Item.nx/Consume/0238.img`).
pub fn is_card(item_id: i32) -> bool {
    item_id / 10_000 == 238
}

/// A character's Monster Book: how many of each card they have collected
/// (1 to 5), and the card shown on the book's cover (0 for none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MonsterBook {
    cover: i32,
    cards: BTreeMap<i32, u8>,
}

impl MonsterBook {
    pub fn new(cover: i32, cards: impl IntoIterator<Item = (i32, u8)>) -> Self {
        let cards = cards
            .into_iter()
            .filter(|&(card, level)| is_card(card) && level > 0)
            .map(|(card, level)| (card, level.min(MAX_CARD_LEVEL)))
            .collect();
        Self { cover, cards }
    }

    pub fn cover(&self) -> i32 {
        self.cover
    }

    /// How many copies of `card` are in the book.
    pub fn level(&self, card: i32) -> u8 {
        self.cards.get(&card).copied().unwrap_or(0)
    }

    pub fn cards(&self) -> impl Iterator<Item = (i32, u8)> + '_ {
        self.cards.iter().map(|(&card, &level)| (card, level))
    }

    /// Put a picked-up card in the book. Returns its new level, or `None`
    /// if the page was already full.
    pub fn add(&mut self, card: i32) -> Option<u8> {
        let level = self.cards.entry(card).or_insert(0);
        if *level >= MAX_CARD_LEVEL {
            return None;
        }
        *level += 1;
        Some(*level)
    }

    /// Show `card` on the cover; 0 clears it. Only collected cards can be
    /// shown.
    pub fn set_cover(&mut self, card: i32) -> bool {
        if card != 0 && self.level(card) == 0 {
            return false;
        }
        self.cover = card;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAIL_CARD: i32 = 2_380_000;

    #[test]
    fn a_page_holds_five_copies() {
        let mut book = MonsterBook::default();
        for level in 1..=5 {
            assert_eq!(book.add(SNAIL_CARD), Some(level));
        }
        assert_eq!(book.add(SNAIL_CARD), None);
        assert_eq!(book.level(SNAIL_CARD), 5);
        assert_eq!(book.cards().collect::<Vec<_>>(), [(SNAIL_CARD, 5)]);
    }

    #[test]
    fn only_collected_cards_go_on_the_cover() {
        let mut book = MonsterBook::new(0, [(SNAIL_CARD, 2)]);
        assert!(!book.set_cover(2_380_001));
        assert!(book.set_cover(SNAIL_CARD));
        assert_eq!(book.cover(), SNAIL_CARD);
        assert!(book.set_cover(0));
        assert_eq!(book.cover(), 0);
    }

    #[test]
    fn loading_drops_non_cards_and_caps_levels() {
        let book = MonsterBook::new(0, [(SNAIL_CARD, 9), (2_000_000, 1), (2_380_001, 0)]);
        assert_eq!(book.cards().collect::<Vec<_>>(), [(SNAIL_CARD, 5)]);
        assert!(is_card(SNAIL_CARD));
        assert!(!is_card(2_000_000));
    }
}
