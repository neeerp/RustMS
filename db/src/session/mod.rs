use crate::schema::sessions;
use ipnetwork::IpNetwork;
use std::time::SystemTime;

pub mod repository;
pub use repository::*;

/// Where a session is in the login → world flow.
///
/// `BeforeLogin`/`AfterLogin` belong to a live login-server connection,
/// `Transition` is a short-lived, single-use ticket to enter one channel, and
/// `InGame` belongs to a live world connection. A row exists only while its
/// owning connection does (plus the brief transition window), so the
/// sessions table is the source of truth for whether an account is online.
#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::SessionState"]
pub enum SessionState {
    BeforeLogin,
    AfterLogin,
    Transition,
    InGame,
}

#[derive(Clone, Debug, Identifiable, Queryable, AsChangeset)]
#[diesel(table_name = sessions)]
pub struct Session {
    pub id: i32,
    pub account_id: i32,
    pub character_id: Option<i32>,
    pub ip: IpNetwork,
    pub hwid: String,
    pub state: SessionState,
    pub updated_at: SystemTime,
    pub created_at: SystemTime,
    pub selected_world_id: Option<i16>,
    pub selected_channel_id: Option<i16>,
}

/// Session creation projection.
#[derive(Insertable)]
#[diesel(table_name = sessions)]
pub struct NewSession<'a> {
    pub account_id: i32,
    pub ip: IpNetwork,
    pub hwid: &'a str,
    pub state: SessionState,
}
