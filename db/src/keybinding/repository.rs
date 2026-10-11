use super::{KeybindDTO, Keybinding};
use crate::schema::keybindings::dsl::*;
use diesel::expression_methods::*;
use diesel::pg::upsert::*;
use diesel::pg::PgConnection;
use diesel::{QueryDsl, QueryResult, RunQueryDsl};

pub fn get_keybindings_by_characterid(c_id: i32) -> crate::Result<Vec<Keybinding>> {
    let mut connection = crate::connection()?;

    Ok(keybindings
        .filter(character_id.eq(c_id))
        .load::<Keybinding>(&mut connection)?)
}

pub fn upsert_keybindings(bindings: Vec<KeybindDTO>) -> crate::Result<Vec<Keybinding>> {
    let mut connection = crate::connection()?;
    Ok(upsert_keybindings_on(&mut connection, bindings)?)
}

pub(crate) fn upsert_keybindings_on(
    connection: &mut PgConnection,
    bindings: Vec<KeybindDTO>,
) -> QueryResult<Vec<Keybinding>> {
    diesel::insert_into(keybindings)
        .values(bindings)
        .on_conflict(on_constraint("key_is_unique_per_character"))
        .do_update()
        .set((bind_type.eq(excluded(bind_type)), action.eq(excluded(action))))
        .get_results(connection)
}
