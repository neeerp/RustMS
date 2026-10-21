//! Guild alliances. A guild is in at most one (`guilds.alliance_id`, 0 for
//! none); each member's alliance rank is kept on their guild membership.

use crate::schema::{alliances, guild_members, guilds};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use game::alliance::{self, Alliance, GUILD_MASTER_RANK, LOWEST_RANK, MASTER_RANK};
use game::guild::{self as guild_model, Guild};

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// Why founding an alliance failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateError {
    /// Another alliance in the world has the name, ignoring case.
    NameTaken,
    /// The leader leads no guild, the others not exactly one, or a guild
    /// is in an alliance already.
    NotAllowed,
}

/// Why a guild couldn't join an alliance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinError {
    /// The alliance has as many guilds as it may.
    Full,
    /// The guild is in an alliance already.
    InAlliance,
    /// The alliance or the guild is gone, or the player no longer leads
    /// the guild.
    Gone,
}

/// A guild out of its alliance: the alliance as it is now, and the guild
/// as it left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Removed {
    pub alliance: Alliance,
    pub guild: Guild,
}

/// What became of an alliance one of whose guilds was disbanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildGone {
    /// It was the master's guild, so the alliance went too; as it was.
    Disbanded(Alliance),
    /// The alliance as it is without the guild.
    Remains(Alliance),
}

#[derive(Queryable)]
struct AllianceRow {
    id: i32,
    _world_id: i16,
    name: String,
    title1: String,
    title2: String,
    title3: String,
    title4: String,
    title5: String,
    capacity: i16,
    notice: String,
}

/// Found an alliance named `name` in `world_id` of the guild `leader`
/// leads and the one guild a character among `others` leads; `leader` is
/// its master.
pub fn create(
    world_id: u8,
    name: &str,
    leader: i32,
    others: &[i32],
) -> crate::Result<Result<Alliance, CreateError>> {
    let mut conn = crate::connection()?;
    let created = conn.transaction(|conn| -> QueryResult<_> {
        let mut partners = Vec::new();
        for &other in others {
            if led_guild(conn, other)?.is_some() {
                partners.push(other);
            }
        }
        let [partner] = partners[..] else {
            return Ok(Err(CreateError::NotAllowed));
        };
        let masters = [leader, partner];
        let mut guild_ids = Vec::new();
        for master in masters {
            let Some(guild_id) = led_guild(conn, master)? else {
                return Ok(Err(CreateError::NotAllowed));
            };
            let free = guilds::table
                .find(guild_id)
                .select(guilds::alliance_id)
                .for_update()
                .first::<i32>(conn)?
                == 0;
            if !free || guild_ids.contains(&guild_id) {
                return Ok(Err(CreateError::NotAllowed));
            }
            guild_ids.push(guild_id);
        }
        let id = diesel::insert_into(alliances::table)
            .values((
                alliances::world_id.eq(i16::from(world_id)),
                alliances::name.eq(name),
                alliances::capacity.eq(i16::from(alliance::START_CAPACITY)),
            ))
            .returning(alliances::id)
            .get_result::<i32>(conn)?;
        for (guild_id, master, rank) in [
            (guild_ids[0], masters[0], MASTER_RANK),
            (guild_ids[1], masters[1], GUILD_MASTER_RANK),
        ] {
            join(conn, id, guild_id)?;
            diesel::update(guild_members::table.find(master))
                .set(guild_members::alliance_rank.eq(i16::from(rank)))
                .execute(conn)?;
        }
        Ok(load_in(conn, id)?.ok_or(DieselError::NotFound).map(Ok)?)
    });
    match created {
        // The unique index on the world and lowercased name.
        Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            Ok(Err(CreateError::NameTaken))
        }
        other => Ok(other?),
    }
}

/// The guild `cid` is the master of.
fn led_guild(conn: &mut PgConnection, cid: i32) -> QueryResult<Option<i32>> {
    guild_members::table
        .find(cid)
        .filter(guild_members::rank.eq(i16::from(guild_model::MASTER_RANK)))
        .select(guild_members::guild_id)
        .first(conn)
        .optional()
}

