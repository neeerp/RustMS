//! What equipment asks of its wearer, and which of what a player wears
//! counts toward their stats.
//!
//! The v83 client rebuilds its list of counted equipment on every stat
//! change (FUN_0077e227, report b341): an item whose requirements its wearer
//! no longer meets stays worn but adds nothing, and a weapon that doesn't
//! count can't attack ("You cannot use this weapon with the current
//! stats."). The item data lives in `game-data`, so `net` registers a lookup
//! for it; without one everything worn counts.

use std::sync::OnceLock;

use crate::{CharacterStats, EquipStats, Item};

/// An equipment item's `reqLevel`, `reqJob`, `reqSTR`, `reqDEX`, `reqINT`,
/// `reqLUK` and `reqPOP`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Requirements {
    pub level: i16,
    pub job: i16,
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub fame: i16,
}

/// Looks up an item's requirements, or None for an item with no data.
pub type Lookup = fn(i32) -> Option<Requirements>;

static LOOKUP: OnceLock<Lookup> = OnceLock::new();

/// Use `lookup` for every player's counted equipment from now on. The first
/// lookup registered stays.
pub fn register(lookup: Lookup) {
    let _ = LOOKUP.set(lookup);
}

impl Requirements {
    /// Whether a character with `stats`' level, job and fame, and `totals`
    /// for STR, DEX, INT and LUK, meets them (v83 FUN_004f2cee). GMs meet
    /// any.
    pub fn met_by(&self, stats: &CharacterStats, totals: [i32; 4]) -> bool {
        let branch = stats.job % 1000 / 100;
        if branch == 8 || branch == 9 {
            return true;
        }
        let mask = if branch == 0 { 0 } else { 1 << (branch - 1) };
        let job_ok = match self.job {
            0 => true,
            -1 => mask == 0,
            req => req > 0 && req & mask != 0,
        };
        let needed = [self.str, self.dex, self.int, self.luk];
        job_ok
            && i16::from(stats.level) >= self.level
            && totals
                .iter()
                .zip(needed)
                .all(|(&total, need)| total >= i32::from(need))
            && (self.fame == 0 || stats.fame >= self.fame)
    }
}

/// STR, DEX, INT and LUK of the character's own, raised by Maple Warrior's
/// `percent` (BasicStat::SetFrom, 0x77ec9f).
pub fn own_stats(stats: &CharacterStats, percent: i32) -> [i32; 4] {
    [stats.str, stats.dex, stats.int, stats.luk].map(|stat| {
        let stat = i32::from(stat);
        stat + stat * percent / 100
    })
}

/// The worn slot an item's place in the loop is taken from: cash items
/// 100 below their slot share their slot's turn, after its normal item.
fn order(position: i16) -> (i32, i32, bool) {
    let index = -i32::from(position);
    match index {
        1000.. => (1, index, false),
        101..=199 => (0, index - 100, true),
        _ => (0, index, false),
    }
}

/// Whether a slot's item adds its STR, DEX, INT and LUK to what the others
/// are checked with. Pet equipment doesn't, except the first pet's clothes.
fn adds_stats(index: i32) -> bool {
    index != 30 && index != 38 && (index == 14 || index <= 20 || index > 48)
}

/// Whether a slot's item is checked against its wearer. Pet equipment asks
/// things of its pet instead, which the worn check doesn't see here.
fn checked(index: i32) -> bool {
    index != 14 && adds_stats(index)
}

fn bonus(equip: &EquipStats) -> [i32; 4] {
    [equip.str, equip.dex, equip.int, equip.luk].map(i32::from)
}

/// Which of the `worn` items (position, item) count, in their order
/// (FUN_0077e227):
/// - Each is checked with the character's own stats plus every other
///   counted item's, never its own.
/// - Normal items go before cash ones in each slot, slots in order, the
///   dragon's last.
/// - The first to fail stops counting, and the check starts over, until a
///   whole pass drops none.
/// - A cash weapon counts only over a counted weapon, and a cash shield
///   only over a counted shield.
pub fn counted(
    stats: &CharacterStats,
    percent: i32,
    worn: &[(i16, &Item)],
    lookup: Lookup,
) -> Vec<bool> {
    let mut order_of: Vec<usize> = (0..worn.len()).collect();
    order_of.sort_by_key(|&i| order(worn[i].0));
    let requirements: Vec<Option<Requirements>> =
        worn.iter().map(|(_, item)| lookup(item.item_id)).collect();
    let own = own_stats(stats, percent);
    let mut counts = vec![true; worn.len()];
    let mut added = [0; 4];
    for &i in &order_of {
        if let (true, Some(equip)) = (adds_stats(order(worn[i].0).1), &worn[i].1.equip) {
            for (total, add) in added.iter_mut().zip(bonus(equip)) {
                *total += add;
            }
        }
    }
    let counted_at = |counts: &[bool], position: i16| {
        worn.iter()
            .zip(counts)
            .any(|((at, _), &counts)| *at == position && counts)
    };
    'scan: loop {
        for &i in &order_of {
            if !counts[i] {
                continue;
            }
            let (position, item) = worn[i];
            let (_, index, cash) = order(position);
            let mine = match (&item.equip, adds_stats(index)) {
                (Some(equip), true) => bonus(equip),
                _ => [0; 4],
            };
            let over = match (cash, index) {
                (true, 10) => item.item_id / 10000 == 109,
                (true, 11) => item.item_id / 10000 == 170,
                _ => false,
            };
            let under = !over || counted_at(&counts, -(index as i16));
            let mut totals = own;
            for ((total, add), mine) in totals.iter_mut().zip(added).zip(mine) {
                *total += add - mine;
            }
            let met =
                !checked(index) || requirements[i].is_none_or(|needs| needs.met_by(stats, totals));
            if !(met && under) {
                counts[i] = false;
                for (total, mine) in added.iter_mut().zip(mine) {
                    *total -= mine;
                }
                continue 'scan;
            }
        }
        return counts;
    }
}

