use super::{NewSession, Session, SessionState};
use crate::establish_connection;
use crate::schema::sessions::dsl::*;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SaveChangesDsl};
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

pub fn get_session_to_reattach(c_id: i32, ip_addr: IpNetwork) -> QueryResult<Session> {
    let connection = establish_connection();

    sessions
        .filter(character_id.eq(c_id))
        .filter(ip.eq(ip_addr))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&connection)
}

pub fn create_session(new_session: NewSession) -> QueryResult<Session> {
    let connection = establish_connection();

    diesel::insert_into(sessions)
        .values(&new_session)
        .get_result::<Session>(&connection)
}

pub fn update_session(ses: &Session) -> QueryResult<Session> {
    let connection = establish_connection();

    ses.save_changes(&connection)
}

pub fn delete_session_by_id(s_id: i32) -> QueryResult<usize> {
    let connection = establish_connection();

    diesel::delete(sessions.filter(id.eq(s_id))).execute(&connection)
}
