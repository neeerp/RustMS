//! Pets, kept by the cash serial of their item so they keep their name and
//! closeness in and out of the Cash Inventory.

use crate::schema::pets;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;
use game::{Inventory, InventoryType, PetData};
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Queryable, Insertable)]
#[diesel(table_name = pets)]
struct PetRow {
    serial: i64,
    name: String,
    level: i16,
    closeness: i16,
    fullness: i16,
    skill: i32,
    attribute: i16,
    summoned: Option<i16>,
}

impl PetRow {
    fn data(self) -> PetData {
        PetData {
            name: self.name,
            level: u8::try_from(self.level).unwrap_or(1),
            closeness: self.closeness,
            fullness: u8::try_from(self.fullness).unwrap_or(0),
            skill: u16::try_from(self.skill).unwrap_or(0),
            attribute: self.attribute,
        }
    }
}

/// The pet with cash serial `serial`, if it has been out of the Cash
/// Inventory before.
pub fn find(serial: i64) -> crate::Result<Option<PetData>> {
    let mut conn = crate::connection()?;
    Ok(pets::table
        .find(serial)
        .first::<PetRow>(&mut conn)
        .optional()?
        .map(PetRow::data))
}

/// The pets with these serials.
pub(crate) fn load(conn: &mut PgConnection, serials: &[i64]) -> QueryResult<HashMap<i64, PetData>> {
    Ok(pets::table
        .filter(pets::serial.eq_any(serials))
        .load::<PetRow>(conn)?
        .into_iter()
        .map(|row| (row.serial, row.data()))
        .collect())
}

/// The serials of the pets in `inventory` that were out, in list order.
pub(crate) fn summoned(conn: &mut PgConnection, inventory: &Inventory) -> QueryResult<Vec<i64>> {
    let serials: Vec<i64> = inventory
        .items(InventoryType::Cash)
        .filter(|(_, item)| item.pet.is_some())
        .filter_map(|(_, item)| item.cash_serial)
        .collect();
    let mut out: Vec<(i64, i16)> = pets::table
        .filter(pets::serial.eq_any(&serials))
        .filter(pets::summoned.is_not_null())
        .select((pets::serial, pets::summoned.assume_not_null()))
        .load(conn)?;
    out.sort_by_key(|&(_, place)| place);
    Ok(out
        .into_iter()
        .map(|(serial, _)| serial)
        .take(game::pet::MAX_PETS)
        .collect())
}

/// Write the pets in `inventory`, with their places among `summoned`.
pub(crate) fn save(
    conn: &mut PgConnection,
    inventory: &Inventory,
    summoned: &[i64],
) -> QueryResult<()> {
    let rows: Vec<PetRow> = inventory
        .items(InventoryType::Cash)
        .filter_map(|(_, item)| Some((item.cash_serial?, item.pet.as_ref()?)))
        .map(|(serial, pet)| PetRow {
            serial,
            name: pet.name.clone(),
            level: i16::from(pet.level),
            closeness: pet.closeness,
            fullness: i16::from(pet.fullness),
            skill: i32::from(pet.skill),
            attribute: pet.attribute,
            summoned: summoned
                .iter()
                .position(|&out| out == serial)
                .map(|place| place as i16),
        })
        .collect();
    if rows.is_empty() {
        return Ok(());
    }
    diesel::insert_into(pets::table)
        .values(&rows)
        .on_conflict(pets::serial)
        .do_update()
        .set((
            pets::name.eq(excluded(pets::name)),
            pets::level.eq(excluded(pets::level)),
            pets::closeness.eq(excluded(pets::closeness)),
            pets::fullness.eq(excluded(pets::fullness)),
            pets::skill.eq(excluded(pets::skill)),
            pets::attribute.eq(excluded(pets::attribute)),
            pets::summoned.eq(excluded(pets::summoned)),
        ))
        .execute(conn)?;
    Ok(())
}
