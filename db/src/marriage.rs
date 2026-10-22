//! Engaged and married couples (report b052): one row per couple, while it
//! lasts.

use crate::schema::{characters, marriages};
use diesel::prelude::*;
use game::marriage::{MarriageRecord, Refusal, ENGAGED};

diesel::define_sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

type Row = (i32, i32, i32, i16, i32);

fn columns() -> (
    marriages::id,
    marriages::groom_id,
    marriages::bride_id,
    marriages::status,
    marriages::item_id,
) {
    (
        marriages::id,
        marriages::groom_id,
        marriages::bride_id,
        marriages::status,
        marriages::item_id,
    )
}

/// The couple's record, with both names.
fn with_names(conn: &mut PgConnection, row: Row) -> QueryResult<MarriageRecord> {
    let (id, groom_id, bride_id, status, item_id) = row;
    let names: Vec<(i32, String)> = characters::table
        .filter(characters::id.eq_any([groom_id, bride_id]))
        .select((characters::id, characters::name))
        .load(conn)?;
    let name = |cid| {
        names
            .iter()
            .find(|(id, _)| *id == cid)
            .map(|(_, name)| name.clone())
            .unwrap_or_default()
    };
    Ok(MarriageRecord {
        id,
        groom_id,
        bride_id,
        status,
        item_id,
        groom_name: name(groom_id),
        bride_name: name(bride_id),
    })
}

fn row_of(conn: &mut PgConnection, character_id: i32) -> QueryResult<Option<Row>> {
    marriages::table
        .filter(
            marriages::groom_id
                .eq(character_id)
                .or(marriages::bride_id.eq(character_id)),
        )
        .select(columns())
        .first(conn)
        .optional()
}

/// The character's engagement or marriage, if any.
pub fn record(conn: &mut PgConnection, character_id: i32) -> QueryResult<Option<MarriageRecord>> {
    row_of(conn, character_id)?
        .map(|row| with_names(conn, row))
        .transpose()
}

/// Engage `groom` and `bride` with the ring box `item_id`; refused in the
/// words of `proposer` if either is in a couple already.
pub fn engage(
    proposer: i32,
    groom: i32,
    bride: i32,
    item_id: i32,
) -> crate::Result<Result<MarriageRecord, Refusal>> {
    let mut conn = crate::connection()?;
    Ok(conn.transaction(|conn| -> QueryResult<_> {
        for cid in [groom, bride] {
            if let Some(row) = row_of(conn, cid)? {
                let taken = with_names(conn, row)?;
                return Ok(Err(Refusal::taken(&taken, cid != proposer)));
            }
        }
        let row = diesel::insert_into(marriages::table)
            .values((
                marriages::groom_id.eq(groom),
                marriages::bride_id.eq(bride),
                marriages::status.eq(ENGAGED),
                marriages::item_id.eq(item_id),
            ))
            .returning(columns())
            .get_result(conn)?;
        Ok(Ok(with_names(conn, row)?))
    })?)
}

/// The one a proposal names: their id, gender, and any couple they are in.
pub struct Partner {
    pub id: i32,
    pub gender: u8,
    pub record: Option<MarriageRecord>,
}

/// The character named `name`, in any case, to propose to.
pub fn partner(name: &str) -> crate::Result<Option<Partner>> {
    let mut conn = crate::connection()?;
    let Some((id, gender)) = characters::table
        .filter(lower(characters::name).eq(lower(name)))
        .select((characters::id, characters::gender))
        .first::<(i32, i16)>(&mut conn)
        .optional()?
    else {
        return Ok(None);
    };
    Ok(Some(Partner {
        id,
        gender: gender as u8,
        record: record(&mut conn, id)?,
    }))
}
