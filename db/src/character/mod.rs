use crate::schema::characters;
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