/// Put the guild in the alliance, its members at the lowest rank.
fn join(conn: &mut PgConnection, alliance_id: i32, guild_id: i32) -> QueryResult<()> {
    diesel::update(guilds::table.find(guild_id))
        .set(guilds::alliance_id.eq(alliance_id))
        .execute(conn)?;
    reset_ranks(conn, guild_id)
}

/// The guild's members at the lowest rank, as in no alliance.
fn reset_ranks(conn: &mut PgConnection, guild_id: i32) -> QueryResult<()> {
    diesel::update(guild_members::table.filter(guild_members::guild_id.eq(guild_id)))
        .set(guild_members::alliance_rank.eq(i16::from(LOWEST_RANK)))
        .execute(conn)?;
    Ok(())
}

/// Bring the guild `master` leads into the alliance, if there is room.
/// Returns the alliance with it.
pub fn add_guild(alliance_id: i32, master: i32) -> crate::Result<Result<Alliance, JoinError>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(alliance) = locked(conn, alliance_id)? else {
            return Ok(Err(JoinError::Gone));
        };
        let Some(guild_id) = led_guild(conn, master)? else {
            return Ok(Err(JoinError::Gone));
        };
        // The guild may have been disbanded since.
        let in_alliance = guilds::table
            .find(guild_id)
            .select(guilds::alliance_id)
            .for_update()
            .first::<i32>(conn)
            .optional()?;
        match in_alliance {
            None => return Ok(Err(JoinError::Gone)),
            Some(0) => {}
            Some(_) => return Ok(Err(JoinError::InAlliance)),
        }
        if alliance.guilds.len() >= usize::from(alliance.capacity) {
            return Ok(Err(JoinError::Full));
        }
        join(conn, alliance_id, guild_id)?;
        diesel::update(guild_members::table.find(master))
            .set(guild_members::alliance_rank.eq(i16::from(GUILD_MASTER_RANK)))
            .execute(conn)?;
        Ok(load_in(conn, alliance_id)?
            .ok_or(DieselError::NotFound)
            .map(Ok)?)
    })?)
}

/// Take member `by`'s guild out of the alliance, if they may
/// (`Alliance::leaving_guild`).
pub fn leave(alliance_id: i32, by: i32) -> crate::Result<Option<Removed>> {
    remove(alliance_id, |alliance| {
        alliance.leaving_guild(by).map(|guild| guild.id)
    })
}

/// Put guild `guild_id` out of the alliance, if member `by` may.
pub fn expel(alliance_id: i32, by: i32, guild_id: i32) -> crate::Result<Option<Removed>> {
    remove(alliance_id, |alliance| {
        alliance.may_expel(by, guild_id).then_some(guild_id)
    })
}

fn remove(
    alliance_id: i32,
    which: impl FnOnce(&Alliance) -> Option<i32>,
) -> crate::Result<Option<Removed>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(guild_id) = locked(conn, alliance_id)?.and_then(|alliance| which(&alliance))
        else {
            return Ok(None);
        };
        diesel::update(guilds::table.find(guild_id))
            .set(guilds::alliance_id.eq(0))
            .execute(conn)?;
        reset_ranks(conn, guild_id)?;
        let alliance = load_in(conn, alliance_id)?.ok_or(DieselError::NotFound)?;
        // A guild disbanded meanwhile is gone already.
        let guild = crate::guild::load_in(conn, guild_id)?;
        Ok(guild.map(|guild| Removed { alliance, guild }))
    })?)
}

/// Hand the alliance to `to`, if member `by` may. Returns the alliance
/// before the change.
pub fn change_master(alliance_id: i32, by: i32, to: i32) -> crate::Result<Option<Alliance>> {
    change(
        alliance_id,
        |alliance| alliance.may_change_master(by, to),
        |conn| {
            set_member_rank(conn, by, GUILD_MASTER_RANK)?;
            set_member_rank(conn, to, MASTER_RANK)
        },
    )
}

