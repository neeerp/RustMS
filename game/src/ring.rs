//! Crush (couple) and friendship rings: cash rings bought in pairs, one for
//! the buyer and one sent to a partner (report b061). v83 tells the kinds
//! apart by their item ids alone.

use crate::inventory::equip_slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingKind {
    /// 1112001 to 1112099; 1112000 is a plain ring.
    Crush,
    /// 1112800 to 1112802, 1112810 to 1112812, and so on.
    Friendship,
    /// 1112803, 1112806, 1112807 and 1112809 (`crate::marriage`).
    Wedding,
}

/// The kind of ring `item_id` is, if any. v83 wears one of each kind at a
/// time (StringPool 0x365, 0x366 and 0x10aa).
pub fn kind(item_id: i32) -> Option<RingKind> {
    if crate::marriage::is_wedding_ring(item_id) {
        Some(RingKind::Wedding)
    } else if item_id / 100 == 11120 && item_id != 1112000 {
        Some(RingKind::Crush)
    } else if item_id / 100 == 11128 && item_id % 10 <= 2 {
        Some(RingKind::Friendship)
    } else {
        None
    }
}

/// One ring of a pair; saved by `db::ring`. Its id is the ring's cash
/// serial, and the partner's ring has `partner_ring_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoupleRing {
    pub id: i64,
    pub partner_ring_id: i64,
    pub partner_id: i32,
    pub partner_name: String,
    pub item_id: i32,
}

/// The crush or friendship block of a look: the worn ring's serial, its
/// partner ring's, and its item. Two users pair when each one's partner
/// serial is the other's serial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingLook {
    pub ring_id: i64,
    pub partner_ring_id: i64,
    pub item_id: i32,
}

/// The three ring blocks that end a look, as others see it: the worn crush
/// and friendship rings, and the couple's wedding ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RingLooks {
    pub crush: Option<RingLook>,
    pub friendship: Option<RingLook>,
    pub marriage: Option<crate::marriage::MarriageRing>,
}

/// The positions a cash ring is worn at: the ring slots less 100.
pub fn cash_ring_slots() -> impl Iterator<Item = i16> {
    equip_slot::RINGS.into_iter().map(|slot| slot - 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_go_by_item_id() {
        assert_eq!(kind(1112001), Some(RingKind::Crush));
        assert_eq!(kind(1112099), Some(RingKind::Crush));
        assert_eq!(kind(1112000), None);
        assert_eq!(kind(1112800), Some(RingKind::Friendship));
        assert_eq!(kind(1112812), Some(RingKind::Friendship));
        assert_eq!(kind(1112803), Some(RingKind::Wedding));
        assert_eq!(kind(1112809), Some(RingKind::Wedding));
        assert_eq!(kind(1112804), None);
        assert_eq!(kind(1112100), None);
        assert_eq!(
            cash_ring_slots().collect::<Vec<_>>(),
            [-112, -113, -115, -116]
        );
    }
}
