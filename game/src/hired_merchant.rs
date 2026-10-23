//! Hired merchants (report 165, plan c007): a character's one merchant on
//! a Free Market map, opened with a permit (503xxxx), which sells what its
//! owner listed while they are away, for 24 hours. When the time runs out
//! it closes and keeps what is left, with its mesos, for Fredrick.

use crate::mini_room::Listing;
use std::time::{Duration, SystemTime};

/// How long a merchant sells once open (the client counts down 24 hours).
pub const LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// Where a merchant is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MerchantState {
    /// Its owner is setting it up, in the room.
    Setup = 0,
    /// Selling.
    Open = 1,
    /// Its time ran out; Fredrick keeps what is left.
    Closed = 2,
}

impl MerchantState {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Setup),
            1 => Some(Self::Open),
            2 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// A row a merchant lists, under its id in the database.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stock {
    pub id: i64,
    pub listing: Listing,
}

/// A sale, for the owner's sold-items list: `quantity` items of `item_id`
/// for `mesos` before tax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sale {
    pub item_id: i32,
    pub quantity: i16,
    pub mesos: i32,
    pub buyer: String,
}

/// Where a merchant stands, as its permit placed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stand {
    pub channel_id: u8,
    pub map_id: i32,
    pub x: i16,
    pub y: i16,
    pub foothold: i16,
}

/// A merchant with what it lists and has sold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiredMerchant {
    pub owner_id: i32,
    pub owner_name: String,
    pub stand: Stand,
    /// The permit, which picks the merchant and its balloon.
    pub item_id: i32,
    pub title: String,
    pub state: MerchantState,
    pub opened_at: Option<SystemTime>,
    pub closed_at: Option<SystemTime>,
    /// What it has made, after tax, and not yet handed over.
    pub meso: i32,
    pub stock: Vec<Stock>,
    pub sales: Vec<Sale>,
}

impl HiredMerchant {
    /// Whether an open merchant's time has run out at `now`.
    pub fn expired(&self, now: SystemTime) -> bool {
        self.state == MerchantState::Open
            && self
                .opened_at
                .and_then(|opened| now.duration_since(opened).ok())
                .is_some_and(|open| open >= LIFETIME)
    }

    /// How long it has been open at `now`, for the owner's countdown.
    pub fn open_for(&self, now: SystemTime) -> Duration {
        self.opened_at
            .and_then(|opened| now.duration_since(opened).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merchant(state: MerchantState, opened_at: Option<SystemTime>) -> HiredMerchant {
        HiredMerchant {
            owner_id: 1,
            owner_name: "Alice".into(),
            stand: Stand {
                channel_id: 0,
                map_id: 910000001,
                x: 0,
                y: 0,
                foothold: 0,
            },
            item_id: 5030000,
            title: "shop".into(),
            state,
            opened_at,
            closed_at: None,
            meso: 0,
            stock: vec![],
            sales: vec![],
        }
    }

    #[test]
    fn open_merchants_expire_after_a_day() {
        let opened = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let open = merchant(MerchantState::Open, Some(opened));
        assert!(!open.expired(opened + LIFETIME - Duration::from_secs(1)));
        assert!(open.expired(opened + LIFETIME));
        assert_eq!(
            open.open_for(opened + Duration::from_secs(90)),
            Duration::from_secs(90)
        );
        // Only open ones run on the clock.
        let setup = merchant(MerchantState::Setup, None);
        assert!(!setup.expired(opened + LIFETIME * 2));
        assert_eq!(setup.open_for(opened), Duration::ZERO);
    }

    #[test]
    fn states_round_trip() {
        for state in [
            MerchantState::Setup,
            MerchantState::Open,
            MerchantState::Closed,
        ] {
            assert_eq!(MerchantState::from_i16(state as i16), Some(state));
        }
        assert_eq!(MerchantState::from_i16(3), None);
    }
}
