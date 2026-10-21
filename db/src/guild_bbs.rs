//! The guild bulletin board. A guild numbers its threads from 1, and its
//! notice is the thread numbered 0. Who may do what is read from the
//! member's row in the same transaction: the poster or a rank of Jr. Master
//! or better may edit or delete a post, and only those ranks may post the
//! notice.

use crate::schema::{guild_bbs_replies, guild_bbs_threads, guild_members, guilds};
use diesel::dsl::{count_star, max, now};
use diesel::prelude::*;
use game::guild::JR_MASTER_RANK;
use game::guild_bbs::{NOTICE, PAGE_SIZE};
use std::time::SystemTime;

/// A thread as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The guild's number for the thread.
    pub id: i32,
    pub poster: i32,
    pub title: String,
    pub posted_at: SystemTime,
    pub icon: i16,
    pub replies: i32,
}

/// One page of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub notice: Option<Summary>,
    /// Every thread but the notice.
    pub total: i32,
    /// This page's threads, newest first.
    pub threads: Vec<Summary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Queryable)]
pub struct Reply {
    /// The board's number for the reply, unique across guilds.
    pub id: i32,
    pub poster: i32,
    pub text: String,
    pub posted_at: SystemTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    pub id: i32,
    pub poster: i32,
    pub title: String,
    pub body: String,
    pub posted_at: SystemTime,
    pub icon: i16,
    /// Oldest first.
    pub replies: Vec<Reply>,
}

const THREAD_COLUMNS: (
    guild_bbs_threads::id,
    guild_bbs_threads::local_id,
    guild_bbs_threads::poster_id,
    guild_bbs_threads::title,
    guild_bbs_threads::body,
    guild_bbs_threads::icon,
    guild_bbs_threads::posted_at,
) = (
    guild_bbs_threads::id,
    guild_bbs_threads::local_id,
    guild_bbs_threads::poster_id,
    guild_bbs_threads::title,
    guild_bbs_threads::body,
    guild_bbs_threads::icon,
    guild_bbs_threads::posted_at,
);

#[derive(Queryable)]
struct ThreadRow {
    key: i32,
    id: i32,
    poster: i32,
    title: String,
    body: String,
    icon: i16,
    posted_at: SystemTime,
}

/// The guild and rank of `character`, unless they are in none.
fn membership(conn: &mut PgConnection, character: i32) -> QueryResult<Option<(i32, i16)>> {
    guild_members::table
        .find(character)
        .select((guild_members::guild_id, guild_members::rank))
        .first(conn)
        .optional()
}

fn may_moderate(rank: i16) -> bool {
    rank <= i16::from(JR_MASTER_RANK)
}

fn thread_row(conn: &mut PgConnection, guild: i32, id: i32) -> QueryResult<Option<ThreadRow>> {
    guild_bbs_threads::table
        .filter(guild_bbs_threads::guild_id.eq(guild))
        .filter(guild_bbs_threads::local_id.eq(id))
        .select(THREAD_COLUMNS)
        .first(conn)
        .optional()
}

