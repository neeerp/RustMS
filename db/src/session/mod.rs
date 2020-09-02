use crate::schema::sessions;

extern crate ipnetwork;
use ipnetwork::IpNetwork;

use std::time::SystemTime;

pub mod repository;
pub use repository::*;

#[derive(Debug, DbEnum)]
#[DieselType = "Session_state"]
#[PgType = "session_state"]
pub enum SessionState {
    BeforeLogin,
    AfterLogin,
    Transition,
    InGame,
}

#[derive(Identifiable, Queryable, AsChangeset)]
#[table_name = "sessions"]
pub struct Session {
    pub id: i32,
    pub account_id: i32,
    pub character_id: Option<i32>,
    pub ip: IpNetwork,
    pub hwid: String,
    pub state: SessionState,
    pub updated_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[table_name = "sessions"]
pub struct NewSession<'a> {
    pub account_id: i32,
    pub ip: IpNetwork,
    pub hwid: &'a str,
    pub state: SessionState,
}
