use super::{NewSession, Session, SessionState};
use crate::establish_connection;
use crate::schema::sessions::dsl::*;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl};
use ipnetwork::IpNetwork;

pub fn get_session_by_accountid(a_id: i32) -> QueryResult<Session> {
    let connection = establish_connection();

    sessions
        .filter(account_id.eq(a_id))
        .first::<Session>(&connection)
}

pub fn get_session_by_characterid(c_id: i32) -> QueryResult<Session> {
    let connection = establish_connection();

    sessions
        .filter(character_id.eq(c_id))
        .first::<Session>(&connection)
}

pub fn create_session<'a>(
    a_id: i32,
    hardware_id: &'a str,
    ip_addr: IpNetwork,
    session_state: SessionState,
) -> QueryResult<Session> {
    let connection = establish_connection();

    let new_session = NewSession {
        account_id: a_id,
        hwid: hardware_id,
        ip: ip_addr,
        state: session_state,
    };

    diesel::insert_into(sessions)
        .values(&new_session)
        .get_result::<Session>(&connection)
}

pub fn delete_session_by_id(s_id: i32) -> QueryResult<usize> {
    let connection = establish_connection();

    diesel::delete(sessions.filter(id.eq(s_id))).execute(&connection)
}
