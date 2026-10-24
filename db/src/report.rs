//! Players' reports of other players (v83 CLAIM): kept for a GM to read,
//! and counted, since each reporter has only a few a week.

use crate::schema::{characters, reports};
use diesel::dsl::{now, IntervalDsl};
use diesel::prelude::*;

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// A report as the reporter's client sent it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report<'a> {
    pub reporter_id: i32,
    /// Who it is about, by name.
    pub victim: &'a str,
    /// 1 for a conversation, 0 for an illegal program.
    pub kind: i16,
    pub reason: i16,
    pub description: &'a str,
    /// The lines of the conversation; empty for a program.
    pub chat_log: &'a str,
}

/// What became of a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filed {
    /// It was kept; the reporter has `left` more this week.
    Kept { left: i64 },
    /// No other character has that name.
    NotFound,
    /// The reporter made `a_week` in the last seven days already.
    NoneLeft,
}

/// Keep `report` unless its reporter made `a_week` reports in the last
/// seven days or names no other character.
pub fn file(report: &Report, a_week: i64) -> crate::Result<Filed> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> diesel::QueryResult<_> {
        let victim: Option<i32> = characters::table
            .filter(lower(characters::name).eq(lower(report.victim)))
            .select(characters::id)
            .first(conn)
            .optional()?;
        let Some(victim) = victim.filter(|&id| id != report.reporter_id) else {
            return Ok(Filed::NotFound);
        };
        let this_week: i64 = reports::table
            .filter(reports::reporter_id.eq(report.reporter_id))
            .filter(reports::reported_at.gt(now - 7.days()))
            .count()
            .get_result(conn)?;
        if this_week >= a_week {
            return Ok(Filed::NoneLeft);
        }
        diesel::insert_into(reports::table)
            .values((
                reports::reporter_id.eq(report.reporter_id),
                reports::victim_id.eq(victim),
                reports::kind.eq(report.kind),
                reports::reason.eq(report.reason),
                reports::description.eq(report.description),
                reports::chat_log.eq(report.chat_log),
            ))
            .execute(conn)?;
        Ok(Filed::Kept {
            left: a_week - this_week - 1,
        })
    })?)
}
