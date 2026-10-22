//! Families (report b039). A row's `senior_id` is the character's senior;
//! juniors are the rows naming them. The senior at the top leads the family,
//! which goes by their name, and keeps its message.

use crate::schema::{characters, family_members};
use diesel::prelude::*;
use game::family::{
    ChartMember, FamilyChart, FamilyInfo, Refusal, Relatives, MAX_GENERATIONS, MAX_JUNIORS,
};

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// The id of the character named `name`, ignoring case.
pub fn character(name: &str) -> crate::Result<Option<i32>> {
    Ok(characters::table
        .filter(lower(characters::name).eq(lower(name)))
        .select(characters::id)
        .first(&mut crate::connection()?)
        .optional()?)
}

/// The character's senior and juniors, read again.
pub fn relatives_of(character_id: i32) -> crate::Result<Relatives> {
    let mut conn = crate::connection()?;
    Ok(relatives(&mut conn, character_id)?)
}

/// The character's senior and juniors, juniors in the order they joined
/// the table.
pub fn relatives(conn: &mut PgConnection, character_id: i32) -> QueryResult<Relatives> {
    let senior = family_members::table
        .find(character_id)
        .select(family_members::senior_id)
        .first::<Option<i32>>(conn)
        .optional()?
        .flatten();
    Ok(Relatives {
        senior,
        juniors: juniors(conn, character_id)?,
    })
}

fn juniors(conn: &mut PgConnection, character_id: i32) -> QueryResult<Vec<i32>> {
    family_members::table
        .filter(family_members::senior_id.eq(character_id))
        .order(family_members::character_id)
        .select(family_members::character_id)
        .load(conn)
}

fn senior_of(conn: &mut PgConnection, character_id: i32) -> QueryResult<Option<i32>> {
    Ok(family_members::table
        .find(character_id)
        .select(family_members::senior_id)
        .first::<Option<i32>>(conn)
        .optional()?
        .flatten())
}

/// The character's seniors, nearest first.
fn seniors(conn: &mut PgConnection, character_id: i32) -> QueryResult<Vec<i32>> {
    let mut seniors = Vec::new();
    let mut at = character_id;
    while let Some(senior) = senior_of(conn, at)? {
        // A loop would be a bug; stop at the most a family may span.
        if seniors.len() >= MAX_GENERATIONS || senior == character_id {
            break;
        }
        seniors.push(senior);
        at = senior;
    }
    Ok(seniors)
}

/// The character and everyone below them, generation by generation, and
/// how many generations that is.
fn descendants(conn: &mut PgConnection, character_id: i32) -> QueryResult<(Vec<i32>, usize)> {
    let mut all = vec![character_id];
    let mut generation = vec![character_id];
    let mut generations = 1;
    while generations < MAX_GENERATIONS {
        generation = family_members::table
            .filter(family_members::senior_id.eq_any(&generation))
            .select(family_members::character_id)
            .load(conn)?;
        if generation.is_empty() {
            break;
        }
        all.extend(&generation);
        generations += 1;
    }
    Ok((all, generations))
}

/// What the Family window shows the character. Out of a family, only their
/// own rep, with no family name.
pub fn info(character_id: i32) -> crate::Result<FamilyInfo> {
    let mut conn = crate::connection()?;
    let row = family_members::table
        .find(character_id)
        .select((
            family_members::reputation,
            family_members::total_reputation,
            family_members::today_reputation,
        ))
        .first::<(i32, i32, i32)>(&mut conn)
        .optional()?;
    let (reputation, total_reputation, today_reputation) = row.unwrap_or_default();
    let juniors = juniors(&mut conn, character_id)?;
    let seniors = seniors(&mut conn, character_id)?;
    let mut info = FamilyInfo {
        reputation,
        total_reputation,
        today_reputation,
        juniors: juniors.len() as u16,
        ..FamilyInfo::default()
    };
    if seniors.is_empty() && juniors.is_empty() {
        return Ok(info);
    }
    let leader = seniors.last().copied().unwrap_or(character_id);
    let (name, precepts) = characters::table
        .inner_join(family_members::table)
        .filter(characters::id.eq(leader))
        .select((characters::name, family_members::precepts))
        .first::<(String, String)>(&mut conn)?;
    info.leader_id = leader;
    info.name = name;
    info.precepts = precepts;
    Ok(info)
}

