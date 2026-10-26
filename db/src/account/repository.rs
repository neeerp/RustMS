use super::{Account, NewAccount};
use crate::schema;
use diesel::expression_methods::*;
use diesel::{QueryDsl, RunQueryDsl, SaveChangesDsl};
use schema::accounts;
use schema::accounts::dsl::*;

pub fn get_account(user: &str) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    Ok(accounts
        .filter(user_name.eq(user))
        .first::<Account>(&mut connection)?)
}

pub fn get_account_by_id(a_id: i32) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    Ok(accounts
        .filter(id.eq(a_id))
        .first::<Account>(&mut connection)?)
}

pub fn create_account<'a>(user: &'a str, pw: &'a str) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    let new_account = NewAccount {
        user_name: user,
        password: pw,
    };

    Ok(diesel::insert_into(accounts::table)
        .values(&new_account)
        .get_result::<Account>(&mut connection)?)
}

pub fn update_account(acc: &Account) -> crate::Result<Account> {
    let mut connection = crate::connection()?;
    Ok(acc.save_changes(&mut connection)?)
}

/// Block the account until `until`, or for good: login refuses it, giving
/// `reason` (Etc/BlockReason.img's number, 0 for none) and the end. `note`
/// is the GM's, for the record.
pub fn block(
    account_id: i32,
    reason: i16,
    until: Option<std::time::SystemTime>,
    note: Option<&str>,
) -> crate::Result<()> {
    let mut connection = crate::connection()?;
    diesel::update(accounts.filter(id.eq(account_id)))
        .set((
            banned.eq(true),
            ban_reason.eq(reason),
            ban_until.eq(until),
            ban_msg.eq(note),
        ))
        .execute(&mut connection)?;
    Ok(())
}

/// Lift the account's block. False if it had none.
pub fn unblock(account_id: i32) -> crate::Result<bool> {
    let mut connection = crate::connection()?;
    let lifted = diesel::update(accounts.filter(id.eq(account_id)).filter(banned.eq(true)))
        .set((
            banned.eq(false),
            ban_reason.eq(0),
            ban_until.eq(None::<std::time::SystemTime>),
            ban_msg.eq(None::<String>),
        ))
        .execute(&mut connection)?;
    Ok(lifted > 0)
}

/// Block the account from chatting until `until`, giving `reason`
/// (Etc/ChatBlockReason.img's number, 1..=6).
pub fn chat_block(account_id: i32, reason: i16, until: std::time::SystemTime) -> crate::Result<()> {
    let mut connection = crate::connection()?;
    diesel::update(accounts.filter(id.eq(account_id)))
        .set((chat_block_reason.eq(reason), chat_block_until.eq(until)))
        .execute(&mut connection)?;
    Ok(())
}

/// Lift the account's chat block at `now`. False if it had none.
pub fn chat_unblock(account_id: i32, now: std::time::SystemTime) -> crate::Result<bool> {
    let mut connection = crate::connection()?;
    let lifted = diesel::update(
        accounts
            .filter(id.eq(account_id))
            .filter(chat_block_reason.gt(0))
            .filter(chat_block_until.gt(now)),
    )
    .set((
        chat_block_reason.eq(0),
        chat_block_until.eq(None::<std::time::SystemTime>),
    ))
    .execute(&mut connection)?;
    Ok(lifted > 0)
}

/// The ids of the account's characters, in every world.
pub fn character_ids(account_id: i32) -> crate::Result<Vec<i32>> {
    use schema::characters;
    let mut connection = crate::connection()?;
    Ok(characters::table
        .filter(characters::accountid.eq(account_id))
        .select(characters::id)
        .load(&mut connection)?)
}
