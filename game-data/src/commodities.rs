//! Cash Shop commodities from `Etc.nx` (`Commodity.img`), keyed by their
//! serial number (SN). The client lists the same entries, so a purchase
//! names a commodity by SN.

use crate::{GameDataError, NxDataFile};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Commodity {
    pub sn: i32,
    pub item_id: i32,
    /// How many of the item one purchase gives.
    pub count: i16,
    pub price: i32,
    /// Days the item lasts; 0 is forever.
    pub period: i32,
    /// 0 male, 1 female, 2 either.
    pub gender: i8,
    pub on_sale: bool,
}

impl Commodity {
    /// Whether a character of `gender` (0 male, 1 female) may buy it.
    pub fn for_gender(&self, gender: i16) -> bool {
        !matches!(self.gender, 0 | 1) || i16::from(self.gender) == gender
    }
}

#[derive(Debug, Default)]
pub struct CommodityData {
    by_sn: HashMap<i32, Commodity>,
}

impl CommodityData {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let root = nx
            .child_by_name(0, "Commodity.img")?
            .ok_or_else(|| GameDataError::InvalidData("missing Commodity.img".to_string()))?;
        let mut by_sn = HashMap::new();
        for entry in nx.child_indices(root)? {
            let int =
                |key: &str| -> Result<Option<i32>, GameDataError> { nx.int_child(entry, key) };
            let Some(sn) = int("SN")? else {
                continue;
            };
            let commodity = Commodity {
                sn,
                item_id: int("ItemId")?.unwrap_or(0),
                count: i16::try_from(int("Count")?.unwrap_or(1))
                    .unwrap_or(1)
                    .max(1),
                price: int("Price")?.unwrap_or(0),
                period: int("Period")?.unwrap_or(0),
                gender: i8::try_from(int("Gender")?.unwrap_or(2)).unwrap_or(2),
                on_sale: int("OnSale")? == Some(1),
            };
            by_sn.insert(sn, commodity);
        }
        Ok(Self { by_sn })
    }

    pub fn get(&self, sn: i32) -> Option<&Commodity> {
        self.by_sn.get(&sn)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Commodity> {
        self.by_sn.values()
    }

    pub fn len(&self) -> usize {
        self.by_sn.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_sn.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_commodities_by_serial_number() {
        let Some(path) = crate::test_assets::path("Etc.nx") else {
            return;
        };
        let data = CommodityData::load(path).unwrap();
        assert_eq!(data.len(), 8941);

        // Etc > Game: 35 Gachapon Tickets, labelled HOT.
        let tickets = data.get(50200003).unwrap();
        assert_eq!(tickets.item_id, 5220000);
        assert_eq!(
            (tickets.count, tickets.price, tickets.period),
            (35, 30000, 90)
        );
        assert!(tickets.on_sale && tickets.for_gender(0) && tickets.for_gender(1));

        // A hat that is no longer sold.
        let hat = data.get(20000000).unwrap();
        assert_eq!((hat.item_id, hat.price), (1002000, 300));
        assert!(!hat.on_sale);

        // Men's equipment.
        let male = data.get(20000053).unwrap();
        assert!(male.for_gender(0) && !male.for_gender(1));

        assert!(data.get(-1).is_none());
    }
}
