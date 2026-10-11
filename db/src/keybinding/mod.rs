/// Persisted form of a key binding's kind. Keymaps themselves are loaded and
/// saved through `crate::player`.
#[derive(Debug, Clone, Copy, DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::KeybindType"]
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