/// Page `page` (from 0) of the board of `by`'s guild, or the first page if
/// it is past the end. `None` if `by` is in no guild.
pub fn page(by: i32, page: i32) -> crate::Result<Option<Page>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, _)) = membership(conn, by)? else {
            return Ok(None);
        };
        let total: i64 = guild_bbs_threads::table
            .filter(guild_bbs_threads::guild_id.eq(guild))
            .filter(guild_bbs_threads::local_id.ne(NOTICE))
            .count()
            .get_result(conn)?;
        let mut start = i64::from(page.max(0)) * PAGE_SIZE;
        if start >= total {
            start = 0;
        }
        let rows: Vec<(i32, i32, i32, String, SystemTime, i16)> = guild_bbs_threads::table
            .filter(guild_bbs_threads::guild_id.eq(guild))
            .order(guild_bbs_threads::local_id.desc())
            .select((
                guild_bbs_threads::id,
                guild_bbs_threads::local_id,
                guild_bbs_threads::poster_id,
                guild_bbs_threads::title,
                guild_bbs_threads::posted_at,
                guild_bbs_threads::icon,
            ))
            .load(conn)?;
        // The notice sorts last; the page is cut from the rest.
        let (notice, threads): (Vec<_>, Vec<_>) = rows.into_iter().partition(|row| row.1 == NOTICE);
        let threads: Vec<_> = threads
            .into_iter()
            .skip(start as usize)
            .take(PAGE_SIZE as usize)
            .collect();
        let keys: Vec<i32> = notice.iter().chain(&threads).map(|row| row.0).collect();
        let counts: Vec<(i32, i64)> = guild_bbs_replies::table
            .filter(guild_bbs_replies::thread_id.eq_any(&keys))
            .group_by(guild_bbs_replies::thread_id)
            .select((guild_bbs_replies::thread_id, count_star()))
            .load(conn)?;
        let summary = |(key, id, poster, title, posted_at, icon): (
            i32,
            i32,
            i32,
            String,
            SystemTime,
            i16,
        )| {
            let replies = counts
                .iter()
                .find(|(thread, _)| *thread == key)
                .map_or(0, |(_, count)| *count as i32);
            Summary {
                id,
                poster,
                title,
                posted_at,
                icon,
                replies,
            }
        };
        Ok(Some(Page {
            notice: notice.into_iter().next().map(summary),
            total: total as i32,
            threads: threads.into_iter().map(summary).collect(),
        }))
    })?)
}

/// Thread `id` of `by`'s guild with its replies, unless there is none.
pub fn thread(by: i32, id: i32) -> crate::Result<Option<Thread>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, _)) = membership(conn, by)? else {
            return Ok(None);
        };
        load_thread(conn, guild, id)
    })?)
}

fn load_thread(conn: &mut PgConnection, guild: i32, id: i32) -> QueryResult<Option<Thread>> {
    let Some(row) = thread_row(conn, guild, id)? else {
        return Ok(None);
    };
    let replies = guild_bbs_replies::table
        .filter(guild_bbs_replies::thread_id.eq(row.key))
        .order(guild_bbs_replies::id)
        .select((
            guild_bbs_replies::id,
            guild_bbs_replies::poster_id,
            guild_bbs_replies::text,
            guild_bbs_replies::posted_at,
        ))
        .load(conn)?;
    Ok(Some(Thread {
        id: row.id,
        poster: row.poster,
        title: row.title,
        body: row.body,
        posted_at: row.posted_at,
        icon: row.icon,
        replies,
    }))
}

/// Post a thread on the board of `by`'s guild, or its notice. Returns the
/// thread, or `None` if `by` is in no guild or may not post the notice, or
/// the guild has one already.
pub fn post(
    by: i32,
    title: &str,
    body: &str,
    icon: i16,
    notice: bool,
) -> crate::Result<Option<Thread>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, _)) = membership(conn, by)? else {
            return Ok(None);
        };
        // Two posts at once can't take the same number. The guild may be
        // gone meanwhile, and the rank is read again under the lock.
        let locked: Option<i32> = guilds::table
            .find(guild)
            .select(guilds::id)
            .for_update()
            .first(conn)
            .optional()?;
        if locked.is_none() {
            return Ok(None);
        }
        let Some((guild, rank)) = membership(conn, by)?.filter(|(current, _)| *current == guild)
        else {
            return Ok(None);
        };
        let id = if notice {
            if !may_moderate(rank) || thread_row(conn, guild, NOTICE)?.is_some() {
                return Ok(None);
            }
            NOTICE
        } else {
            let last: Option<i32> = guild_bbs_threads::table
                .filter(guild_bbs_threads::guild_id.eq(guild))
                .select(max(guild_bbs_threads::local_id))
                .first(conn)?;
            last.unwrap_or(NOTICE) + 1
        };
        diesel::insert_into(guild_bbs_threads::table)
            .values((
                guild_bbs_threads::guild_id.eq(guild),
                guild_bbs_threads::local_id.eq(id),
                guild_bbs_threads::poster_id.eq(by),
                guild_bbs_threads::title.eq(title),
                guild_bbs_threads::body.eq(body),
                guild_bbs_threads::icon.eq(icon),
            ))
            .execute(conn)?;
        load_thread(conn, guild, id)
    })?)
}

