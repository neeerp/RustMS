use crate::schema::keybindings;

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

#[derive(Identifiable, Queryable, Insertable, AsChangeset)]
#[table_name = "keybindings"]
pub struct Keybinding {
    pub id: i32,
    pub character_id: i32,
    pub key: i16,
    pub bind_type: KeybindType,
    pub action: i16,
}

#[derive(AsChangeset)]
#[table_name = "keybindings"]
pub struct KeybindingChange {
    action: i16,
    key: i16,
}
