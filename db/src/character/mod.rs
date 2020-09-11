use crate::{
    keybinding::{self, Keybinding},
    schema::characters,
};
use diesel::QueryResult;
use keybinding::KeybindSet;
use std::time::SystemTime;

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
}

impl NewCharacter<'_> {
    pub fn create(self) -> QueryResult<Character> {
        Character::new(self)
    }
}

/// This struct is meant to hold data pertaining to a character
/// beyond simply the character itself.
pub struct CharacterDTO {
    pub character: Character,
    pub key_binds: KeybindSet,
}

impl CharacterDTO {
    pub fn from_character_id(character_id: i32) -> QueryResult<Self> {
        let character = repository::get_character_by_id(character_id)?;
        Self::from_character(character)
    }

    pub fn from_character(character: Character) -> QueryResult<Self> {
        let key_binds = KeybindSet::from_character(&character)?;

        let dto = Self {
            character,
            key_binds,
        };
        Ok(dto)
    }
}
