use crate::{
    keybinding::{self, Keybinding},
    schema::characters,
};
use diesel::QueryResult;
use keybinding::NewKeybinding;
use std::{collections::HashMap, time::SystemTime};

pub mod repository;

pub use repository::*;

#[derive(Identifiable, Queryable)]
pub struct Character {
    pub id: i32,
    pub accountid: i32,
    pub world: i16,
    pub name: String,

    pub level: i16,
    pub exp: i32,

    pub stre: i16,
    pub dex: i16,
    pub luk: i16,
    pub int: i16,
    pub hp: i16,
    pub mp: i16,
    pub maxhp: i16,
    pub maxmp: i16,
    pub ap: i16,
    pub fame: i16,

    pub meso: i32,

    pub job: i16,

    pub face: i32,
    pub hair: i32,
    pub hair_color: i32,
    pub skin: i32,
    pub gender: i16,

    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[table_name = "characters"]
pub struct NewCharacter<'a> {
    pub accountid: i32,
    pub world: i16,
    pub name: &'a str,
    pub job: i16,
    pub face: i32,
    pub hair: i32,
    pub hair_color: i32,
    pub skin: i32,
    pub gender: i16,
}

impl Character {
    pub fn new(new_character: NewCharacter) -> QueryResult<Self> {
        let new_character = repository::create_character(new_character)?;
        Keybinding::set_default_bindings(new_character.id)?;

        Ok(new_character)
    }

    pub fn get_binds(&self) -> QueryResult<HashMap<i16, Keybinding>> {
        Ok(Keybinding::vec_to_map(
            keybinding::get_keybindings_by_characterid(self.id)?,
        ))
    }

    pub fn upsert_binds(&self, new_binds: Vec<NewKeybinding>) -> QueryResult<Vec<Keybinding>> {
        keybinding::upsert_keybindings(new_binds)
    }
}

impl NewCharacter<'_> {
    pub fn create(self) -> QueryResult<Character> {
        Character::new(self)
    }
}
