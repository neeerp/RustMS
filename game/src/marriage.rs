//! Engagement and marriage (report b052): a player proposes with an
//! engagement ring box, and the couple's record rides in the character
//! data and MARRIAGE_RESULT.

/// The record's `status` while the couple is engaged.
pub const ENGAGED: i16 = 1;
/// The record's `status` once the couple is married.
pub const MARRIED: i16 = 3;
/// The lowest level at which either may propose or be proposed to (a
/// recommended default; the client checks no level).
pub const MIN_LEVEL: u8 = 50;
/// The longest name the client's proposal box takes.
pub const MAX_NAME: usize = 12;

/// The engagement ring boxes, Moonstone, Star gem, Golden Heart and Silver
/// Swan, that a player proposes with.
pub fn is_ring_box(item_id: i32) -> bool {
    (2240000..=2240003).contains(&item_id)
}

/// What each partner gets when a proposal with `ring_box` is accepted: the
/// proposer the empty box, the partner the ring (4031357 to 4031364, which
/// the client treats as engagement tokens).
pub fn tokens(ring_box: i32) -> (i32, i32) {
    let empty_box = 4031357 + 2 * (ring_box - 2240000);
    (empty_box, empty_box + 1)
}

/// The wedding rings, one for each ring box in order. The client knows
/// them by id alone, and draws a couple's pair effect from
/// `Effect/ItemEff.img/<id>` (report b059).
pub const WEDDING_RINGS: [i32; 4] = [1112803, 1112806, 1112807, 1112809];

pub fn is_wedding_ring(item_id: i32) -> bool {
    WEDDING_RINGS.contains(&item_id)
}

/// The empty boxes and rings a couple holds. The client lets a player drop
/// one only to break off (report b054).
pub fn is_token(item_id: i32) -> bool {
    (4031357..=4031364).contains(&item_id)
}

/// A couple, engaged or married: GW_MarriageRecord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarriageRecord {
    pub id: i32,
    pub groom_id: i32,
    pub bride_id: i32,
    pub status: i16,
    /// The ring box the proposal was made with.
    pub item_id: i32,
    pub groom_name: String,
    pub bride_name: String,
}

impl MarriageRecord {
    pub fn married(&self) -> bool {
        self.status == MARRIED
    }

    /// The wedding ring that goes with the couple's ring box, which the
    /// record names as both the groom's and the bride's ring.
    pub fn ring_id(&self) -> i32 {
        let index = usize::try_from(self.item_id - 2240000).unwrap_or(0);
        WEDDING_RINGS[index.min(WEDDING_RINGS.len() - 1)]
    }

    /// The other one of the couple, for one of them, `id`.
    pub fn partner_of(&self, id: i32) -> i32 {
        if self.groom_id == id {
            self.bride_id
        } else {
            self.groom_id
        }
    }

    /// The other one's name, for one of the couple, `id`.
    pub fn partner_name_of(&self, id: i32) -> &str {
        if self.groom_id == id {
            &self.bride_name
        } else {
            &self.groom_name
        }
    }
}

/// The marriage block of a player's look: the player, the partner and the
/// ring whose effect the two show near each other (report b059). The
/// client pairs two players when one's partner is the other's self.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarriageRing {
    pub self_id: i32,
    pub partner_id: i32,
    pub item_id: i32,
}

/// Why a proposal failed, or what became of it: MARRIAGE_RESULT's mode,
/// which picks the client's line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// 0x1090: no one by that name is online, or it is the player's own.
    WrongName = 18,
    /// 0x1091: the partner isn't on the same map.
    NotSameMap = 19,
    /// 0x109f: the player's ETC tab is full.
    EtcFull = 20,
    /// 0x1092: the partner's ETC tab is full.
    PartnerEtcFull = 21,
    /// 0x1093: the two are of the same gender.
    SameGender = 22,
    /// 0x1094: the player is engaged already.
    Engaged = 23,
    /// 0x1096: the partner is engaged already.
    PartnerEngaged = 24,
    /// 0x1095: the player is married already.
    Married = 25,
    /// 0x1097: the partner is married already.
    PartnerMarried = 26,
    /// 0x1098: the player is waiting on an answer already.
    Proposing = 27,
    /// 0x1099: someone else is proposing to the partner.
    BeingAsked = 28,
    /// 0x109a: to the partner, the proposer withdrew.
    Withdrew = 29,
    /// 0x108d: to the proposer, the partner said no.
    Declined = 30,
}

impl Refusal {
    /// The refusal for a player who already has `record`, in the player's
    /// own words or (`partner`) of the one asked.
    pub fn taken(record: &MarriageRecord, partner: bool) -> Self {
        match (record.married(), partner) {
            (true, false) => Self::Married,
            (false, false) => Self::Engaged,
            (true, true) => Self::PartnerMarried,
            (false, true) => Self::PartnerEngaged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_box_gives_its_own_empty_box_and_ring() {
        assert_eq!(tokens(2240000), (4031357, 4031358));
        assert_eq!(tokens(2240003), (4031363, 4031364));
        assert!(is_ring_box(2240002));
        assert!(!is_ring_box(2240004));
        assert!(is_token(4031357) && is_token(4031364));
        assert!(!is_token(4031365));
    }

    #[test]
    fn each_box_gives_its_own_wedding_ring() {
        let record = |item_id| MarriageRecord {
            id: 1,
            groom_id: 4,
            bride_id: 5,
            status: 1,
            item_id,
            groom_name: String::new(),
            bride_name: String::new(),
        };
        let rings: Vec<i32> = (2240000..=2240003).map(|b| record(b).ring_id()).collect();
        assert_eq!(rings, WEDDING_RINGS);
        assert!(rings.iter().all(|&ring| is_wedding_ring(ring)));
    }
}
