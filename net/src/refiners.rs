//! The refiners who turn ore into plates, jewels and crystals, and wood
//! and plates into materials and arrows, as many at a time as the player
//! asks: Mr. Thunder in Perion (1022003), Chris in Kerning City (1052003),
//! Vogen in El Nath (2020000), Pi in Ludibrium (2040016), Muhamad in
//! Ariant (2100001) and Spiruna in Orbis (2032001); and the five in
//! Altaire Camp who trade a hundred of something for a Perfect Pitch.
//! Items, materials and prices follow Cosmic's scripts.

use crate::item_makers::{recipe, Recipe};

/// One thing a refiner makes in bulk: `yields` of `item` for each set of
/// `materials` (item and quantity) and `cost` mesos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refine {
    pub item: i32,
    pub yields: i16,
    pub materials: &'static [(i32, i16)],
    pub cost: i32,
}

const fn refine(item: i32, materials: &'static [(i32, i16)], cost: i32) -> Refine {
    Refine {
        item,
        yields: 1,
        materials,
        cost,
    }
}

impl Refine {
    /// The mesos `times` sets cost at `percent` of the price.
    pub fn meso(&self, times: i32, percent: i32) -> i32 {
        self.cost * percent / 100 * times
    }
}

/// The most sets a refiner makes at once.
pub const MOST_AT_ONCE: i32 = 100;

pub const MINERALS: &[Refine] = &[
    refine(4011000, &[(4010000, 10)], 300),
    refine(4011001, &[(4010001, 10)], 300),
    refine(4011002, &[(4010002, 10)], 300),
    refine(4011003, &[(4010003, 10)], 500),
    refine(4011004, &[(4010004, 10)], 500),
    refine(4011005, &[(4010005, 10)], 500),
    refine(4011006, &[(4010006, 10)], 800),
];

/// Muhamad refines Lidium Ore as well.
pub const MINERALS_AND_LIDIUM: &[Refine] = &[
    refine(4011000, &[(4010000, 10)], 300),
    refine(4011001, &[(4010001, 10)], 300),
    refine(4011002, &[(4010002, 10)], 300),
    refine(4011003, &[(4010003, 10)], 500),
    refine(4011004, &[(4010004, 10)], 500),
    refine(4011005, &[(4010005, 10)], 500),
    refine(4011006, &[(4010006, 10)], 800),
    refine(4011008, &[(4010007, 10)], 300),
];

pub const JEWELS: &[Refine] = &[
    refine(4021000, &[(4020000, 10)], 500),
    refine(4021001, &[(4020001, 10)], 500),
    refine(4021002, &[(4020002, 10)], 500),
    refine(4021003, &[(4020003, 10)], 500),
    refine(4021004, &[(4020004, 10)], 500),
    refine(4021005, &[(4020005, 10)], 500),
    refine(4021006, &[(4020006, 10)], 500),
    refine(4021007, &[(4020007, 10)], 1000),
    refine(4021008, &[(4020008, 10)], 3000),
];

/// A Moon Rock from one of each plate, a Star Rock from one of each jewel.
pub const ROCKS: &[Refine] = &[
    refine(
        4011007,
        &[
            (4011000, 1),
            (4011001, 1),
            (4011002, 1),
            (4011003, 1),
            (4011004, 1),
            (4011005, 1),
            (4011006, 1),
        ],
        10000,
    ),
    refine(
        4021009,
        &[
            (4021000, 1),
            (4021001, 1),
            (4021002, 1),
            (4021003, 1),
            (4021004, 1),
            (4021005, 1),
            (4021006, 1),
            (4021007, 1),
            (4021008, 1),
        ],
        15000,
    ),
];

pub const CRYSTALS: &[Refine] = &[
    refine(4005000, &[(4004000, 10)], 5000),
    refine(4005001, &[(4004001, 10)], 5000),
    refine(4005002, &[(4004002, 10)], 5000),
    refine(4005003, &[(4004003, 10)], 5000),
];

/// Vogen refines Dark Crystal Ore too.
pub const CRYSTALS_AND_DARK: &[Refine] = &[
    refine(4005000, &[(4004000, 10)], 5000),
    refine(4005001, &[(4004001, 10)], 5000),
    refine(4005002, &[(4004002, 10)], 5000),
    refine(4005003, &[(4004003, 10)], 5000),
    refine(4005004, &[(4004004, 10)], 1_000_000),
];

/// Processed Wood from Tree Branches or Firewood, and Screws.
pub const MATERIALS: &[Refine] = &[
    refine(4003001, &[(4000003, 10)], 0),
    refine(4003001, &[(4000018, 5)], 0),
    Refine {
        item: 4003000,
        yields: 15,
        materials: &[(4011000, 1), (4011001, 1)],
        cost: 0,
    },
];

const fn arrows(item: i32, yields: i16, materials: &'static [(i32, i16)]) -> Refine {
    Refine {
        item,
        yields,
        materials,
        cost: 0,
    }
}

/// Arrows come a bundle at a time, without asking how many.
pub const ARROWS: &[Refine] = &[
    arrows(2060000, 1000, &[(4003001, 1), (4003004, 1)]),
    arrows(2061000, 1000, &[(4003001, 1), (4003004, 1)]),
    arrows(2060001, 900, &[(4011000, 1), (4003001, 3), (4003004, 10)]),
    arrows(2061001, 900, &[(4011000, 1), (4003001, 3), (4003004, 10)]),
    arrows(2060002, 800, &[(4011001, 1), (4003001, 5), (4003005, 15)]),
    arrows(2061002, 800, &[(4011001, 1), (4003001, 5), (4003005, 15)]),
];

/// Chris makes a Steel Plate from a hundred Iron Hog's Metal Hoofs.
pub const METAL_HOOF: Refine = refine(4011001, &[(4000039, 100)], 1000);

