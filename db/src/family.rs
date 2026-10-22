//! Families (report b039). A row's `senior_id` is the character's senior;
//! juniors are the rows naming them. The senior at the top leads the family,
//! which goes by their name, and keeps its message.

use crate::schema::{characters, family_buffs, family_entitlement_uses, family_members};
use diesel::dsl::sql;
use diesel::expression::SqlLiteral;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use game::family::{
    rep_gain, separation_fee, separation_rep, ChartMember, FamilyBuff, FamilyChart, FamilyInfo,
    Refusal, Relatives, ENTITLEMENTS, MAX_GENERATIONS, MAX_JUNIORS, MAX_PRECEPTS,
};
use std::time::SystemTime;

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

/// A counter for today: as written if `rep_day` is today, else 0.
fn today(column: &str) -> SqlLiteral<Integer> {
    sql(&format!(
        "CASE WHEN family_members.rep_day = CURRENT_DATE THEN family_members.{column} ELSE 0 END"
    ))
}

/// Today, by the database's clock.
fn current_date() -> SqlLiteral<diesel::sql_types::Date> {
    sql("CURRENT_DATE")
}

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
            today("today_reputation"),
        ))
        .first::<(i32, i32, i32)>(&mut conn)
        .optional()?;
    let (reputation, total_reputation, today_reputation) = row.unwrap_or_default();
    let juniors = juniors(&mut conn, character_id)?;
    let seniors = seniors(&mut conn, character_id)?;
    let used = family_entitlement_uses::table
        .filter(family_entitlement_uses::character_id.eq(character_id))
        .filter(family_entitlement_uses::used_on.eq(current_date()))
        .filter(family_entitlement_uses::times.gt(0))
        .order(family_entitlement_uses::entitlement)
        .select((
            family_entitlement_uses::entitlement,
            family_entitlement_uses::times,
        ))
        .load::<(i16, i32)>(&mut conn)?
        .into_iter()
        .map(|(index, times)| (i32::from(index), times))
        .collect();
    let mut info = FamilyInfo {
        reputation,
        total_reputation,
        today_reputation,
        juniors: juniors.len() as u16,
        used,
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
    // The client only shows the first entry, the leader, as the top of the
    // tree and links the others by their seniors, so the leader comes again
    // where it falls in that chain.
    let mut ids: Vec<i32> = seniors.iter().take(2).rev().copied().collect();
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
    ids.insert(0, leader);

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
            today("senior_reputation"),
            today("today_reputation"),
            characters::map_id,
            characters::name,
        ))
        .load::<(i32, Option<i32>, i16, i16, i32, i32, i32, i32, i32, String)>(conn)?;
    let rows: std::collections::HashMap<_, _> = rows.into_iter().map(|row| (row.0, row)).collect();
    chart.members = ids
        .iter()
        .filter_map(|id| rows.get(id).cloned())
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

/// The leader's new precepts. Anyone else, or a leader of no one, changes
/// nothing; otherwise everyone in the family, who see them in the window.
pub fn set_precepts(leader: i32, precepts: &str) -> crate::Result<Option<Vec<i32>>> {
    if precepts.chars().count() > MAX_PRECEPTS {
        return Ok(None);
    }
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        if senior_of(conn, leader)?.is_some() || juniors(conn, leader)?.is_empty() {
            return Ok(None);
        }
        diesel::update(family_members::table.find(leader))
            .set(family_members::precepts.eq(precepts))
            .execute(conn)?;
        Ok(Some(descendants(conn, leader)?.0))
    })?)
}

/// `junior` (level `junior_level`) earned their senior and the senior's
/// senior `gain` Rep, halved if the senior (level `senior_level`, else as
/// saved) is below them. Each gets it to spend, in their total and in
/// today's; the junior records the senior's share. Returns who got how
/// much.
pub fn earn(
    junior: i32,
    gain: i32,
    junior_level: u8,
    senior_level: Option<u8>,
) -> crate::Result<Vec<(i32, i32)>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let Some(senior) = senior_of(conn, junior)? else {
            return Ok(Vec::new());
        };
        let senior_level = match senior_level {
            Some(level) => level,
            None => characters::table
                .find(senior)
                .select(characters::level)
                .first::<i16>(conn)?
                .clamp(0, 255) as u8,
        };
        let gain = rep_gain(gain, senior_level, junior_level);
        if gain <= 0 {
            return Ok(Vec::new());
        }
        let earners: Vec<i32> = std::iter::once(senior)
            .chain(senior_of(conn, senior)?)
            .collect();
        // Rows in id order, as `separate` locks them.
        let mut rows: Vec<i32> = earners.iter().copied().chain([junior]).collect();
        rows.sort_unstable();
        for id in rows {
            let (earned, given) = if id == junior { (0, gain) } else { (gain, 0) };
            diesel::sql_query(
                "UPDATE family_members SET \
                 reputation = reputation + $1, \
                 total_reputation = total_reputation + $1, \
                 today_reputation = CASE WHEN rep_day = CURRENT_DATE \
                     THEN today_reputation ELSE 0 END + $1, \
                 senior_reputation = CASE WHEN rep_day = CURRENT_DATE \
                     THEN senior_reputation ELSE 0 END + $2, \
                 rep_day = CURRENT_DATE \
                 WHERE character_id = $3",
            )
            .bind::<Integer, _>(earned)
            .bind::<Integer, _>(given)
            .bind::<Integer, _>(id)
            .execute(conn)?;
        }
        Ok(earners.into_iter().map(|id| (id, gain)).collect())
    })?)
}

