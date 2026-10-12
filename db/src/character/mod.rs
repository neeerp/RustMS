use crate::schema::characters;
use diesel::Connection;
use game::{AvatarLook, CharacterStats, InventoryType, Item, Keymap};
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
    pub sp: i16,
    pub spawn_point: i16,
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
    pub map_id: i32,
}

impl NewCharacter<'_> {
    /// Save the new character with its default keymap and starting items.
    ///
    /// Everything is written in one transaction so a failure cannot leave a
    /// half-created character behind. Returns what the character list needs.
    pub fn create(
        self,
        starting_items: Vec<(InventoryType, i16, Item)>,
    ) -> crate::Result<(CharacterStats, AvatarLook)> {
        let mut connection = crate::connection()?;
        connection
            .transaction(|connection| -> diesel::QueryResult<_> {
                let character = repository::insert_character(connection, &self)?;
                crate::player::save_keymap(connection, character.id, &Keymap::defaults())?;
                let stats = crate::player::character_stats(&character);
                let look = AvatarLook::from_equipped(
                    &stats,
                    starting_items
                        .iter()
                        .filter(|(kind, _, _)| *kind == InventoryType::Equipped)
                        .map(|(_, slot, item)| (*slot, item.item_id)),
                );
                crate::player::insert_items(connection, character.id, starting_items)?;
                Ok((stats, look))
            })
            .map_err(Into::into)
    }
}