/// Which of the `worn` items count, with the registered lookup; all of them
/// when there is none.
pub fn counted_worn(stats: &CharacterStats, percent: i32, worn: &[(i16, &Item)]) -> Vec<bool> {
    match LOOKUP.get() {
        Some(&lookup) => counted(stats, percent, worn, lookup),
        None => vec![true; worn.len()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::test_stats;

    // Gloves give 5 STR; the sword needs 15 STR and a warrior; the shield
    // needs 10 STR and the gloves' STR; 1702000 is a cash weapon.
    fn lookup(item_id: i32) -> Option<Requirements> {
        match item_id {
            1302000 => Some(Requirements {
                str: 15,
                job: 1,
                ..Requirements::default()
            }),
            1092000 => Some(Requirements {
                str: 10,
                ..Requirements::default()
            }),
            1082002 | 1702000 => Some(Requirements::default()),
            _ => None,
        }
    }

    fn equip(item_id: i32, str: i16) -> Item {
        Item::equipment(
            item_id,
            EquipStats {
                str,
                ..EquipStats::default()
            },
        )
    }

    #[test]
    fn the_job_bits_level_and_fame_decide() {
        let mut stats = test_stats();
        let beginners = Requirements {
            job: -1,
            ..Requirements::default()
        };
        let warriors = Requirements {
            job: 1 | 8,
            level: 10,
            ..Requirements::default()
        };
        assert!(beginners.met_by(&stats, [0; 4]));
        assert!(!warriors.met_by(&stats, [0; 4]));
        stats.job = 2110;
        stats.level = 10;
        assert!(!beginners.met_by(&stats, [0; 4]));
        assert!(warriors.met_by(&stats, [0; 4]));
        stats.level = 9;
        assert!(!warriors.met_by(&stats, [0; 4]));
        stats.job = 900;
        assert!(warriors.met_by(&stats, [0; 4]), "GMs wear anything");

        let famous = Requirements {
            fame: 1,
            ..Requirements::default()
        };
        stats.job = 0;
        stats.fame = -3;
        assert!(Requirements::default().met_by(&stats, [0; 4]));
        assert!(!famous.met_by(&stats, [0; 4]));
    }

    #[test]
    fn an_item_counts_on_the_others_stats_and_never_its_own() {
        let mut stats = test_stats();
        stats.job = 100;
        stats.str = 10;
        let gloves = equip(1082002, 5);
        let sword = equip(1302000, 0);
        let worn = [(-11, &sword), (-8, &gloves)];
        assert_eq!(counted(&stats, 0, &worn, lookup), [true, true]);
        assert_eq!(counted(&stats, 0, &worn[..1], lookup), [false]);
        assert_eq!(
            counted(&stats, 50, &worn[..1], lookup),
            [true],
            "Maple Warrior's STR counts"
        );

        // A shield needing 10 STR that gives 10 doesn't hold itself up.
        stats.str = 5;
        let shield = equip(1092000, 10);
        assert_eq!(counted(&stats, 0, &[(-10, &shield)], lookup), [false]);
        assert_eq!(
            counted(&stats, 0, &[(-10, &shield), (-8, &gloves)], lookup),
            [true, true]
        );
    }

    #[test]
    fn a_dropped_item_takes_its_stats_from_the_rest_until_none_drops() {
        let mut stats = test_stats();
        stats.job = 100;
        stats.str = 5;
        // The shield (5 + 5) lets the sword pass, but fails itself at 5
        // with the sword's nothing; the sword then fails too.
        let shield = equip(1092000, 5);
        let gloves = equip(1082002, 0);
        let sword = equip(1302000, 0);
        let worn = [(-11, &sword), (-10, &shield), (-8, &gloves)];
        assert_eq!(counted(&stats, 0, &worn, lookup), [false, false, true]);
    }

    #[test]
    fn a_cash_weapon_needs_a_counted_weapon_under_it() {
        let mut stats = test_stats();
        stats.job = 100;
        stats.str = 20;
        let cash = equip(1702000, 0);
        let sword = equip(1302000, 0);
        assert_eq!(counted(&stats, 0, &[(-111, &cash)], lookup), [false]);
        assert_eq!(
            counted(&stats, 0, &[(-111, &cash), (-11, &sword)], lookup),
            [true, true]
        );
        stats.str = 10;
        assert_eq!(
            counted(&stats, 0, &[(-111, &cash), (-11, &sword)], lookup),
            [false, false]
        );
    }
}