/// The Family Tree around `viewed`, without who is online.
pub fn chart(viewed: i32) -> crate::Result<FamilyChart> {
    let mut conn = crate::connection()?;
    let conn = &mut conn;
    let mut chart = FamilyChart {
        viewed,
        ..FamilyChart::default()
    };
    let seniors = seniors(conn, viewed)?;
    let juniors = juniors(conn, viewed)?;
    if seniors.is_empty() && juniors.is_empty() {
        return Ok(chart);
    }
    let leader = seniors.last().copied().unwrap_or(viewed);
    let mut ids = vec![leader];
    ids.extend(seniors.iter().take(2).rev());
    ids.push(viewed);
    if let Some(&senior) = seniors.first() {
        ids.extend(self::juniors(conn, senior)?);
    }
    ids.extend(&juniors);
    for &junior in &juniors {
        ids.extend(self::juniors(conn, junior)?);
    }
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| seen.insert(*id));

    let rows = characters::table
        .inner_join(family_members::table)
        .filter(characters::id.eq_any(&ids))
        .select((
            characters::id,
            family_members::senior_id,
            characters::job,
            characters::level,
            family_members::reputation,
            family_members::total_reputation,
            family_members::senior_reputation,
            family_members::today_reputation,
            characters::map_id,
            characters::name,
        ))
        .load::<(i32, Option<i32>, i16, i16, i32, i32, i32, i32, i32, String)>(conn)?;
    let mut rows: std::collections::HashMap<_, _> =
        rows.into_iter().map(|row| (row.0, row)).collect();
    chart.members = ids
        .iter()
        .filter_map(|id| rows.remove(id))
        .map(|row| ChartMember {
            id: row.0,
            senior_id: row.1.unwrap_or(0),
            job: row.2,
            level: row.3 as u8,
            online: false,
            reputation: row.4,
            total_reputation: row.5,
            senior_reputation: row.6,
            today_reputation: row.7,
            map_id: row.8,
            name: row.9,
        })
        .collect();

    let family = descendants(conn, leader)?.0.len();
    chart.counts = vec![(-1, family as i32), (0, seniors.len() as i32)];
    for &junior in &juniors {
        let branch = descendants(conn, junior)?.0.len();
        chart.counts.push((junior, branch as i32));
    }
    Ok(chart)
}

/// Whether `senior` may take `junior`, by the family rules (levels and
/// the map are the caller's).
fn check(conn: &mut PgConnection, senior: i32, junior: i32) -> QueryResult<Result<(), Refusal>> {
    if senior == junior {
        return Ok(Err(Refusal::CannotAdd));
    }
    if senior_of(conn, junior)?.is_some() {
        return Ok(Err(Refusal::HasSenior));
    }
    let seniors = seniors(conn, senior)?;
    if seniors.contains(&junior) {
        return Ok(Err(Refusal::SameFamily));
    }
    if juniors(conn, senior)?.len() >= usize::from(MAX_JUNIORS) {
        return Ok(Err(Refusal::CannotAdd));
    }
    let (_, below) = descendants(conn, junior)?;
    if seniors.len() + 1 + below > MAX_GENERATIONS {
        return Ok(Err(Refusal::Generations));
    }
    Ok(Ok(()))
}

/// Whether `senior` may ask `junior`, as it stands.
pub fn may_add(senior: i32, junior: i32) -> crate::Result<Result<(), Refusal>> {
    let mut conn = crate::connection()?;
    Ok(check(&mut conn, senior, junior)?)
}

/// Make `junior` (and everyone below them) `senior`'s. On success, everyone
/// whose family changed: the senior and the junior's whole branch.
pub fn add_junior(senior: i32, junior: i32) -> crate::Result<Result<Vec<i32>, Refusal>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let mut both = [senior, junior];
        both.sort_unstable();
        for id in both {
            diesel::insert_into(family_members::table)
                .values(family_members::character_id.eq(id))
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        // Two adds to one senior, or each of two to the other, take turns.
        family_members::table
            .filter(family_members::character_id.eq_any(both))
            .order(family_members::character_id)
            .select(family_members::character_id)
            .for_update()
            .load::<i32>(conn)?;
        if let Err(refusal) = check(conn, senior, junior)? {
            return Ok(Err(refusal));
        }
        diesel::update(family_members::table.find(junior))
            .set(family_members::senior_id.eq(senior))
            .execute(conn)?;
        let (mut changed, _) = descendants(conn, junior)?;
        changed.insert(0, senior);
        Ok(Ok(changed))
    })?)
}
