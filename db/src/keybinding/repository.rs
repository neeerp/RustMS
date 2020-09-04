use super::{Keybinding, NewKeybinding};
use crate::establish_connection;
use crate::schema::keybindings::dsl::*;
use diesel::expression_methods::*;
use diesel::pg::upsert::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl};

pub fn get_keybindings_by_characterid(c_id: i32) -> QueryResult<Vec<Keybinding>> {
    let connection = establish_connection();

    keybindings
        .filter(character_id.eq(c_id))
        .load::<Keybinding>(&connection)
}

// TODO: We're going to have conflicts in two ways; need to address this...
// TODO: Manual SQL maybe?
pub fn upsert_keybindings(bindings: Vec<NewKeybinding>) -> QueryResult<Vec<Keybinding>> {
    let connection = establish_connection();

    diesel::insert_into(keybindings)
        .values(&bindings)
        .on_conflict(on_constraint("key_is_unique_per_character"))
        .do_update()
        .set(key.eq(excluded(key)))
        .get_results(&connection)
}
