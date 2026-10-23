//! Name Change (5400000, report c060): a character's request, bought in the
//! Cash Shop with NX Prepaid, to take a new name. The purchase leaves a
//! receipt in the Cash Inventory; the change is made when the character
//! next logs out, or when the world starts for one offline then, and the
//! receipt goes with it. Notes and owner tags keep the old name.

use crate::cash_shop::{balance_of, pay, record_sale, Balance, CashItem, CashType, NewCashItem};
use crate::schema::{
    accounts, cash_items, characters, inventory_items, name_changes, storage_items,
};
use diesel::dsl::exists;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::select;
use std::time::{Duration, SystemTime};

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// How long after a change before the character may ask for another (the
/// rules sheet's "once per month").
pub const COOLDOWN: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Whether a character may ask for a new name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Free,
    /// It has a request waiting.
    Pending,
    /// It changed its name less than [`COOLDOWN`] ago.
    Cooldown,
}

/// What a purchase came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requested {
    /// The receipt now in the Cash Inventory, and the balances.
    Done(CashItem, Balance),
    /// The NX Prepaid is short.
    Short,
    /// The character may not ask now, or the name is someone's.
    Refused,
}

/// A purchase: `receipt` is what commodity `sn` puts in the Cash Inventory.
pub struct NewRequest<'a> {
    pub account_id: i32,
    pub character_id: i32,
    pub sn: i32,
    pub price: i32,
    pub receipt: NewCashItem,
    pub old_name: &'a str,
    pub new_name: &'a str,
}

/// What became of a request when it was due.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    Renamed {
        old_name: String,
        new_name: String,
    },
    /// Another character took the name first; the NX Prepaid went back.
    Refunded {
        new_name: String,
    },
}

#[derive(Queryable)]
struct Open {
    id: i32,
    character_id: i32,
    cash_id: i64,
    price: i32,
    old_name: String,
    new_name: String,
}

pub fn standing(character_id: i32, now: SystemTime) -> crate::Result<Standing> {
    Ok(standing_of(&mut *crate::connection()?, character_id, now)?)
}

/// Whether `name`, ignoring case, is a character's or waits in a request.
pub fn claimed(name: &str) -> crate::Result<bool> {
    Ok(claimed_by(&mut *crate::connection()?, name)?)
}

/// Pay for a request from the account's NX Prepaid and record it, with its
/// receipt, in one transaction, re-checking the character and the name.
pub fn request(new: &NewRequest, now: SystemTime) -> crate::Result<Requested> {
    let mut conn = crate::connection()?;
    let result = conn.transaction(|conn| -> diesel::QueryResult<Requested> {
        if standing_of(conn, new.character_id, now)? != Standing::Free
            || claimed_by(conn, new.new_name)?
        {
            return Ok(Requested::Refused);
        }
        if !pay(conn, new.account_id, CashType::Prepaid, new.price)? {
            return Ok(Requested::Short);
        }
        let receipt = diesel::insert_into(cash_items::table)
            .values(&new.receipt)
            .get_result::<CashItem>(conn)?;
        record_sale(conn, new.sn)?;
        diesel::insert_into(name_changes::table)
            .values((
                name_changes::character_id.eq(new.character_id),
                name_changes::cash_id.eq(receipt.id),
                name_changes::price.eq(new.price),
                name_changes::old_name.eq(new.old_name),
                name_changes::new_name.eq(new.new_name),
                name_changes::requested_at.eq(now),
            ))
            .execute(conn)?;
        Ok(Requested::Done(receipt, balance_of(conn, new.account_id)?))
    });
    match result {
        // Another request for the character or the name got in first.
        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            Ok(Requested::Refused)
        }
        result => Ok(result?),
    }
}

/// Drop the character's waiting request whose receipt is `cash_id`, and the
/// receipt, with no refund; whether there was one. The caller takes the
/// receipt from the bag it holds.
pub fn cancel(character_id: i32, cash_id: i64) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<bool> {
        let cancelled = diesel::delete(
            name_changes::table
                .filter(name_changes::character_id.eq(character_id))
                .filter(name_changes::cash_id.eq(cash_id))
                .filter(name_changes::completed_at.is_null()),
        )
        .execute(conn)?;
        if cancelled > 0 {
            delete_receipt(conn, cash_id)?;
        }
        Ok(cancelled > 0)
    })?)
}

/// Before `account_id`'s character is deleted: refund its waiting request
/// and drop the receipt (the request row goes with the character).
pub(crate) fn withdraw(
    conn: &mut PgConnection,
    character_id: i32,
    account_id: i32,
) -> diesel::QueryResult<()> {
    let open: Vec<(i64, i32)> = name_changes::table
        .inner_join(characters::table)
        .filter(name_changes::character_id.eq(character_id))
        .filter(characters::accountid.eq(account_id))
        .filter(name_changes::completed_at.is_null())
        .select((name_changes::cash_id, name_changes::price))
        .load(conn)?;
    for (cash_id, price) in open {
        delete_receipt(conn, cash_id)?;
        diesel::update(accounts::table.find(account_id))
            .set(accounts::nx_prepaid.eq(accounts::nx_prepaid + price))
            .execute(conn)?;
    }
    Ok(())
}

