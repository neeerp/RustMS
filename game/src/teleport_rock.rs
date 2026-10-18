//! The maps a character keeps for their teleport rocks: five for the
//! regular rock (504000x) and ten for the VIP one (5041000). The v83 client
//! holds each list as fixed slots, the empty ones `EMPTY` (report 179, 4).

/// The map id of an empty slot.
pub const EMPTY: i32 = 999_999_999;

/// Whether `item_id` is a VIP rock, which keeps ten maps and crosses
/// continents (v83 FUN_00a0a63f: `item_id / 1000 != 5040`).
pub fn is_vip(item_id: i32) -> bool {
    item_id / 1000 != 5040
}

/// How many maps a list keeps.
pub fn capacity(vip: bool) -> usize {
    if vip {
        10
    } else {
        5
    }
}

/// Whether the client lets a player keep `map_id`: not Maple Island and
/// not an event map (v83 FUN_0083a084).
pub fn may_keep(map_id: i32) -> bool {
    map_id >= 100_000_000 && (map_id / 1_000_000) % 100 != 9
}

/// The continent of `map_id`, which a regular rock can't leave.
pub fn continent(map_id: i32) -> i32 {
    map_id / 100_000_000
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeleportRocks {
    regular: Vec<i32>,
    vip: Vec<i32>,
}

impl TeleportRocks {
    /// The lists as saved, cut to their capacity.
    pub fn new(mut regular: Vec<i32>, mut vip: Vec<i32>) -> Self {
        regular.truncate(capacity(false));
        vip.truncate(capacity(true));
        Self { regular, vip }
    }

    /// The maps kept in a list, in order.
    pub fn maps(&self, vip: bool) -> &[i32] {
        if vip {
            &self.vip
        } else {
            &self.regular
        }
    }

    /// Every slot of a list, as the client holds it.
    pub fn slots(&self, vip: bool) -> impl Iterator<Item = i32> + '_ {
        let maps = self.maps(vip);
        (0..capacity(vip)).map(move |i| maps.get(i).copied().unwrap_or(EMPTY))
    }

    /// Keep `map_id` at the end of a list; false if it is full or already
    /// keeps it.
    pub fn keep(&mut self, vip: bool, map_id: i32) -> bool {
        let maps = if vip {
            &mut self.vip
        } else {
            &mut self.regular
        };
        if maps.len() >= capacity(vip) || maps.contains(&map_id) {
            return false;
        }
        maps.push(map_id);
        true
    }

    /// Drop `map_id` from a list; the ones after it move up.
    pub fn remove(&mut self, vip: bool, map_id: i32) {
        let maps = if vip {
            &mut self.vip
        } else {
            &mut self.regular
        };
        maps.retain(|&kept| kept != map_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_fills_up_and_closes_up() {
        let mut rocks = TeleportRocks::default();
        for map in 0..5 {
            assert!(rocks.keep(false, 100_000_000 + map));
        }
        assert!(!rocks.keep(false, 200_000_000), "full");
        assert!(rocks.keep(true, 200_000_000), "the VIP list is apart");
        assert!(!rocks.keep(true, 200_000_000), "kept already");
        rocks.remove(false, 100_000_001);
        assert_eq!(
            rocks.slots(false).collect::<Vec<_>>(),
            [100_000_000, 100_000_002, 100_000_003, 100_000_004, EMPTY]
        );
        assert_eq!(rocks.slots(true).count(), 10);
    }

    #[test]
    fn which_rocks_and_maps() {
        assert!(!is_vip(5_040_000));
        assert!(!is_vip(5_040_001));
        assert!(is_vip(5_041_000));
        assert!(may_keep(100_000_000));
        assert!(!may_keep(40_000), "Maple Island");
        assert!(!may_keep(109_010_000), "an event map");
        assert_eq!(continent(220_000_000), 2);
    }
}