/// Spiruna's Dark Crystal, cheaper than Vogen's.
pub const SPIRUNA_DARK_CRYSTAL: Refine = refine(4005004, &[(4004004, 10)], 500_000);

pub const THUNDER_HELMETS: &[Recipe] = &[
    recipe(1002042, &[(1002001, 1), (4011002, 1)], 500),
    recipe(1002041, &[(1002001, 1), (4021006, 1)], 300),
    recipe(1002002, &[(1002043, 1), (4011001, 1)], 500),
    recipe(1002044, &[(1002043, 1), (4011002, 1)], 800),
    recipe(1002003, &[(1002039, 1), (4011001, 1)], 500),
    recipe(1002040, &[(1002039, 1), (4011002, 1)], 800),
    recipe(1002007, &[(1002051, 1), (4011001, 2)], 1000),
    recipe(1002052, &[(1002051, 1), (4011002, 2)], 1500),
    recipe(1002011, &[(1002059, 1), (4011001, 3)], 1500),
    recipe(1002058, &[(1002059, 1), (4011002, 3)], 2000),
    recipe(1002009, &[(1002055, 1), (4011001, 3)], 1500),
    recipe(1002056, &[(1002055, 1), (4011002, 3)], 2000),
    recipe(1002087, &[(1002027, 1), (4011002, 4)], 2000),
    recipe(1002088, &[(1002027, 1), (4011006, 4)], 4000),
    recipe(1002050, &[(1002005, 1), (4011005, 5)], 4000),
    recipe(1002049, &[(1002005, 1), (4011006, 5)], 5000),
    recipe(1002047, &[(1002004, 1), (4021000, 3)], 8000),
    recipe(1002048, &[(1002004, 1), (4021005, 3)], 10000),
    recipe(1002099, &[(1002021, 1), (4011002, 5)], 12000),
    recipe(1002098, &[(1002021, 1), (4011006, 6)], 15000),
    recipe(1002085, &[(1002086, 1), (4011002, 5)], 20000),
    recipe(1002028, &[(1002086, 1), (4011004, 4)], 25000),
    recipe(1002022, &[(1002100, 1), (4011007, 1), (4011001, 7)], 30000),
    recipe(1002101, &[(1002100, 1), (4011007, 1), (4011002, 7)], 30000),
];

pub const THUNDER_SHIELDS: &[Recipe] = &[
    recipe(1092014, &[(1092012, 1), (4011003, 10)], 100000),
    recipe(1092013, &[(1092012, 1), (4011002, 10)], 100000),
    recipe(
        1092010,
        &[(1092009, 1), (4011007, 1), (4011004, 15)],
        120000,
    ),
    recipe(
        1092011,
        &[(1092009, 1), (4011007, 1), (4011003, 15)],
        120000,
    ),
];

pub const CHRIS_CLAWS: &[Recipe] = &[
    recipe(
        1472023,
        &[(1472022, 1), (4011007, 1), (4021000, 8), (2012000, 10)],
        80000,
    ),
    recipe(
        1472024,
        &[(1472022, 1), (4011007, 1), (4021005, 8), (2012002, 10)],
        80000,
    ),
    recipe(
        1472025,
        &[(1472022, 1), (4011007, 1), (4021008, 3), (4000046, 5)],
        100000,
    ),
];

/// The Altaire campers, each with the item they take a hundred of for a
/// Perfect Pitch: Perzen, Loha, Sion, Duru and Tess.
pub const PITCH_TRADERS: &[(i32, i32)] = &[
    (2131001, 4000439),
    (2131003, 4000437),
    (2131005, 4000436),
    (2131006, 4000440),
    (2131007, 4000438),
];

pub const PERFECT_PITCH: i32 = 4310000;
/// Items given for each Perfect Pitch.
pub const PER_PITCH: i32 = 100;
/// The most Perfect Pitches traded at once.
pub const MOST_PITCHES: i32 = 300;

/// How many Perfect Pitches `held` items buy at once.
pub fn pitches(held: i32) -> i32 {
    (held / PER_PITCH).min(MOST_PITCHES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hundred_sets_of_the_dearest_still_fit() {
        let dark = CRYSTALS_AND_DARK.last().unwrap();
        assert_eq!(dark.meso(MOST_AT_ONCE, 100), 100_000_000);
        let most_material = [
            MINERALS_AND_LIDIUM,
            JEWELS,
            ROCKS,
            CRYSTALS_AND_DARK,
            MATERIALS,
            &[METAL_HOOF, SPIRUNA_DARK_CRYSTAL],
        ]
        .iter()
        .flat_map(|list| list.iter())
        .flat_map(|refine| refine.materials.iter().map(|&(_, n)| i32::from(n)))
        .max()
        .unwrap();
        assert!(most_material * MOST_AT_ONCE <= i32::from(i16::MAX));
        assert!(i32::from(MATERIALS[2].yields) * MOST_AT_ONCE <= i32::from(i16::MAX));
        assert!(MOST_PITCHES * PER_PITCH <= i32::from(i16::MAX));
    }

    #[test]
    fn the_discount_is_a_tenth_off() {
        assert_eq!(MINERALS[0].meso(1, 90), 270);
        assert_eq!(MINERALS[6].meso(1, 90), 720);
        assert_eq!(JEWELS[8].meso(1, 90), 2700);
        assert_eq!(ROCKS[1].meso(2, 90), 27000);
        assert_eq!(CRYSTALS[0].meso(100, 90), 450_000);
    }

    #[test]
    fn pitches_need_a_hundred_each_and_stop_at_three_hundred() {
        assert_eq!(pitches(99), 0);
        assert_eq!(pitches(250), 2);
        assert_eq!(pitches(40_000), 300);
    }
}
