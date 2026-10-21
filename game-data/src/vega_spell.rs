//! The scrolls Vega's Spell works on, from `Etc.nx` (`VegaSpell.img`):
//! each entry's `item` and `prob`, the scroll's own success rate (0.1 or
//! 0.6). The client lets only these into the Vega window.

use crate::{GameDataError, NxDataFile};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct VegaSpells {
    /// Scroll id to its success rate in percent.
    rates: HashMap<i32, i32>,
}

impl VegaSpells {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut rates = HashMap::new();
        let Some(list) = nx.child_by_name(0, "VegaSpell.img")? else {
            return Ok(Self { rates });
        };
        for entry in nx.child_indices(list)? {
            let Some(item) = nx.int_child(entry, "item")? else {
                continue;
            };
            // The converter kept some reals as text tagged with their type.
            let prob = match nx.real_child(entry, "prob")? {
                Some(prob) => Some(prob),
                None => nx
                    .string_child(entry, "prob")?
                    .and_then(|text| text.trim_start_matches("[R8]").parse().ok()),
            };
            if let Some(prob) = prob {
                rates.insert(item, (prob * 100.0f64).round() as i32);
            }
        }
        Ok(Self { rates })
    }

    /// The success rate in percent of `scroll_id`, if Vega's Spell takes it.
    pub fn rate(&self, scroll_id: i32) -> Option<i32> {
        self.rates.get(&scroll_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_holds_ten_and_sixty_percent_scrolls() {
        let Some(path) = crate::test_assets::path("Etc.nx") else {
            return;
        };
        let spells = VegaSpells::load(path).unwrap();
        assert_eq!(spells.rates.len(), 199);
        // Helmet for DEF 10% and 60%; Chaos Scroll is not on the list.
        assert_eq!(spells.rate(2040002), Some(10));
        assert_eq!(spells.rate(2040001), Some(60));
        assert_eq!(spells.rate(2049100), None);
        assert!(spells.rates.values().all(|&rate| rate == 10 || rate == 60));
    }
}
