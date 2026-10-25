//! Votes for the server, which a vote site reports to the world's vote hook
//! (`runtime::vote_hook`): each gives the account NX Credit, once per
//! cooldown.

use crate::cash_shop::{add_cash, Balance, CashType};
use crate::schema::{accounts, votes};
use diesel::prelude::*;
use std::time::{Duration, SystemTime};

/// What a vote came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vote {
    /// The account got the NX, and has these balances.
    Counted(Balance),
    /// No account has that login name.
    NoAccount,
    /// The account's last vote was less than the cooldown ago; it may vote
    /// again at this time.
    TooSoon(SystemTime),
}

/// Count a vote from the account with login name `user` at `now`: `amount`
/// NX Credit, unless it voted within `cooldown`.
pub fn record(user: &str, amount: i32, cooldown: Duration, now: SystemTime) -> crate::Result<Vote> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<Vote> {
        // The lock keeps two reports of one vote from both counting.
        let Some(account_id) = accounts::table
            .filter(accounts::user_name.eq(user))
            .select(accounts::id)
            .for_update()
            .first::<i32>(conn)
            .optional()?
        else {
            return Ok(Vote::NoAccount);
        };
        let last = votes::table
            .filter(votes::account_id.eq(account_id))
            .select(diesel::dsl::max(votes::voted_at))
            .first::<Option<SystemTime>>(conn)?;
        if let Some(next) = last.map(|at| at + cooldown).filter(|&next| next > now) {
            return Ok(Vote::TooSoon(next));
        }
        diesel::insert_into(votes::table)
            .values((
                votes::account_id.eq(account_id),
                votes::amount.eq(amount),
                votes::voted_at.eq(now),
            ))
            .execute(conn)?;
        let balance = add_cash(conn, account_id, CashType::Credit, amount)?
            .expect("the account was just found");
        Ok(Vote::Counted(balance))
    })?)
}
