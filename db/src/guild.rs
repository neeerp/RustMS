//! Guilds and their members. A character is in at most one guild (the
//! member row's key); rank 1 is the master.

use crate::schema::{characters, guild_members, guilds};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use game::guild::{Emblem, Guild, GuildMember, GuildTag, MASTER_RANK, MEMBER_RANK};

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

/// Found a guild named `name` in `world_id` with `master` as its only
/// member. Returns the new guild's id.
pub fn create(world_id: u8, name: &str, master: i32) -> crate::Result<Result<i32, CreateError>> {
    let mut conn = crate::connection()?;
    let created = conn.transaction(|conn| -> QueryResult<_> {
        let member = guild_members::table
            .find(master)
            .select(guild_members::guild_id)
            .first::<i32>(conn)
            .optional()?;
        if member.is_some() {
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

/// Add `cid` to the guild at the lowest rank, if there is room.
pub fn add_member(guild_id: i32, cid: i32) -> crate::Result<Result<(), JoinError>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        // Lock the guild so members joining at once can't pass its capacity.
        let capacity = guilds::table
            .find(guild_id)
            .select(guilds::capacity)
            .for_update()
            .first::<i16>(conn)?;
        let member = guild_members::table
            .find(cid)
            .select(guild_members::guild_id)
            .first::<i32>(conn)
            .optional()?;
        if member.is_some() {
            return Ok(Err(JoinError::AlreadyInGuild));
        }
        let count: i64 = guild_members::table
            .filter(guild_members::guild_id.eq(guild_id))
            .count()
            .get_result(conn)?;
        if count >= i64::from(capacity) {
            return Ok(Err(JoinError::Full));
        }
        diesel::insert_into(guild_members::table)
            .values((
                guild_members::character_id.eq(cid),
                guild_members::guild_id.eq(guild_id),
                guild_members::rank.eq(i16::from(MEMBER_RANK)),
            ))
            .execute(conn)?;
        Ok(Ok(()))
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

/// The guild with its members, the master first. Members are not marked
/// online; the world directory knows who is.
pub fn load(guild_id: i32) -> crate::Result<Guild> {
    let mut conn = crate::connection()?;
    let row: GuildRow = guilds::table.find(guild_id).first(&mut conn)?;
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
        ))
        .load::<(i32, String, i16, i16, i16)>(&mut conn)?
        .into_iter()
        .map(|(id, name, job, level, rank)| GuildMember {
            id,
            name,
            job,
            level: level as u8,
            rank: rank as u8,
            online: false,
        })
        .collect();
    Ok(Guild {
        id: row.id,
        name: row.name,
        titles: [row.title1, row.title2, row.title3, row.title4, row.title5],
        capacity: row.capacity as u8,
        emblem: emblem(row.mark_bg, row.mark_bg_color, row.mark, row.mark_color),
        notice: row.notice,
        points: row.points,
        alliance_id: row.alliance_id,
        members,
    })
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
        ))
        .first::<(i32, String, i16, i16, i16, i16)>(conn)
        .optional()?
        .map(|(id, name, bg, bg_color, mark, mark_color)| GuildTag {
            id,
            name,
            emblem: emblem(bg, bg_color, mark, mark_color),
        }))
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
