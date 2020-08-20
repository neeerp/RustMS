use super::{Character, NewCharacter};
use crate::establish_connection;
use crate::schema;
use crate::schema::characters;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl};
use schema::characters::dsl::*;

pub fn get_characters_by_accountid(account_id: i32) -> Option<Vec<Character>> {
    let connection = establish_connection();

    characters
        .filter(accountid.eq(account_id))
        .load::<Character>(&connection)
        .ok()
}

pub fn create_character<'a>(char: NewCharacter) -> Option<Character> {
    let connection = establish_connection();

    diesel::insert_into(characters::table)
        .values(&char)
        .get_result::<Character>(&connection)
        .ok()
}

pub fn delete_character(character_id: i32, account_id: i32) -> QueryResult<usize> {
    let connection = establish_connection();

    diesel::delete(
        characters
            .filter(id.eq(character_id))
            .filter(accountid.eq(account_id)),
    )
    .execute(&connection)
}
