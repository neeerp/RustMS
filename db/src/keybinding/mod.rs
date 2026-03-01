use crate::{character::Character, schema::keybindings};
use diesel::QueryResult;
use itertools::izip;
use std::collections::HashMap;

mod repository;
pub use repository::*;

// Values to build default bindings from
// TODO: These aren't a complete default set.
// TODO: Perhaps we should externalize these.
const DEFAULT_KEY: [i16; 23] = [
    59, 60, 61, 62, 63, 64, 65, 56, 87, 18, 23, 31, 37, 19, 17, 46, 50, 16, 43, 40, 21, 4, 84,
];
const DEFAULT_TYPE: [u8; 23] = [
    6, 6, 6, 6, 6, 6, 6, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
];
const DEFAULT_ACTION: [i16; 23] = [
    100, 101, 102, 103, 104, 105, 106, 54, 54, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15,
];

#[derive(Debug, Clone, Copy, DbEnum)]
#[DieselType = "Keybind_type"]
#[PgType = "keybind_type"]
pub enum KeybindType {
    Nil,
    Skill,
    Item,
    Cash,
    Menu,
    Action,
    Face,
    Macro,
    Text,
}

impl From<KeybindType> for u8 {
    fn from(kind: KeybindType) -> Self {
        match kind {
            KeybindType::Nil => 0,
            KeybindType::Skill => 1,
            KeybindType::Item => 2,
            KeybindType::Cash => 3,
            KeybindType::Menu => 4,
            KeybindType::Action => 5,
            KeybindType::Face => 6,
            KeybindType::Macro => 7,
            KeybindType::Text => 8,
        }
    }
}

impl From<u8> for KeybindType {
    fn from(kind: u8) -> Self {
        match kind {
            1 => KeybindType::Skill,
            2 => KeybindType::Item,
            3 => KeybindType::Cash,
            4 => KeybindType::Menu,
            5 => KeybindType::Action,
            6 => KeybindType::Face,
            7 => KeybindType::Macro,
            8 => KeybindType::Text,
            _ => KeybindType::Nil,
        }
    }
}

/// Keybinding database entity.
#[derive(Identifiable, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = keybindings)]
pub struct Keybinding {
    pub id: i32,
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

/// A projection of the Keybinding entity for less cumbersome manipulation.
#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = keybindings)]
pub struct KeybindDTO {
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

impl KeybindDTO {
    /// Convert a Keybinding entity struct into its DTO projection.
    pub fn from(character_id: i32, key: i16, bind_type: KeybindType, action: i16) -> Self {
        KeybindDTO {
            character_id,
            key,
            bind_type,
            action,
        }
    }

    /// Create an empty default keybind for the given key, for the given character.
    pub fn default(key: i16, character_id: i32) -> Self {
        KeybindDTO {
            character_id,
            key,
            bind_type: KeybindType::Nil,
            action: 0,
        }
    }
}

impl From<&Keybinding> for KeybindDTO {
    fn from(k: &Keybinding) -> Self {
        KeybindDTO {
            character_id: k.character_id,
            key: k.key,
            bind_type: k.bind_type,
            action: k.action,
        }
    }
}

/// A set of keybinds for a given character.
pub struct KeybindSet {
    binds: HashMap<i16, KeybindDTO>,
    character_id: i32,
}

impl KeybindSet {
    /// Get the keybind set for the given character.
    pub fn from_character(character: &Character) -> QueryResult<Self> {
        let character_id = character.id;

        Ok(Self::from_bind_vec(
            character_id,
            repository::get_keybindings_by_characterid(character_id)?,
        ))
    }

    /// Create, set, and return a default keybind set for the given character.
    pub fn set_defaults(character: &Character) -> QueryResult<Self> {
        let character_id = character.id;
        let mut bind_set = Self {
            character_id,
            binds: HashMap::new(),
        };

        izip!(
            DEFAULT_KEY.to_vec(),
            DEFAULT_TYPE.to_vec(),
            DEFAULT_ACTION.to_vec()
        )
        .for_each(|(key, btype_ord, action)| {
            bind_set.set(KeybindDTO::from(
                character_id,
                key,
                btype_ord.into(),
                action,
            ))
        });

        bind_set.save()?;

        Ok(bind_set)
    }

    /// Convert a vector of keybindings to a keybind set for a given character.
    pub fn from_bind_vec(character_id: i32, bind_vec: Vec<Keybinding>) -> Self {
        let mut bind_set = Self {
            character_id,
            binds: HashMap::new(),
        };

        bind_vec
            .iter()
            .for_each(|bind: &Keybinding| bind_set.set(bind.into()));

        bind_set
    }

    /// Get the specified key, or a default if it is not present in the keybind set.
    pub fn get(&mut self, key: i16) -> KeybindDTO {
        self.binds
            .get(&key)
            .map_or(KeybindDTO::default(key, self.character_id), |bind_ref| {
                bind_ref.clone()
            })
    }

    /// Set/replace the given key in the keybind set.
    pub fn set(&mut self, bind: KeybindDTO) {
        self.binds.insert(bind.key, bind);
    }

    /// Save the current state of the keybind set.
    pub fn save(&self) -> QueryResult<()> {
        repository::upsert_keybindings(self.binds.values().map(|x| x.clone()).collect())?;
        Ok(())
    }
}
