//! Buddy lists. A row `(owner, buddy)` puts `buddy` on `owner`'s list; a
//! pending row is a request from `buddy` that `owner` hasn't answered. A
//! buddy is mutual when each has the other on their list, and only mutual
//! buddies see each other's channel.

use crate::schema::{buddies, buddy_capacities, characters};
use diesel::prelude::*;
use diesel::sql_types::Text;
use std::collections::HashSet;

diesel::define_sql_function!(fn lower(x: Text) -> Text);

pub const DEFAULT_GROUP: &str = "Default Group";

/// One row of a character's buddy list, or a request to answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buddy {
    pub id: i32,
    pub name: String,
    pub group: String,
    /// They asked to be added; not on the list until accepted.
    pub pending: bool,
    /// They have this character on their list too.
    pub mutual: bool,
}

/// Why adding a buddy failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddError {
    NotFound,
    AlreadyAdded,
    OwnListFull,
    TheirListFull,
}

/// What adding a buddy did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added {
    pub id: i32,
    pub name: String,
    /// They already had this character, so both now see each other.
    pub mutual: bool,
    /// They were already on the list, in another group, and moved to this one.
    pub regrouped: bool,
}

/// Every row of `cid`'s list, pending requests included.
pub fn list(cid: i32) -> crate::Result<Vec<Buddy>> {
    let mut conn = crate::connection()?;
    let rows: Vec<(i32, String, String, bool)> = buddies::table
        .inner_join(characters::table.on(characters::id.eq(buddies::buddy_id)))
        .filter(buddies::character_id.eq(cid))
        .order(characters::name)
        .select((
            buddies::buddy_id,
            characters::name,
            buddies::group_name,
            buddies::pending,
        ))
        .load(&mut conn)?;
    let listed_by = listed_by(&mut conn, cid)?;
    Ok(rows
        .into_iter()
        .map(|(id, name, group, pending)| Buddy {
            mutual: !pending && listed_by.contains(&id),
            id,
            name,
            group,
            pending,
        })
        .collect())
}

/// Buddies who see `cid`'s channel: those `cid` and they both list.
pub fn mutual_ids(cid: i32) -> crate::Result<Vec<i32>> {
    Ok(list(cid)?
        .into_iter()
        .filter(|buddy| buddy.mutual)
        .map(|buddy| buddy.id)
        .collect())
}

/// Characters whose (accepted) list holds `cid`.
fn listed_by(conn: &mut PgConnection, cid: i32) -> QueryResult<HashSet<i32>> {
    Ok(buddies::table
        .filter(buddies::buddy_id.eq(cid))
        .filter(buddies::pending.eq(false))
        .select(buddies::character_id)
        .load::<i32>(conn)?
        .into_iter()
        .collect())
}

/// Add the character called `name` to `cid`'s list, asking them to add
/// `cid` back unless they already have. Accepting their request has the same
/// effect. Adding a buddy already listed under another group moves them to
/// `group`, which is how v83 changes groups. `capacity` bounds `cid`'s list;
/// the target's own capacity bounds theirs.
pub fn add(
    cid: i32,
    name: &str,
    group: &str,
    capacity: u8,
) -> crate::Result<Result<Added, AddError>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let target = characters::table
            .filter(lower(characters::name).eq(lower(name)))
            .select((characters::id, characters::name))
            .first::<(i32, String)>(conn)
            .optional()?;
        let Some((target, target_name)) = target.filter(|(id, _)| *id != cid) else {
            return Ok(Err(AddError::NotFound));
        };
        // Lock both characters in id order so crossed requests serialize.
        characters::table
            .filter(characters::id.eq_any([cid, target]))
            .order(characters::id)
            .select(characters::id)
            .for_update()
            .load::<i32>(conn)?;

        let existing = buddies::table
            .find((cid, target))
            .select((buddies::pending, buddies::group_name))
            .first::<(bool, String)>(conn)
            .optional()?;
        match existing {
            Some((false, current)) if current == group => return Ok(Err(AddError::AlreadyAdded)),
            Some((false, _)) => {
                diesel::update(buddies::table.find((cid, target)))
                    .set(buddies::group_name.eq(group))
                    .execute(conn)?;
                let mutual = listed_by(conn, cid)?.contains(&target);
                return Ok(Ok(Added {
                    id: target,
                    name: target_name,
                    mutual,
                    regrouped: true,
                }));
            }
            Some((true, _)) => {
                accept_row(conn, cid, target, group)?;
                return Ok(Ok(Added {
                    id: target,
                    name: target_name,
                    mutual: true,
                    regrouped: false,
                }));
            }
            None => {}
        }
        if count(conn, cid, Some(false))? >= i64::from(capacity) {
            return Ok(Err(AddError::OwnListFull));
        }
        let theirs = buddies::table
            .find((target, cid))
            .select(buddies::pending)
            .first::<bool>(conn)
            .optional()?;
        let their_capacity = buddy_capacities::table
            .find(target)
            .select(buddy_capacities::capacity)
            .first::<i16>(conn)
            .optional()?
            .unwrap_or(i16::from(game::DEFAULT_BUDDY_CAPACITY));
        if theirs.is_none() && count(conn, target, None)? >= i64::from(their_capacity) {
            return Ok(Err(AddError::TheirListFull));
        }

        diesel::insert_into(buddies::table)
            .values((
                buddies::character_id.eq(cid),
                buddies::buddy_id.eq(target),
                buddies::group_name.eq(group),
            ))
            .execute(conn)?;
        if theirs.is_none() {
            diesel::insert_into(buddies::table)
                .values((
                    buddies::character_id.eq(target),
                    buddies::buddy_id.eq(cid),
                    buddies::pending.eq(true),
                ))
                .execute(conn)?;
        }
        Ok(Ok(Added {
            id: target,
            name: target_name,
            mutual: theirs == Some(false),
            regrouped: false,
        }))
    })?)
}

fn count(conn: &mut PgConnection, cid: i32, pending: Option<bool>) -> QueryResult<i64> {
    let mut query = buddies::table
        .filter(buddies::character_id.eq(cid))
        .into_boxed();
    if let Some(pending) = pending {
        query = query.filter(buddies::pending.eq(pending));
    }
    query.count().get_result(conn)
}

fn accept_row(conn: &mut PgConnection, cid: i32, buddy: i32, group: &str) -> QueryResult<usize> {
    diesel::update(buddies::table.find((cid, buddy)))
        .filter(buddies::pending.eq(true))
        .set((buddies::pending.eq(false), buddies::group_name.eq(group)))
        .execute(conn)
}

/// Accept `buddy`'s request. Returns whether there was one.
pub fn accept(cid: i32, buddy: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    Ok(accept_row(&mut conn, cid, buddy, DEFAULT_GROUP)? == 1)
}

/// Take `buddy` off `cid`'s list, or decline their request. Returns the
/// removed row's `pending` flag, or `None` if there was no row.
pub fn remove(cid: i32, buddy: i32) -> crate::Result<Option<bool>> {
    let mut conn = crate::connection()?;
    Ok(diesel::delete(buddies::table.find((cid, buddy)))
        .returning(buddies::pending)
        .get_result::<bool>(&mut conn)
        .optional()?)
}
