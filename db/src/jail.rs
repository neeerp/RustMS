//! A GM's `!jail` (LEDGER row 205): until when a character stays in the
//! jail map. The world holds them there while it lasts, even across a
//! logout.

use crate::schema::jails;
use diesel::prelude::*;
use std::time::{Duration, SystemTime};

/// Jail the character for `time` more at `now`, on top of any time left:
/// until when they are now jailed.
pub fn jail(character_id: i32, time: Duration, now: SystemTime) -> crate::Result<SystemTime> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<SystemTime> {
        let left = jails::table
            .find(character_id)
            .select(jails::until)
            .for_update()
            .first::<SystemTime>(conn)
            .optional()?
            .filter(|&until| until > now);
        let until = left.unwrap_or(now) + time;
        diesel::insert_into(jails::table)
            .values((jails::character_id.eq(character_id), jails::until.eq(until)))
            .on_conflict(jails::character_id)
            .do_update()
            .set(jails::until.eq(until))
            .execute(conn)?;
        Ok(until)
    })?)
}

/// Free the character at `now`: whether they were jailed.
pub fn release(character_id: i32, now: SystemTime) -> crate::Result<bool> {
    let freed = diesel::delete(
        jails::table
            .filter(jails::character_id.eq(character_id))
            .filter(jails::until.gt(now)),
    )
    .execute(&mut crate::connection()?)?;
    Ok(freed > 0)
}

/// Until when the character is jailed, if they still are at `now`.
pub fn jailed_until(character_id: i32, now: SystemTime) -> crate::Result<Option<SystemTime>> {
    Ok(jails::table
        .find(character_id)
        .select(jails::until)
        .first::<SystemTime>(&mut crate::connection()?)
        .optional()?
        .filter(|&until| until > now))
}
