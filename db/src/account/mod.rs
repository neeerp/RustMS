use crate::schema::accounts;
use std::{fmt::Debug, time::SystemTime};

mod repository;
pub use repository::*;

#[derive(Identifiable, Queryable, AsChangeset)]
pub struct Account {
    pub id: i32,
    pub user_name: String,
    pub password: String,
    pub pin: String,
    pub pic: String,
    pub logged_in: bool,
    pub last_login_at: Option<SystemTime>,
    pub created_at: SystemTime,
    pub character_slots: i16,
    pub gender: i16,
    pub accepted_tos: bool,
    /// Whether a GM blocked the account (`crate::account::block`), which
    /// only that and an unblock change.
    #[diesel(skip_update)]
    pub banned: bool,
    #[diesel(skip_update)]
    pub ban_msg: Option<String>,
    pub gm_level: i16,
    // Only purchases change the balances (`crate::cash_shop`), so saving a
    // stale account can't undo one.
    #[diesel(skip_update)]
    pub nx_credit: i32,
    #[diesel(skip_update)]
    pub maple_points: i32,
    #[diesel(skip_update)]
    pub nx_prepaid: i32,
    /// Slots in the account's storage; only the Cash Shop changes them.
    #[diesel(skip_update)]
    pub storage_slots: i16,
    /// Mesos in the account's storage; only `crate::storage` changes them.
    #[diesel(skip_update)]
    pub storage_meso: i32,
    /// Whether the client treats the account as v83's admin account (set by
    /// SQL only; `gm_level` never implies it).
    #[diesel(skip_update)]
    pub test_admin: bool,
    /// Why the account is blocked, as Etc/BlockReason.img's number; 0 is
    /// none given.
    #[diesel(skip_update)]
    pub ban_reason: i16,
    /// When the block ends; none is never.
    #[diesel(skip_update)]
    pub ban_until: Option<SystemTime>,
}

impl Account {
    /// Whether the account is blocked at `now`; a block that has run out
    /// no longer counts.
    pub fn blocked_at(&self, now: SystemTime) -> bool {
        self.banned && self.ban_until.map_or(true, |until| until > now)
    }
}

impl Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id: {}, user_name: {}, logged_in: {}",
            self.id, self.user_name, self.logged_in
        )
    }
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub struct NewAccount<'a> {
    pub user_name: &'a str,
    pub password: &'a str,
}
