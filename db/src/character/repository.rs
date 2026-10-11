use super::{Character, NewCharacter};
use crate::schema::characters;
use crate::schema::characters::dsl::*;
use diesel::dsl::exists;
use diesel::expression_methods::*;
use diesel::pg::PgConnection;
use diesel::sql_types::Text;
use diesel::{select, QueryDsl, QueryResult, RunQueryDsl};

diesel::define_sql_function!(fn lower(x: Text) -> Text);

/// Look up a character by name, ignoring case.
pub fn get_character_by_name(cname: &str) -> crate::Result<Character> {
    let mut connection = crate::connection()?;
    Ok(characters
        .filter(lower(name).eq(lower(cname)))
        .first::<Character>(&mut connection)?)
}

/// Whether `character_id` belongs to `account_id`.
pub fn character_belongs_to(character_id: i32, account_id: i32) -> crate::Result<bool> {
    let mut connection = crate::connection()?;
    Ok(select(exists(
        characters
            .filter(id.eq(character_id))
            .filter(accountid.eq(account_id)),
    ))
    .get_result(&mut connection)?)
}

pub(crate) fn insert_character(
    connection: &mut PgConnection,
    new_character: &NewCharacter,
) -> QueryResult<Character> {
    diesel::insert_into(characters::table)
        .values(new_character)
        .get_result::<Character>(connection)
}

pub fn delete_character(character_id: i32, account_id: i32) -> crate::Result<usize> {
    let mut connection = crate::connection()?;
    Ok(diesel::delete(
        characters
            .filter(id.eq(character_id))
            .filter(accountid.eq(account_id)),
    )
    .execute(&mut connection)?)
}