/// Make the character's waiting change, if it has one. Call it only while
/// the character is offline and saved.
pub fn apply(character_id: i32, now: SystemTime) -> crate::Result<Option<Applied>> {
    let mut conn = crate::connection()?;
    let open = open_requests(&mut conn, Some(character_id))?.pop();
    Ok(match open {
        Some(open) => conn.transaction(|conn| apply_one(conn, &open, now))?,
        None => None,
    })
}

/// Make every waiting change, for a world starting with no one online: the
/// character ids and what became of each, or the error that left a request
/// waiting for the character's next logout.
pub fn apply_all(now: SystemTime) -> crate::Result<Vec<(i32, crate::Result<Applied>)>> {
    let mut conn = crate::connection()?;
    let open = open_requests(&mut conn, None)?;
    let mut applied = Vec::with_capacity(open.len());
    for open in open {
        match conn.transaction(|conn| apply_one(conn, &open, now)) {
            Ok(Some(done)) => applied.push((open.character_id, Ok(done))),
            Ok(None) => {}
            Err(error) => applied.push((open.character_id, Err(error.into()))),
        }
    }
    Ok(applied)
}

/// The waiting requests, oldest first, or only `character_id`'s.
fn open_requests(
    conn: &mut PgConnection,
    character_id: Option<i32>,
) -> diesel::QueryResult<Vec<Open>> {
    let mut query = name_changes::table
        .filter(name_changes::completed_at.is_null())
        .select((
            name_changes::id,
            name_changes::character_id,
            name_changes::cash_id,
            name_changes::price,
            name_changes::old_name,
            name_changes::new_name,
        ))
        .order(name_changes::id)
        .into_boxed();
    if let Some(character_id) = character_id {
        query = query.filter(name_changes::character_id.eq(character_id));
    }
    query.load(conn)
}

/// Rename the character, unless another took the name meanwhile, which
/// refunds the request; either way the receipt goes, wherever it is. `None`
/// when the request was no longer waiting.
fn apply_one(
    conn: &mut PgConnection,
    open: &Open,
    now: SystemTime,
) -> diesel::QueryResult<Option<Applied>> {
    let taken = select(exists(
        characters::table
            .filter(lower(characters::name).eq(lower(&open.new_name)))
            .filter(characters::id.ne(open.character_id)),
    ))
    .get_result::<bool>(conn)?;
    let closed = diesel::update(
        name_changes::table
            .find(open.id)
            .filter(name_changes::completed_at.is_null()),
    )
    .set((
        name_changes::completed_at.eq(now),
        name_changes::failed.eq(taken),
    ))
    .execute(conn)?;
    if closed == 0 {
        return Ok(None);
    }
    delete_receipt(conn, open.cash_id)?;
    if taken {
        let account_id = characters::table
            .find(open.character_id)
            .select(characters::accountid)
            .first::<i32>(conn)?;
        diesel::update(accounts::table.find(account_id))
            .set(accounts::nx_prepaid.eq(accounts::nx_prepaid + open.price))
            .execute(conn)?;
        return Ok(Some(Applied::Refunded {
            new_name: open.new_name.clone(),
        }));
    }
    diesel::update(characters::table.find(open.character_id))
        .set(characters::name.eq(&open.new_name))
        .execute(conn)?;
    // Gifts not yet thanked for find their sender by name.
    diesel::update(cash_items::table.filter(cash_items::gift_from.eq(&open.old_name)))
        .set(cash_items::gift_from.eq(&open.new_name))
        .execute(conn)?;
    Ok(Some(Applied::Renamed {
        old_name: open.old_name.clone(),
        new_name: open.new_name.clone(),
    }))
}

/// Delete a request's receipt from the Cash Inventory, a bag or the storage.
fn delete_receipt(conn: &mut PgConnection, cash_id: i64) -> diesel::QueryResult<()> {
    diesel::delete(cash_items::table.find(cash_id)).execute(conn)?;
    diesel::delete(inventory_items::table.filter(inventory_items::cash_serial.eq(cash_id)))
        .execute(conn)?;
    diesel::delete(storage_items::table.filter(storage_items::cash_serial.eq(cash_id)))
        .execute(conn)?;
    Ok(())
}

fn standing_of(
    conn: &mut PgConnection,
    character_id: i32,
    now: SystemTime,
) -> diesel::QueryResult<Standing> {
    // A refunded request leaves no cooldown.
    let done: Vec<Option<SystemTime>> = name_changes::table
        .filter(name_changes::character_id.eq(character_id))
        .filter(name_changes::failed.eq(false))
        .select(name_changes::completed_at)
        .load(conn)?;
    Ok(if done.iter().any(Option::is_none) {
        Standing::Pending
    } else if done.iter().flatten().any(|at| now < *at + COOLDOWN) {
        Standing::Cooldown
    } else {
        Standing::Free
    })
}

fn claimed_by(conn: &mut PgConnection, name: &str) -> diesel::QueryResult<bool> {
    let character = select(exists(
        characters::table.filter(lower(characters::name).eq(lower(name))),
    ))
    .get_result::<bool>(conn)?;
    let requested = select(exists(
        name_changes::table
            .filter(name_changes::completed_at.is_null())
            .filter(lower(name_changes::new_name).eq(lower(name))),
    ))
    .get_result::<bool>(conn)?;
    Ok(character || requested)
}
