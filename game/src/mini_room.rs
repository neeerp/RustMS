//! What mini rooms hold besides their members: the rows a personal shop or
//! a hired merchant lists.

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
