use crate::schema::accounts;
use std::time::SystemTime;

mod repository;
pub use repository::*;

#[derive(Identifiable, Queryable, AsChangeset)]
pub struct Account {
    pub id: i32,
    pub user_name: String,
    pub password: String,
    pub pin: String,
    pub pic: String,
    pub logged_in: bool,
    pub last_login_at: Option<SystemTime>,
    pub created_at: SystemTime,
    pub character_slots: i16,
    pub gender: i16,
    pub banned: bool,
    pub ban_msg: Option<String>,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount<'a> {
    pub user_name: &'a str,
    pub password: &'a str,
}

#[derive(AsChangeset)]
#[table_name = "accounts"]
pub struct AccountLoginUpdate {
    pub logged_in: bool,
    pub last_login_at: SystemTime,
}
