use crate::{character::Character, schema::keybindings};
use diesel::QueryResult;
use itertools::izip;
use std::collections::HashMap;

mod repository;
pub use repository::*;

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

impl KeybindType {
    pub fn ord(&self) -> u8 {
        match self {
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

#[derive(Identifiable, Queryable, Insertable, AsChangeset)]
#[table_name = "keybindings"]
pub struct Keybinding {
    pub id: i32,
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "keybindings"]
pub struct NewKeybinding {
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

impl Keybinding {
    pub fn set_default_bindings(c_id: i32) -> QueryResult<Vec<Keybinding>> {
        let new_binds = izip!(
            DEFAULT_KEY.to_vec(),
            DEFAULT_TYPE.to_vec(),
            DEFAULT_ACTION.to_vec()
        )
        .map(|(key, btype_ord, action)| KeybindDTO {
            character_id: c_id,
            key,
            bind_type: btype_ord.into(),
            action,
        })
        .collect();

        repository::upsert_keybindings(new_binds)
    }

    pub fn vec_to_map(bind_vec: Vec<Keybinding>) -> HashMap<i16, Keybinding> {
        let mut bind_map = HashMap::new();

        for bind in bind_vec {
            bind_map.insert(bind.key, bind);
        }

        bind_map
    }
}

#[derive(Clone, Insertable, AsChangeset)]
#[table_name = "keybindings"]
pub struct KeybindDTO {
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

impl KeybindDTO {
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

pub struct KeybindSet {
    binds: HashMap<i16, KeybindDTO>,
    character_id: i32,
}

impl KeybindSet {
    pub fn from_character(character: &Character) -> QueryResult<Self> {
        let character_id = character.id;

        let mut bind_set = Self {
            character_id,
            binds: HashMap::new(),
        };

        repository::get_keybindings_by_characterid(character_id)?
            .iter()
            .for_each(|bind: &Keybinding| bind_set.set(bind.into()));

        Ok(bind_set)
    }

    pub fn get(&mut self, key: i16) -> KeybindDTO {
        match self.binds.get(&key) {
            Some(bind_ref) => bind_ref.clone(),
            None => {
                self.binds
                    .insert(key, KeybindDTO::default(key, self.character_id));

                self.get(key)
            }
        }
    }

    pub fn set(&mut self, bind: KeybindDTO) {
        self.binds.insert(bind.key, bind);
    }

    pub fn save(&self) -> QueryResult<()> {
        repository::upsert_keybindings(self.binds.values().map(|x| x.clone()).collect())?;
        Ok(())
    }
}
