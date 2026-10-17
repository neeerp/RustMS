//! Fame given: a character may raise or drop someone's fame once a day, and
//! the same character's once a month (OdinMS-family rules).

use crate::schema::{characters, fame_log};
use diesel::dsl::{now, IntervalDsl};
use diesel::prelude::*;

/// Why a character may not give fame now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// There is no such character.
    NotFound,
    /// They gave fame in the last day.
    NotToday,
    /// They gave `target` fame in the last 30 days.
    NotThisMonth,
}

/// If `giver` may give `target` fame now, record that they did.
pub fn give(giver: i32, target: i32) -> crate::Result<Result<(), Refusal>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        let found: i64 = characters::table.find(target).count().get_result(conn)?;
        if found == 0 {
            return Ok(Err(Refusal::NotFound));
        }
        let today: i64 = fame_log::table
            .filter(fame_log::giver_id.eq(giver))
            .filter(fame_log::given_at.gt(now - 1.day()))
            .count()
            .get_result(conn)?;
        if today > 0 {
            return Ok(Err(Refusal::NotToday));
        }
        let this_month: i64 = fame_log::table
            .filter(fame_log::giver_id.eq(giver))
            .filter(fame_log::target_id.eq(target))
            .filter(fame_log::given_at.gt(now - 30.days()))
            .count()
            .get_result(conn)?;
        if this_month > 0 {
            return Ok(Err(Refusal::NotThisMonth));
        }
        diesel::insert_into(fame_log::table)
            .values((fame_log::giver_id.eq(giver), fame_log::target_id.eq(target)))
            .execute(conn)?;
        Ok(Ok(()))
    })?)
}

/// Take back the fame `giver` just gave `target`, which didn't reach them.
pub fn forget(giver: i32, target: i32) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    let latest: Option<i32> = fame_log::table
        .filter(fame_log::giver_id.eq(giver))
        .filter(fame_log::target_id.eq(target))
        .order(fame_log::id.desc())
        .select(fame_log::id)
        .first(&mut conn)
        .optional()?;
    if let Some(id) = latest {
        diesel::delete(fame_log::table.find(id)).execute(&mut conn)?;
    }
    Ok(())
}
