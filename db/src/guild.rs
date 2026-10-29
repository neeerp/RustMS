//! Guilds and their members. A character is in at most one guild (the
//! member row's key); rank 1 is the master.

use crate::alliance::GuildGone;
use crate::schema::{characters, guild_members, guilds};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use game::guild::{
    Emblem, Guild, GuildMember, GuildTag, Standing, CAPACITY_STEP, JR_MASTER_RANK, MASTER_RANK,
    RANKING_SIZE,
};

/// Why founding a guild failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateError {
    /// Another guild in the world has the name, ignoring case.
    NameTaken,
    AlreadyInGuild,
}

/// Why joining a guild failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinError {
    /// The guild has as many members as it may.
    Full,
    AlreadyInGuild,
    /// The guild was disbanded since the invitation.
    NoGuild,
}

#[derive(Queryable)]
struct GuildRow {
    id: i32,
    _world_id: i16,
    name: String,
    title1: String,
    title2: String,
    title3: String,
    title4: String,
    title5: String,
    capacity: i16,
    mark_bg: i16,
    mark_bg_color: i16,
    mark: i16,
    mark_color: i16,
    notice: String,
    points: i32,
    alliance_id: i32,
}

fn emblem(background: i16, background_color: i16, mark: i16, mark_color: i16) -> Emblem {
    Emblem {
        background: background as u16,
        background_color: background_color as u8,
        mark: mark as u16,
        mark_color: mark_color as u8,
    }
}

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// Whether a guild in `world_id` has the name `name`, ignoring case.
pub fn name_taken(world_id: u8, name: &str) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    let found = guilds::table
        .filter(guilds::world_id.eq(i16::from(world_id)))
        .filter(lower(guilds::name).eq(lower(name)))
        .select(guilds::id)
        .first::<i32>(&mut conn)
        .optional()?;
    Ok(found.is_some())
}

/// Found a guild named `name` in `world_id` with `master` and the party
/// members who agreed to it, `cofounders`, made jr. masters. Returns the
/// new guild's id.
pub fn create(
    world_id: u8,
    name: &str,
    master: i32,
    cofounders: &[i32],
) -> crate::Result<Result<i32, CreateError>> {
    let mut conn = crate::connection()?;
    let created = conn.transaction(|conn| -> QueryResult<_> {
        let ids: Vec<i32> = std::iter::once(master)
            .chain(cofounders.iter().copied())
            .collect();
        let members = guild_members::table
            .filter(guild_members::character_id.eq_any(&ids))
            .count()
            .get_result::<i64>(conn)?;
        if members > 0 {
            return Ok(Err(CreateError::AlreadyInGuild));
        }
        let id = diesel::insert_into(guilds::table)
            .values((
                guilds::world_id.eq(i16::from(world_id)),
                guilds::name.eq(name),
            ))
            .returning(guilds::id)
            .get_result::<i32>(conn)?;
        diesel::insert_into(guild_members::table)
            .values((
                guild_members::character_id.eq(master),
                guild_members::guild_id.eq(id),
                guild_members::rank.eq(i16::from(MASTER_RANK)),
            ))
            .execute(conn)?;
        for &cofounder in cofounders {
            diesel::insert_into(guild_members::table)
                .values((
                    guild_members::character_id.eq(cofounder),
                    guild_members::guild_id.eq(id),
                    guild_members::rank.eq(i16::from(JR_MASTER_RANK)),
                ))
                .execute(conn)?;
        }
        Ok(Ok(id))
    });
    match created {
        // The unique index on the world and lowercased name.
        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            Ok(Err(CreateError::NameTaken))
        }
        other => Ok(other?),
    }
}

/// Add `cid` to the guild at its lowest rank with a title, if there is
/// room. Returns that rank.
pub fn add_member(guild_id: i32, cid: i32) -> crate::Result<Result<u8, JoinError>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        // Lock the guild so members joining at once can't pass its capacity.
        let Some(guild) = locked(conn, guild_id)? else {
            return Ok(Err(JoinError::NoGuild));
        };
        let member = guild_members::table
            .find(cid)
            .select(guild_members::guild_id)
            .first::<i32>(conn)
            .optional()?;
        if member.is_some() {
            return Ok(Err(JoinError::AlreadyInGuild));
        }
        if guild.members.len() >= usize::from(guild.capacity) {
            return Ok(Err(JoinError::Full));
        }
        let rank = guild.lowest_rank();
        diesel::insert_into(guild_members::table)
            .values((
                guild_members::character_id.eq(cid),
                guild_members::guild_id.eq(guild_id),
                guild_members::rank.eq(i16::from(rank)),
            ))
            .execute(conn)?;
        Ok(Ok(rank))
    })?)
}

/// Take `cid` out of the guild unless they are its master. Returns whether
/// they were a member it could lose.
pub fn remove_member(guild_id: i32, cid: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    let removed = diesel::delete(
        guild_members::table
            .filter(guild_members::character_id.eq(cid))
            .filter(guild_members::guild_id.eq(guild_id))
            .filter(guild_members::rank.ne(i16::from(MASTER_RANK))),
    )
    .execute(&mut conn)?;
    Ok(removed == 1)
}

