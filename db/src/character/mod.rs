use crate::{keybinding::KeybindSet, schema::characters};
use diesel::QueryResult;
use std::time::SystemTime;

pub mod repository;

pub use repository::*;

/// Character database entity.
#[derive(Identifiable, Queryable, AsChangeset)]
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

    pub map_id: i32,
}

impl Character {
    pub fn save(&self) -> QueryResult<Character> {
        repository::update_character(self)
    }
}

/// Character creation projection.
#[derive(Insertable)]
#[diesel(table_name = characters)]
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

impl NewCharacter<'_> {
    /// Save the new character and return the saved Character's
    /// wrapper.
    pub fn create(self) -> QueryResult<CharacterWrapper> {
        let character = repository::create_character(self)?;
        let key_binds = KeybindSet::set_defaults(&character)?;

        Ok(CharacterWrapper {
            character,
            key_binds,
        })
    }
}

/// This struct is meant to hold data pertaining to a character
/// beyond simply the character itself.
pub struct CharacterWrapper {
    pub character: Character,
    pub key_binds: KeybindSet,
}

impl CharacterWrapper {
    /// Load an existing character given their ID.
    pub fn from_character_id(character_id: i32) -> QueryResult<Self> {
        let character = repository::get_character_by_id(character_id)?;
        Self::from_character(character)
    }

    /// Wrap a character entity struct along with any additional information.
    pub fn from_character(character: Character) -> QueryResult<Self> {
        let key_binds = KeybindSet::from_character(&character)?;

        let dto = Self {
            character,
            key_binds,
        };
        Ok(dto)
    }
}