/// Whether an entitlement could be paid for.
#[derive(Debug, PartialEq, Eq)]
pub enum Spending {
    Spent,
    /// Not enough Rep to spend.
    Short,
    /// Used as often as it may be today.
    UsedUp,
}

/// Pay `cost` Rep for entitlement `index`, which may be used `limit` times a
/// day. Only the Rep to spend goes down; what was earned stays.
pub fn spend(character_id: i32, index: i16, cost: i32, limit: i32) -> crate::Result<Spending> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        let reputation = family_members::table
            .find(character_id)
            .select(family_members::reputation)
            .for_update()
            .first::<i32>(conn)
            .optional()?;
        if reputation.is_none_or(|reputation| reputation < cost) {
            return Ok(Spending::Short);
        }
        let times = family_entitlement_uses::table
            .find((character_id, index))
            .filter(family_entitlement_uses::used_on.eq(current_date()))
            .select(family_entitlement_uses::times)
            .first::<i32>(conn)
            .optional()?
            .unwrap_or(0);
        if times >= limit {
            return Ok(Spending::UsedUp);
        }
        diesel::update(family_members::table.find(character_id))
            .set(family_members::reputation.eq(family_members::reputation - cost))
            .execute(conn)?;
        diesel::sql_query(
            "INSERT INTO family_entitlement_uses (character_id, entitlement, used_on, times) \
             VALUES ($1, $2, CURRENT_DATE, 1) \
             ON CONFLICT (character_id, entitlement) DO UPDATE SET \
             times = CASE WHEN family_entitlement_uses.used_on = CURRENT_DATE \
                 THEN family_entitlement_uses.times ELSE 0 END + 1, \
             used_on = CURRENT_DATE",
        )
        .bind::<Integer, _>(character_id)
        .bind::<diesel::sql_types::SmallInt, _>(index)
        .execute(conn)?;
        Ok(Spending::Spent)
    })?)
}

/// Give back `cost` Rep and today's use of entitlement `index`, which
/// came to nothing.
pub fn refund(character_id: i32, index: i16, cost: i32) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> QueryResult<_> {
        diesel::update(family_members::table.find(character_id))
            .set(family_members::reputation.eq(family_members::reputation + cost))
            .execute(conn)?;
        diesel::update(
            family_entitlement_uses::table
                .find((character_id, index))
                .filter(family_entitlement_uses::used_on.eq(current_date()))
                .filter(family_entitlement_uses::times.gt(0)),
        )
        .set(family_entitlement_uses::times.eq(family_entitlement_uses::times - 1))
        .execute(conn)?;
        Ok(())
    })?;
    Ok(())
}

/// The rate entitlement working for the character, if one still is.
pub fn buff(conn: &mut PgConnection, character_id: i32) -> QueryResult<Option<FamilyBuff>> {
    let row = family_buffs::table
        .find(character_id)
        .filter(family_buffs::until.gt(SystemTime::now()))
        .select((family_buffs::entitlement, family_buffs::until))
        .first::<(i16, SystemTime)>(conn)
        .optional()?;
    Ok(row.and_then(|(index, until)| {
        let index = usize::try_from(index).ok()?;
        ENTITLEMENTS.get(index)?;
        Some(FamilyBuff { index, until })
    }))
}

/// `buff` works for each of `ids` now, in place of any they had.
pub fn set_buff(ids: &[i32], buff: FamilyBuff) -> crate::Result<()> {
    use diesel::upsert::excluded;
    let rows: Vec<_> = ids
        .iter()
        .map(|&id| {
            (
                family_buffs::character_id.eq(id),
                family_buffs::entitlement.eq(buff.index as i16),
                family_buffs::until.eq(buff.until),
            )
        })
        .collect();
    diesel::insert_into(family_buffs::table)
        .values(&rows)
        .on_conflict(family_buffs::character_id)
        .do_update()
        .set((
            family_buffs::entitlement.eq(excluded(family_buffs::entitlement)),
            family_buffs::until.eq(excluded(family_buffs::until)),
        ))
        .execute(&mut crate::connection()?)?;
    Ok(())
}

