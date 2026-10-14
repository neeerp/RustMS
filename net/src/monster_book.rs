//! Monster Book cards: which card each mob drops, and how often.

use crate::load;
use std::collections::HashMap;
use std::sync::OnceLock;

/// v83 card drop chance per kill.
const CARD_DROP_RATE: f64 = 0.01;

/// Mob id to its card, from the cards' `info/mob` in `Item.nx`.
fn cards() -> Result<&'static HashMap<i32, i32>, &'static str> {
    static CARDS: OnceLock<Result<HashMap<i32, i32>, String>> = OnceLock::new();
    load::once(&CARDS, || {
        let data = crate::items::data()?;
        let mut by_mob = HashMap::new();
        for (card, mob) in data.monster_cards().map_err(|e| e.to_string())? {
            by_mob.entry(mob).or_insert(card);
        }
        Ok::<_, String>(by_mob)
    })
}

/// The card `mob_id` drops, if it has one.
pub fn card_for(mob_id: i32) -> Option<i32> {
    cards().ok()?.get(&mob_id).copied()
}

/// The chance a kill drops the mob's card. `RUSTMS_CARD_DROP_RATE`
/// overrides it (the integration harness sets 1 so card tests need not
/// grind).
pub fn drop_rate() -> f64 {
    static RATE: OnceLock<f64> = OnceLock::new();
    *RATE.get_or_init(|| {
        std::env::var("RUSTMS_CARD_DROP_RATE")
            .ok()
            .and_then(|rate| rate.parse::<f64>().ok())
            .filter(|rate| (0.0..=1.0).contains(rate))
            .unwrap_or(CARD_DROP_RATE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobs_drop_the_card_that_names_them() {
        if crate::items::data().is_err() {
            return;
        }
        assert_eq!(card_for(100100), Some(2380000)); // Snail
        assert_eq!(card_for(100101), Some(2380001)); // Blue Snail
        assert_eq!(card_for(9300018), None); // the tutorial's Jr. Sentinel
    }
}
