//! The Cash Shop: an account's NX balances and Cash Inventory, and each
//! character's wish list.

use crate::player::write_player;
use crate::schema::{accounts, cash_items, wish_lists};
use diesel::prelude::*;
use game::Player;
use std::time::SystemTime;

/// What a purchase is paid with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CashType {
    Credit,
    MaplePoints,
    Prepaid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Queryable)]
pub struct Balance {
    pub credit: i32,
    pub points: i32,
    pub prepaid: i32,
}

/// An item in an account's Cash Inventory. `id` is its cash serial number.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, Insertable)]
#[diesel(table_name = cash_items)]
pub struct CashItem {
    pub id: i64,
    pub account_id: i32,
    pub item_id: i32,
    pub sn: i32,
    pub quantity: i16,
    pub gift_from: String,
    pub expires_at: Option<SystemTime>,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = cash_items)]
pub struct NewCashItem {
    pub account_id: i32,
    pub item_id: i32,
    pub sn: i32,
    pub quantity: i16,
    pub expires_at: Option<SystemTime>,
}

pub fn balance(account_id: i32) -> crate::Result<Balance> {
    Ok(accounts::table
        .find(account_id)
        .select((
            accounts::nx_credit,
            accounts::maple_points,
            accounts::nx_prepaid,
        ))
        .first(&mut crate::connection()?)?)
}

pub fn cash_items(account_id: i32) -> crate::Result<Vec<CashItem>> {
    Ok(cash_items::table
        .filter(cash_items::account_id.eq(account_id))
        .order(cash_items::id)
        .load(&mut crate::connection()?)?)
}

/// The account's Cash Inventory item with cash serial `id`.
pub fn cash_item(account_id: i32, id: i64) -> crate::Result<Option<CashItem>> {
    Ok(cash_items::table
        .find(id)
        .filter(cash_items::account_id.eq(account_id))
        .first(&mut crate::connection()?)
        .optional()?)
}

/// Move a Cash Inventory item to a character: delete it and save `player`,
/// who now carries it, in one transaction. False if it was already gone.
pub fn take_out(account_id: i32, id: i64, player: &Player) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<bool> {
        let taken = diesel::delete(
            cash_items::table
                .find(id)
                .filter(cash_items::account_id.eq(account_id)),
        )
        .execute(conn)?;
        if taken == 0 {
            return Ok(false);
        }
        write_player(conn, player)?;
        Ok(true)
    })?)
}

/// Move a character's cash item back to the Cash Inventory: insert it and
/// save `player`, who no longer carries it, in one transaction.
pub fn put_back(item: &CashItem, player: &Player) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<()> {
        diesel::insert_into(cash_items::table)
            .values(item)
            .execute(conn)?;
        write_player(conn, player)
    })?;
    Ok(())
}

/// Pay `price` from the account's `cash_type` balance and put `item` in its
/// Cash Inventory, in one transaction. `None` if the balance is short.
pub fn buy(
    cash_type: CashType,
    price: i32,
    item: &NewCashItem,
) -> crate::Result<Option<(CashItem, Balance)>> {
    let account_id = item.account_id;
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        let account = accounts::table.find(account_id);
        let paid = match cash_type {
            CashType::Credit => diesel::update(account.filter(accounts::nx_credit.ge(price)))
                .set(accounts::nx_credit.eq(accounts::nx_credit - price))
                .execute(conn)?,
            CashType::MaplePoints => {
                diesel::update(account.filter(accounts::maple_points.ge(price)))
                    .set(accounts::maple_points.eq(accounts::maple_points - price))
                    .execute(conn)?
            }
            CashType::Prepaid => diesel::update(account.filter(accounts::nx_prepaid.ge(price)))
                .set(accounts::nx_prepaid.eq(accounts::nx_prepaid - price))
                .execute(conn)?,
        };
        if paid == 0 {
            return Ok(None);
        }
        let bought = diesel::insert_into(cash_items::table)
            .values(item)
            .get_result::<CashItem>(conn)?;
        let balance = account
            .select((
                accounts::nx_credit,
                accounts::maple_points,
                accounts::nx_prepaid,
            ))
            .first::<Balance>(conn)?;
        Ok(Some((bought, balance)))
    })?)
}

/// The character's wish list, in slot order.
pub fn wish_list(character_id: i32) -> crate::Result<Vec<i32>> {
    Ok(wish_lists::table
        .filter(wish_lists::character_id.eq(character_id))
        .order(wish_lists::slot)
        .select(wish_lists::sn)
        .load(&mut crate::connection()?)?)
}

/// Replace the character's wish list.
pub fn set_wish_list(character_id: i32, sns: &[i32]) -> crate::Result<()> {
    let rows: Vec<_> = sns
        .iter()
        .enumerate()
        .map(|(slot, &sn)| {
            (
                wish_lists::character_id.eq(character_id),
                wish_lists::slot.eq(slot as i16),
                wish_lists::sn.eq(sn),
            )
        })
        .collect();
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<()> {
        diesel::delete(wish_lists::table.filter(wish_lists::character_id.eq(character_id)))
            .execute(conn)?;
        if !rows.is_empty() {
            diesel::insert_into(wish_lists::table)
                .values(&rows)
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}
