//! Notes (memos): short messages left for a character, who sees each once,
//! since reading them deletes them. A thank-you for a Cash Shop gift is a
//! note too, flagged `FAME`, which raises its reader's fame.

use crate::schema::{cash_items, characters, gift_replies, notes};
use diesel::prelude::*;
use std::time::SystemTime;

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// A plain note.
pub const PLAIN: i16 = 0;
/// A gift's thank-you, which raises its reader's fame by one.
pub const FAME: i16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Queryable)]
pub struct Note {
    pub id: i32,
    pub sender: String,
    pub message: String,
    pub sent_at: SystemTime,
    pub flag: i16,
}

#[derive(Insertable)]
#[diesel(table_name = notes)]
struct NewNote<'a> {
    receiver_id: i32,
    sender: &'a str,
    message: &'a str,
    flag: i16,
}

/// The notes waiting for `receiver`, oldest first, at most `limit`.
pub fn list(receiver: i32, limit: i64) -> crate::Result<Vec<Note>> {
    Ok(notes::table
        .filter(notes::receiver_id.eq(receiver))
        .order(notes::id)
        .limit(limit)
        .select((
            notes::id,
            notes::sender,
            notes::message,
            notes::sent_at,
            notes::flag,
        ))
        .load(&mut crate::connection()?)?)
}

/// The id and name of the character named `name`, ignoring case.
pub fn character(name: &str) -> crate::Result<Option<(i32, String)>> {
    Ok(characters::table
        .filter(lower(characters::name).eq(lower(name)))
        .select((characters::id, characters::name))
        .first(&mut crate::connection()?)
        .optional()?)
}

/// Leave `receiver` a plain note from `sender`, unless they already have
/// `limit` waiting. Whether it was left.
pub fn send(receiver: i32, sender: &str, message: &str, limit: i64) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        // Two notes sent at once can't both take the last place.
        characters::table
            .find(receiver)
            .select(characters::id)
            .for_update()
            .first::<i32>(conn)?;
        let waiting: i64 = notes::table
            .filter(notes::receiver_id.eq(receiver))
            .count()
            .get_result(conn)?;
        if waiting >= limit {
            return Ok(false);
        }
        diesel::insert_into(notes::table)
            .values(&NewNote {
                receiver_id: receiver,
                sender,
                message,
                flag: PLAIN,
            })
            .execute(conn)?;
        Ok(true)
    })?)
}

/// Thank the sender of the gift with cash serial `gift`, in the Cash
/// Inventory of `account`, with a fame note from `sender`: once per gift,
/// and only to whoever sent it. The receiver's id, if the note was left.
pub fn thank(
    account: i32,
    gift: i64,
    to: &str,
    sender: &str,
    message: &str,
) -> crate::Result<Option<i32>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        let gift_from: Option<String> = cash_items::table
            .find(gift)
            .filter(cash_items::account_id.eq(account))
            .select(cash_items::gift_from)
            .first(conn)
            .optional()?;
        let Some(gift_from) = gift_from.filter(|from| from.eq_ignore_ascii_case(to)) else {
            return Ok(None);
        };
        let receiver: Option<i32> = characters::table
            .filter(lower(characters::name).eq(lower(&gift_from)))
            .select(characters::id)
            .first(conn)
            .optional()?;
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let first = diesel::insert_into(gift_replies::table)
            .values(gift_replies::cash_item_id.eq(gift))
            .on_conflict_do_nothing()
            .execute(conn)?;
        if first == 0 {
            return Ok(None);
        }
        diesel::insert_into(notes::table)
            .values(&NewNote {
                receiver_id: receiver,
                sender,
                message,
                flag: FAME,
            })
            .execute(conn)?;
        Ok(Some(receiver))
    })?)
}

/// Delete `receiver`'s notes among `ids`, returning the flags of those
/// deleted.
pub fn delete(receiver: i32, ids: &[i32]) -> crate::Result<Vec<i16>> {
    Ok(diesel::delete(
        notes::table
            .filter(notes::receiver_id.eq(receiver))
            .filter(notes::id.eq_any(ids)),
    )
    .returning(notes::flag)
    .get_results(&mut crate::connection()?)?)
}