/// Move `member_id` to `rank`, if member `by` may. Returns every member's
/// id, or `None` if `by` may not.
pub fn set_rank(
    guild_id: i32,
    by: i32,
    member_id: i32,
    rank: u8,
) -> crate::Result<Option<Vec<i32>>> {
    change(
        guild_id,
        |guild| guild.may_change_rank(by, member_id, rank),
        |conn| {
            diesel::update(guild_members::table.find(member_id))
                .set(guild_members::rank.eq(i16::from(rank)))
                .execute(conn)
        },
    )
}

/// Give the ranks these titles, if member `by` may; `forbids` is the
/// forbidden-word check. Returns every member's id, or `None` if `by` may
/// not.
pub fn set_titles(
    guild_id: i32,
    by: i32,
    titles: &[String; 5],
    forbids: impl Fn(&str) -> bool,
) -> crate::Result<Option<Vec<i32>>> {
    change(
        guild_id,
        |guild| guild.may_set_titles(by, titles, forbids),
        |conn| {
            diesel::update(guilds::table.find(guild_id))
                .set((
                    guilds::title1.eq(&titles[0]),
                    guilds::title2.eq(&titles[1]),
                    guilds::title3.eq(&titles[2]),
                    guilds::title4.eq(&titles[3]),
                    guilds::title5.eq(&titles[4]),
                ))
                .execute(conn)
        },
    )
}

/// Post the guild's notice, if member `by` may. Returns every member's id,
/// or `None` if `by` may not.
pub fn set_notice(guild_id: i32, by: i32, notice: &str) -> crate::Result<Option<Vec<i32>>> {
    change(
        guild_id,
        |guild| guild.may_set_notice(by, notice),
        |conn| {
            diesel::update(guilds::table.find(guild_id))
                .set(guilds::notice.eq(notice))
                .execute(conn)
        },
    )
}

/// Add `amount` to the guild's points, never below none. Returns the new
/// total and every member's id, or `None` if the guild was disbanded.
pub fn add_points(guild_id: i32, amount: i32) -> crate::Result<Option<(i32, Vec<i32>)>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(guild) = locked(conn, guild_id)? else {
            return Ok(None);
        };
        let points = guild.points.saturating_add(amount).max(0);
        diesel::update(guilds::table.find(guild_id))
            .set(guilds::points.eq(points))
            .execute(conn)?;
        Ok(Some((
            points,
            guild.members.iter().map(|member| member.id).collect(),
        )))
    })?)
}

/// Give the guild this emblem, if member `by` may. Returns every member's
/// id, or `None` if `by` may not.
pub fn set_emblem(guild_id: i32, by: i32, emblem: Emblem) -> crate::Result<Option<Vec<i32>>> {
    change(
        guild_id,
        |guild| guild.may_set_emblem(by),
        |conn| {
            diesel::update(guilds::table.find(guild_id))
                .set((
                    guilds::mark_bg.eq(emblem.background as i16),
                    guilds::mark_bg_color.eq(i16::from(emblem.background_color)),
                    guilds::mark.eq(emblem.mark as i16),
                    guilds::mark_color.eq(i16::from(emblem.mark_color)),
                ))
                .execute(conn)
        },
    )
}

/// Grow the guild a step from `from` members, if member `by` may. Returns
/// every member's id, or `None` if `by` may not.
pub fn grow(guild_id: i32, by: i32, from: u8) -> crate::Result<Option<Vec<i32>>> {
    change(
        guild_id,
        |guild| guild.may_grow(by, from),
        |conn| {
            diesel::update(guilds::table.find(guild_id))
                .set(guilds::capacity.eq(i16::from(from + CAPACITY_STEP)))
                .execute(conn)
        },
    )
}

/// A guild disbanded.
#[derive(Debug)]
pub struct Disbanded {
    /// Every member's id, as it was.
    pub members: Vec<i32>,
    /// What became of its alliance, if it was in one.
    pub alliance: Option<GuildGone>,
}

/// Disband the guild, if member `by` is its master; its members go with it,
/// and, in the same transaction, its place in an alliance. Returns `None`
/// if `by` may not.
pub fn disband(guild_id: i32, by: i32) -> crate::Result<Option<Disbanded>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let alliance_id = guilds::table
            .find(guild_id)
            .select(guilds::alliance_id)
            .first::<i32>(conn)
            .optional()?;
        if let Some(alliance_id) = alliance_id.filter(|&id| id != 0) {
            crate::alliance::lock(conn, alliance_id)?;
        }
        let Some(guild) = locked(conn, guild_id)?.filter(|guild| guild.is_master(by)) else {
            return Ok(None);
        };
        diesel::delete(guilds::table.find(guild_id)).execute(conn)?;
        let alliance = match guild.alliance_id {
            0 => None,
            alliance_id => crate::alliance::guild_gone(conn, alliance_id)?,
        };
        Ok(Some(Disbanded {
            members: guild.members.iter().map(|member| member.id).collect(),
            alliance,
        }))
    })?)
}