/// The thread `id` of `by`'s guild, locked, if `by` posted it or may
/// moderate.
fn own_thread(conn: &mut PgConnection, by: i32, id: i32) -> QueryResult<Option<(i32, ThreadRow)>> {
    let Some((guild, rank)) = membership(conn, by)? else {
        return Ok(None);
    };
    let row: Option<ThreadRow> = guild_bbs_threads::table
        .filter(guild_bbs_threads::guild_id.eq(guild))
        .filter(guild_bbs_threads::local_id.eq(id))
        .select(THREAD_COLUMNS)
        .for_update()
        .first(conn)
        .optional()?;
    Ok(row
        .filter(|row| row.poster == by || may_moderate(rank))
        .map(|row| (guild, row)))
}

/// Rewrite thread `id`, dated now, if `by` may. Returns the thread, or
/// `None` if there is none or `by` may not.
pub fn edit(by: i32, id: i32, title: &str, body: &str, icon: i16) -> crate::Result<Option<Thread>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, row)) = own_thread(conn, by, id)? else {
            return Ok(None);
        };
        diesel::update(guild_bbs_threads::table.find(row.key))
            .set((
                guild_bbs_threads::title.eq(title),
                guild_bbs_threads::body.eq(body),
                guild_bbs_threads::icon.eq(icon),
                guild_bbs_threads::posted_at.eq(now),
            ))
            .execute(conn)?;
        load_thread(conn, guild, id)
    })?)
}

/// Delete thread `id` and its replies, if `by` may. Whether it was deleted.
pub fn delete(by: i32, id: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((_, row)) = own_thread(conn, by, id)? else {
            return Ok(false);
        };
        diesel::delete(guild_bbs_threads::table.find(row.key)).execute(conn)?;
        Ok(true)
    })?)
}

/// Reply to thread `id` of `by`'s guild. Returns the thread, or `None` if
/// there is none.
pub fn reply(by: i32, id: i32, text: &str) -> crate::Result<Option<Thread>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, _)) = membership(conn, by)? else {
            return Ok(None);
        };
        // Hold the thread so a delete can't remove it under the reply.
        let row: Option<ThreadRow> = guild_bbs_threads::table
            .filter(guild_bbs_threads::guild_id.eq(guild))
            .filter(guild_bbs_threads::local_id.eq(id))
            .select(THREAD_COLUMNS)
            .for_key_share()
            .first(conn)
            .optional()?;
        let Some(row) = row else {
            return Ok(None);
        };
        diesel::insert_into(guild_bbs_replies::table)
            .values((
                guild_bbs_replies::thread_id.eq(row.key),
                guild_bbs_replies::poster_id.eq(by),
                guild_bbs_replies::text.eq(text),
            ))
            .execute(conn)?;
        load_thread(conn, guild, id)
    })?)
}

/// Delete reply `reply` from a thread of `by`'s guild, if `by` wrote it or
/// may moderate. Returns its thread, or `None` if nothing was deleted.
pub fn delete_reply(by: i32, reply: i32) -> crate::Result<Option<Thread>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some((guild, rank)) = membership(conn, by)? else {
            return Ok(None);
        };
        // A reply of another guild's board is not found.
        let found: Option<(i32, i32)> = guild_bbs_replies::table
            .inner_join(guild_bbs_threads::table)
            .filter(guild_bbs_replies::id.eq(reply))
            .filter(guild_bbs_threads::guild_id.eq(guild))
            .select((guild_bbs_replies::poster_id, guild_bbs_threads::local_id))
            .first(conn)
            .optional()?;
        let Some((_, id)) = found.filter(|(poster, _)| *poster == by || may_moderate(rank)) else {
            return Ok(None);
        };
        diesel::delete(guild_bbs_replies::table.find(reply)).execute(conn)?;
        load_thread(conn, guild, id)
    })?)
}