fn set_member_rank(conn: &mut PgConnection, cid: i32, rank: u8) -> QueryResult<usize> {
    diesel::update(guild_members::table.find(cid))
        .set(guild_members::alliance_rank.eq(i16::from(rank)))
        .execute(conn)
}

/// Move `member_id` a rank up or down, if member `by` may. Returns the
/// new rank and the alliance before the change.
pub fn set_rank(
    alliance_id: i32,
    by: i32,
    member_id: i32,
    up: bool,
) -> crate::Result<Option<(u8, Alliance)>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(alliance) = locked(conn, alliance_id)? else {
            return Ok(None);
        };
        let Some(rank) = alliance.rank_change(by, member_id, up) else {
            return Ok(None);
        };
        set_member_rank(conn, member_id, rank)?;
        Ok(Some((rank, alliance)))
    })?)
}

/// Give the ranks these titles, if member `by` may; `forbids` is the
/// forbidden-word check. Returns the alliance before the change.
pub fn set_titles(
    alliance_id: i32,
    by: i32,
    titles: &[String; 5],
    forbids: impl Fn(&str) -> bool,
) -> crate::Result<Option<Alliance>> {
    change(
        alliance_id,
        |alliance| alliance.may_set_titles(by, titles, forbids),
        |conn| {
            diesel::update(alliances::table.find(alliance_id))
                .set((
                    alliances::title1.eq(&titles[0]),
                    alliances::title2.eq(&titles[1]),
                    alliances::title3.eq(&titles[2]),
                    alliances::title4.eq(&titles[3]),
                    alliances::title5.eq(&titles[4]),
                ))
                .execute(conn)
        },
    )
}

/// Post the alliance's notice, if member `by` may. Returns the alliance
/// before the change.
pub fn set_notice(alliance_id: i32, by: i32, notice: &str) -> crate::Result<Option<Alliance>> {
    change(
        alliance_id,
        |alliance| alliance.may_set_notice(by, notice),
        |conn| {
            diesel::update(alliances::table.find(alliance_id))
                .set(alliances::notice.eq(notice))
                .execute(conn)
        },
    )
}

/// Make room for one more guild than `from`, if member `by` is the
/// master and Lenario may still grow it. Returns the alliance before.
pub fn grow(alliance_id: i32, by: i32, from: u8) -> crate::Result<Option<Alliance>> {
    change(
        alliance_id,
        |alliance| {
            alliance.is_master(by) && alliance.capacity == from && from < alliance::MAX_CAPACITY
        },
        |conn| {
            diesel::update(alliances::table.find(alliance_id))
                .set(alliances::capacity.eq(i16::from(from + 1)))
                .execute(conn)
        },
    )
}

/// Disband the alliance, if member `by` is its master. Returns it as it
/// was.
pub fn disband(alliance_id: i32, by: i32) -> crate::Result<Option<Alliance>> {
    change(
        alliance_id,
        |alliance| alliance.is_master(by),
        |conn| dissolve(conn, alliance_id),
    )
}

/// The alliance and its guilds part.
fn dissolve(conn: &mut PgConnection, alliance_id: i32) -> QueryResult<usize> {
    let guild_ids: Vec<i32> = guilds::table
        .filter(guilds::alliance_id.eq(alliance_id))
        .select(guilds::id)
        .load(conn)?;
    for &guild_id in &guild_ids {
        reset_ranks(conn, guild_id)?;
    }
    diesel::update(guilds::table.filter(guilds::alliance_id.eq(alliance_id)))
        .set(guilds::alliance_id.eq(0))
        .execute(conn)?;
    diesel::delete(alliances::table.find(alliance_id)).execute(conn)
}

