use crate::schema::keybindings;
use diesel::QueryResult;
use itertools::izip;
use std::collections::HashMap;

mod repository;
pub use repository::*;

#[derive(Debug, DbEnum)]
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
        .map(|(key, btype_ord, action)| NewKeybinding {
            character_id: c_id,
            key: key,
            bind_type: btype_ord.into(),
            action: action,
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

const DEFAULT_KEY: [i16; 26] = [
    2, 3, 4, 5, 31, 56, 59, 32, 42, 6, 17, 29, 30, 41, 50, 60, 61, 62, 63, 64, 65, 16, 7, 9, 13, 8,
];
const DEFAULT_TYPE: [u8; 26] = [
    4, 4, 4, 4, 5, 5, 6, 5, 5, 4, 4, 4, 5, 4, 4, 6, 6, 6, 6, 6, 6, 4, 4, 4, 4, 4,
];
const DEFAULT_ACTION: [i16; 26] = [
    1, 0, 3, 2, 53, 54, 100, 52, 51, 19, 5, 9, 50, 7, 22, 101, 102, 103, 104, 105, 106, 8, 17, 26,
    20, 4,
];