/// Check a change against the guild and make it, with the guild locked so
/// another change can't slip in between. A guild disbanded meanwhile
/// allows nothing.
fn change(
    guild_id: i32,
    allowed: impl FnOnce(&Guild) -> bool,
    update: impl FnOnce(&mut PgConnection) -> QueryResult<usize>,
) -> crate::Result<Option<Vec<i32>>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(guild) = locked(conn, guild_id)?.filter(|guild| allowed(guild)) else {
            return Ok(None);
        };
        update(conn)?;
        Ok(Some(guild.members.iter().map(|member| member.id).collect()))
    })?)
}

/// The guild, locked until the transaction ends, unless it was disbanded.
fn locked(conn: &mut PgConnection, guild_id: i32) -> QueryResult<Option<Guild>> {
    let found = guilds::table
        .find(guild_id)
        .select(guilds::id)
        .for_update()
        .first::<i32>(conn)
        .optional()?;
    match found {
        Some(_) => load_in(conn, guild_id),
        None => Ok(None),
    }
}

/// The guild with its members, the master first, unless it was disbanded.
/// Members are not marked online; the world directory knows who is.
pub fn load(guild_id: i32) -> crate::Result<Option<Guild>> {
    let mut conn = crate::connection()?;
    Ok(load_in(&mut conn, guild_id)?)
}

pub(crate) fn load_in(conn: &mut PgConnection, guild_id: i32) -> QueryResult<Option<Guild>> {
    let Some(row) = guilds::table
        .find(guild_id)
        .first::<GuildRow>(conn)
        .optional()?
    else {
        return Ok(None);
    };
    let members = guild_members::table
        .inner_join(characters::table)
        .filter(guild_members::guild_id.eq(guild_id))
        .order((guild_members::rank, characters::name))
        .select((
            characters::id,
            characters::name,
            characters::job,
            characters::level,
            guild_members::rank,
            guild_members::alliance_rank,
        ))
        .load::<(i32, String, i16, i16, i16, i16)>(conn)?
        .into_iter()
        .map(|(id, name, job, level, rank, alliance_rank)| GuildMember {
            id,
            name,
            job,
            level: level as u8,
            rank: rank as u8,
            alliance_rank: alliance_rank as u8,
            online: false,
        })
        .collect();
    Ok(Some(Guild {
        id: row.id,
        name: row.name,
        titles: [row.title1, row.title2, row.title3, row.title4, row.title5],
        capacity: row.capacity as u8,
        emblem: emblem(row.mark_bg, row.mark_bg_color, row.mark, row.mark_color),
        notice: row.notice,
        points: row.points,
        alliance_id: row.alliance_id,
        members,
    }))
}

/// What others see of `cid`'s guild, if they are in one.
pub fn tag_of(conn: &mut PgConnection, cid: i32) -> QueryResult<Option<GuildTag>> {
    Ok(guild_members::table
        .inner_join(guilds::table)
        .filter(guild_members::character_id.eq(cid))
        .select((
            guilds::id,
            guilds::name,
            guilds::mark_bg,
            guilds::mark_bg_color,
            guilds::mark,
            guilds::mark_color,
            guilds::alliance_id,
        ))
        .first::<(i32, String, i16, i16, i16, i16, i32)>(conn)
        .optional()?
        .map(
            |(id, name, bg, bg_color, mark, mark_color, alliance_id)| GuildTag {
                id,
                name,
                emblem: emblem(bg, bg_color, mark, mark_color),
                alliance_id,
            },
        ))
}

/// What others see of `cid`'s guild, read again after someone else changed
/// it.
pub fn tag(cid: i32) -> crate::Result<Option<GuildTag>> {
    let mut conn = crate::connection()?;
    Ok(tag_of(&mut conn, cid)?)
}

/// Every member of the guild, online or not.
pub fn member_ids(guild_id: i32) -> crate::Result<Vec<i32>> {
    let mut conn = crate::connection()?;
    Ok(guild_members::table
        .filter(guild_members::guild_id.eq(guild_id))
        .select(guild_members::character_id)
        .load(&mut conn)?)
}

/// The world's guilds with the most points, as many as the guild ranking
/// lists; the older first among equals.
pub fn ranking(world_id: u8) -> crate::Result<Vec<Standing>> {
    let mut conn = crate::connection()?;
    Ok(guilds::table
        .filter(guilds::world_id.eq(i16::from(world_id)))
        .order((guilds::points.desc(), guilds::id))
        .limit(RANKING_SIZE as i64)
        .select((
            guilds::name,
            guilds::points,
            guilds::mark_bg,
            guilds::mark_bg_color,
            guilds::mark,
            guilds::mark_color,
        ))
        .load::<(String, i32, i16, i16, i16, i16)>(&mut conn)?
        .into_iter()
        .map(|(name, points, bg, bg_color, mark, mark_color)| Standing {
            name,
            points,
            emblem: emblem(bg, bg_color, mark, mark_color),
        })
        .collect())
}