/// One of the alliance's guilds was disbanded. Without its master's guild
/// the alliance goes too.
pub fn guild_gone(alliance_id: i32) -> crate::Result<Option<GuildGone>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(alliance) = locked(conn, alliance_id)? else {
            return Ok(None);
        };
        if alliance.master().is_some() {
            return Ok(Some(GuildGone::Remains(alliance)));
        }
        dissolve(conn, alliance_id)?;
        Ok(Some(GuildGone::Disbanded(alliance)))
    })?)
}

/// Check a change against the alliance and make it, with the alliance
/// locked so another change can't slip in between. Returns the alliance
/// as it was before.
fn change(
    alliance_id: i32,
    allowed: impl FnOnce(&Alliance) -> bool,
    update: impl FnOnce(&mut PgConnection) -> QueryResult<usize>,
) -> crate::Result<Option<Alliance>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(alliance) = locked(conn, alliance_id)?.filter(|alliance| allowed(alliance)) else {
            return Ok(None);
        };
        update(conn)?;
        Ok(Some(alliance))
    })?)
}

/// The alliance, locked until the transaction ends, unless it was
/// disbanded.
fn locked(conn: &mut PgConnection, alliance_id: i32) -> QueryResult<Option<Alliance>> {
    let found = alliances::table
        .find(alliance_id)
        .select(alliances::id)
        .for_update()
        .first::<i32>(conn)
        .optional()?;
    match found {
        Some(_) => load_in(conn, alliance_id),
        None => Ok(None),
    }
}

/// The alliance with its guilds, by id, unless it was disbanded. Members
/// are not marked online; the world directory knows who is.
pub fn load(alliance_id: i32) -> crate::Result<Option<Alliance>> {
    let mut conn = crate::connection()?;
    Ok(load_in(&mut conn, alliance_id)?)
}

fn load_in(conn: &mut PgConnection, alliance_id: i32) -> QueryResult<Option<Alliance>> {
    let Some(row) = alliances::table
        .find(alliance_id)
        .first::<AllianceRow>(conn)
        .optional()?
    else {
        return Ok(None);
    };
    let guild_ids: Vec<i32> = guilds::table
        .filter(guilds::alliance_id.eq(alliance_id))
        .order(guilds::id)
        .select(guilds::id)
        .load(conn)?;
    let mut guilds = Vec::new();
    for guild_id in guild_ids {
        guilds.extend(crate::guild::load_in(conn, guild_id)?);
    }
    Ok(Some(Alliance {
        id: row.id,
        name: row.name,
        titles: [row.title1, row.title2, row.title3, row.title4, row.title5],
        capacity: row.capacity as u8,
        notice: row.notice,
        guilds,
    }))
}

/// Everyone in the alliance's guilds.
pub fn member_ids(alliance_id: i32) -> crate::Result<Vec<i32>> {
    let mut conn = crate::connection()?;
    Ok(guild_members::table
        .inner_join(guilds::table)
        .filter(guilds::alliance_id.eq(alliance_id))
        .select(guild_members::character_id)
        .load(&mut conn)?)
}

/// The alliance's name, for the character info window.
pub fn name(alliance_id: i32) -> crate::Result<Option<String>> {
    let mut conn = crate::connection()?;
    Ok(alliances::table
        .find(alliance_id)
        .select(alliances::name)
        .first(&mut conn)
        .optional()?)
}

/// The guild named `name` in `world_id`, ignoring case: its id, its
/// alliance's, and its master's id.
pub fn find_guild(world_id: u8, name: &str) -> crate::Result<Option<(i32, i32, i32)>> {
    let mut conn = crate::connection()?;
    Ok(guilds::table
        .inner_join(guild_members::table)
        .filter(guilds::world_id.eq(i16::from(world_id)))
        .filter(lower(guilds::name).eq(lower(name)))
        .filter(guild_members::rank.eq(i16::from(guild_model::MASTER_RANK)))
        .select((guilds::id, guilds::alliance_id, guild_members::character_id))
        .first(&mut conn)
        .optional()?)
}
