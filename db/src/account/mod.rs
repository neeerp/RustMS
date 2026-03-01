use crate::schema::accounts;
use std::{fmt::Debug, time::SystemTime};

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
    pub accepted_tos: bool,
    pub banned: bool,
    pub ban_msg: Option<String>,
}

impl Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id: {}, user_name: {}, logged_in: {}",
            self.id, self.user_name, self.logged_in
        )
    }
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub struct NewAccount<'a> {
    pub user_name: &'a str,
    pub password: &'a str,
}
