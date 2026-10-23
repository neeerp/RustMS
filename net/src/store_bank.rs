//! Fredrick's Store Bank (report 165 §4): where a hired merchant's owner
//! takes back what it held when it closed without them, its unsold items
//! and the mesos it made, for a storing fee. Talking to Fredrick opens the
//! Store Bank (STORE_BANK 311); its Get All asks for the fee
//! (STORE_BANK_REQUEST 0x40) and the confirm takes everything at once, or
//! nothing (STORE_BANK_MESSAGE 310).

use std::time::{Duration, SystemTime};

/// Fredrick, at the Free Market Entrance.
pub const FREDRICK: i32 = 9030000;

/// The fee is charged for each whole day kept, up to the whole total
/// (the contract, SP 0xdaa, rules 4 and 5).
pub const FEE_DAYS_MAX: i32 = 100;

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// What the Store Bank window asks for (0x79b233, 0x79b333, 0x79a0a0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreBankRequest {
    /// Get All: what will it cost?
    GetAll,
    /// Yes to the fee: take it all.
    Confirm,
    /// The window closed.
    Close,
}

impl StoreBankRequest {
    pub fn from_mode(mode: u8) -> Option<Self> {
        match mode {
            0x1a => Some(Self::GetAll),
            0x1b => Some(Self::Confirm),
            0x1c => Some(Self::Close),
            _ => None,
        }
    }
}

/// STORE_BANK modes (0x79b3b6).
pub mod mode {
    /// The window with what the bank holds.
    pub const OPEN: u8 = 0x23;
    /// The fee for Get All: "These were here for %d days, ..." (SP 0xda2),
    /// or "Are you sure you want to retrieve them?" (0xda3) when it is 0.
    pub const FEE: u8 = 0x24;
    /// Fredrick says where the merchant is open, or that he keeps nothing.
    pub const ELSEWHERE: u8 = 0x25;
    /// "This is currently unavailable. Please try again later"
    pub const UNAVAILABLE: u8 = 0x26;
}

/// STORE_BANK_MESSAGE codes (0x79b70e), the answers to a confirm.
pub mod message {
    /// "You have retrieved your items and mesos."
    pub const RETRIEVED: u8 = 0x1e;
    /// More mesos than the player can carry.
    pub const TOO_MUCH_MESO: u8 = 0x1f;
    /// One of the items the player may have only one of.
    pub const ONE_OF_A_KIND: u8 = 0x20;
    /// The player can't pay the fee.
    pub const NO_FEE: u8 = 0x21;
    pub const INVENTORY_FULL: u8 = 0x22;
}

/// The map and channel in a mode 0x25 that tell the client Fredrick keeps
/// nothing (SP 0xd9f rather than 0xda0).
pub const NOTHING_KEPT: (i32, u8) = (999_999_999, 0xff);

/// The whole days from `closed_at` to `now`: none for one closed in the
/// future, or never closed (left in setup).
pub fn days_kept(closed_at: Option<SystemTime>, now: SystemTime) -> i32 {
    closed_at
        .and_then(|closed| now.duration_since(closed).ok())
        .map_or(0, |kept| {
            i32::try_from(kept.as_secs() / DAY.as_secs()).unwrap_or(i32::MAX)
        })
}

/// The storing fee for `days` of keeping mesos and items worth `total`
/// (the mesos plus the items' listed prices): 1 % a day, at most all of
/// it. The client shows the same capped percentage.
pub fn storing_fee(total: i64, days: i32) -> i64 {
    total * i64::from(days.clamp(0, FEE_DAYS_MAX)) / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_days_count_from_the_close() {
        let closed = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let at = |secs| closed + Duration::from_secs(secs);
        assert_eq!(days_kept(Some(closed), at(DAY.as_secs() - 1)), 0);
        assert_eq!(days_kept(Some(closed), at(DAY.as_secs())), 1);
        assert_eq!(days_kept(Some(closed), at(3 * DAY.as_secs() + 5)), 3);
        assert_eq!(days_kept(Some(at(10)), closed), 0, "closed later");
        assert_eq!(days_kept(None, closed), 0, "left in setup");
    }

    #[test]
    fn the_fee_is_a_percent_a_day_up_to_everything() {
        assert_eq!(storing_fee(250_000, 0), 0);
        assert_eq!(storing_fee(250_000, 3), 7_500);
        assert_eq!(storing_fee(99, 1), 0, "rounds down");
        assert_eq!(storing_fee(250_000, 100), 250_000);
        assert_eq!(storing_fee(250_000, 365), 250_000);
    }

    #[test]
    fn requests_by_mode() {
        assert_eq!(
            StoreBankRequest::from_mode(0x1a),
            Some(StoreBankRequest::GetAll)
        );
        assert_eq!(
            StoreBankRequest::from_mode(0x1b),
            Some(StoreBankRequest::Confirm)
        );
        assert_eq!(
            StoreBankRequest::from_mode(0x1c),
            Some(StoreBankRequest::Close)
        );
        assert_eq!(StoreBankRequest::from_mode(0x1d), None);
    }
}
