use super::{Character, NewCharacter};
use crate::establish_connection;
use crate::schema;
use crate::schema::characters;
use diesel::expression_methods::*;
use diesel::{QueryDsl, RunQueryDsl};
use schema::characters::dsl::*;

pub fn get_characters_by_accountid(query_id: i32) -> Option<Vec<Character>> {
    let connection = establish_connection();

    characters
        .filter(accountid.eq(query_id))
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
