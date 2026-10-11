use super::{Character, NewCharacter};
use crate::schema;
use crate::schema::characters;
use diesel::expression_methods::*;
use diesel::pg::PgConnection;
use diesel::sql_types::Text;
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SaveChangesDsl};
use schema::characters::dsl::*;

diesel::define_sql_function!(fn lower(x: Text) -> Text);

pub fn get_characters_by_accountid(account_id: i32) -> crate::Result<Vec<Character>> {
    let mut connection = crate::connection()?;

    Ok(characters
        .filter(accountid.eq(account_id))
        .load::<Character>(&mut connection)?)
}

/// Look up a character by name, ignoring case.
pub fn get_character_by_name(cname: &str) -> crate::Result<Character> {
    let mut connection = crate::connection()?;

    Ok(characters
        .filter(lower(name).eq(lower(cname)))
        .first::<Character>(&mut connection)?)
}

pub fn get_character_by_id(cid: i32) -> crate::Result<Character> {
    let mut connection = crate::connection()?;

    Ok(characters
        .filter(id.eq(cid))
        .first::<Character>(&mut connection)?)
}

pub(crate) fn insert_character(
    connection: &mut PgConnection,
    new_character: &NewCharacter,
) -> QueryResult<Character> {
    diesel::insert_into(characters::table)
        .values(new_character)
        .get_result::<Character>(connection)
}

pub fn update_character(character: &Character) -> crate::Result<Character> {
    let mut connection = crate::connection()?;

    Ok(character.save_changes(&mut connection)?)
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