/// The character's buff that lasted until `until` is over; one that took
/// its place stays.
pub fn end_buff(character_id: i32, until: SystemTime) -> crate::Result<()> {
    diesel::delete(
        family_buffs::table
            .find(character_id)
            .filter(family_buffs::until.le(until)),
    )
    .execute(&mut crate::connection()?)?;
    Ok(())
}

/// The character's juniors and their juniors: the six places below them
/// on the Family Tree.
pub fn tree_juniors(character_id: i32) -> crate::Result<Vec<i32>> {
    let mut conn = crate::connection()?;
    let juniors = juniors(&mut conn, character_id)?;
    let mut all = juniors.clone();
    for junior in juniors {
        all.extend(self::juniors(&mut conn, junior)?);
    }
    Ok(all)
}

/// Whether `a` and `b` are in one family: under the same leader.
pub fn same_family(a: i32, b: i32) -> crate::Result<bool> {
    let mut conn = crate::connection()?;
    let leader = |conn: &mut PgConnection, id: i32| -> QueryResult<Option<i32>> {
        let seniors = seniors(conn, id)?;
        if seniors.is_empty() && juniors(conn, id)?.is_empty() {
            return Ok(None);
        }
        Ok(Some(seniors.last().copied().unwrap_or(id)))
    };
    let leader_a = leader(&mut conn, a)?;
    Ok(leader_a.is_some() && leader_a == leader(&mut conn, b)?)
}

/// How parting a senior from a junior went.
#[derive(Debug, PartialEq, Eq)]
pub enum Separation {
    /// They aren't senior and junior (any more).
    NotRelated,
    /// The asker hasn't the fee.
    Short { fee: i32 },
    /// Parted, for `fee`. `ids` is the family as it was, whose windows all
    /// change.
    Parted { fee: i32, ids: Vec<i32> },
}

/// Part `junior`, with their branch, from `senior`, for the asker, who has
/// `meso`. Levels are those online now where given, else the saved ones.
/// The senior loses twice `separation_rep` and their senior once; the
/// junior leads a family of their own, with no precepts yet (report b048).
pub fn separate(
    senior: i32,
    junior: i32,
    levels: (Option<u8>, Option<u8>),
    meso: i32,
) -> crate::Result<Separation> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        // The senior's senior loses Rep too: lock all three in id order, so
        // as not to deadlock with a separation or an add one level up.
        let above = senior_of(conn, senior)?;
        let mut rows: Vec<i32> = [senior, junior].into_iter().chain(above).collect();
        rows.sort_unstable();
        family_members::table
            .filter(family_members::character_id.eq_any(rows))
            .order(family_members::character_id)
            .select(family_members::character_id)
            .for_update()
            .load::<i32>(conn)?;
        if senior_of(conn, junior)? != Some(senior) || senior_of(conn, senior)? != above {
            return Ok(Separation::NotRelated);
        }
        let mut level = |id: i32, known: Option<u8>| -> QueryResult<u8> {
            match known {
                Some(level) => Ok(level),
                None => characters::table
                    .find(id)
                    .select(characters::level)
                    .first::<i16>(conn)
                    .map(|level| level.clamp(0, 255) as u8),
            }
        };
        let (senior_level, junior_level) = (level(senior, levels.0)?, level(junior, levels.1)?);
        let fee = separation_fee(senior_level, junior_level);
        if meso < fee {
            return Ok(Separation::Short { fee });
        }
        let seniors = seniors(conn, senior)?;
        let leader = seniors.last().copied().unwrap_or(senior);
        let (ids, _) = descendants(conn, leader)?;

        let rep = separation_rep(junior_level);
        let losses = std::iter::once((senior, 2 * rep)).chain(above.map(|s| (s, rep)));
        for (id, loss) in losses {
            let reputation = family_members::table
                .find(id)
                .select(family_members::reputation)
                .first::<i32>(conn)?;
            diesel::update(family_members::table.find(id))
                .set(family_members::reputation.eq((reputation - loss).max(0)))
                .execute(conn)?;
        }
        diesel::update(family_members::table.find(junior))
            .set((
                family_members::senior_id.eq(None::<i32>),
                family_members::senior_reputation.eq(0),
                family_members::precepts.eq(""),
            ))
            .execute(conn)?;
        Ok(Separation::Parted { fee, ids })
    })?)
}
