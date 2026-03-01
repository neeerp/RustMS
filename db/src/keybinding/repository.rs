use super::{KeybindDTO, Keybinding};
use crate::establish_connection;
use crate::schema::keybindings::dsl::*;
use diesel::expression_methods::*;
use diesel::pg::upsert::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl};

pub fn get_keybindings_by_characterid(c_id: i32) -> QueryResult<Vec<Keybinding>> {
    let mut connection = establish_connection();

    keybindings
        .filter(character_id.eq(c_id))
        .load::<Keybinding>(&mut connection)
}

pub fn upsert_keybindings(bindings: Vec<KeybindDTO>) -> QueryResult<Vec<Keybinding>> {
    let mut connection = establish_connection();

    diesel::insert_into(keybindings)
        .values(bindings)
        .on_conflict(on_constraint("key_is_unique_per_character"))
        .do_update()
        .set(key.eq(excluded(key)))
        .get_results(&mut connection)
}
