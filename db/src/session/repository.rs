use super::{NewSession, Session, SessionState};
use crate::establish_connection;
use crate::schema::sessions::dsl::*;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SaveChangesDsl};
use ipnetwork::IpNetwork;

pub fn get_session_by_accountid(a_id: i32) -> QueryResult<Session> {
    let mut connection = establish_connection();

    sessions
        .filter(account_id.eq(a_id))
        .first::<Session>(&mut connection)
}

pub fn get_session_by_characterid(c_id: i32) -> QueryResult<Session> {
    let mut connection = establish_connection();

    sessions
        .filter(character_id.eq(c_id))
        .first::<Session>(&mut connection)
}

pub fn get_session_to_reattach(c_id: i32, ip_addr: IpNetwork) -> QueryResult<Session> {
    let mut connection = establish_connection();

    sessions
        .filter(character_id.eq(c_id))
        .filter(ip.eq(ip_addr))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&mut connection)
}

pub fn get_transition_session_by_character_id(c_id: i32) -> QueryResult<Session> {
    let mut connection = establish_connection();

    sessions
        .filter(character_id.eq(c_id))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&mut connection)
}

pub fn create_session(new_session: NewSession) -> QueryResult<Session> {
    let mut connection = establish_connection();

    diesel::insert_into(sessions)
        .values(&new_session)
        .get_result::<Session>(&mut connection)
}

pub fn update_session(ses: &Session) -> QueryResult<Session> {
    let mut connection = establish_connection();

    ses.save_changes(&mut connection)
}

pub fn delete_session_by_id(s_id: i32) -> QueryResult<usize> {
    let mut connection = establish_connection();

    diesel::delete(sessions.filter(id.eq(s_id))).execute(&mut connection)
}

/// Alias for get_session_by_characterid
pub fn get_session_by_character_id(c_id: i32) -> QueryResult<Session> {
    get_session_by_characterid(c_id)
}

/// Update the character_id for a session
pub fn update_session_character(s_id: i32, c_id: i32) -> QueryResult<usize> {
    let mut connection = establish_connection();

    diesel::update(sessions.filter(id.eq(s_id)))
        .set(character_id.eq(Some(c_id)))
        .execute(&mut connection)
}

pub fn update_session_selection(s_id: i32, w_id: i16, ch_id: i16) -> QueryResult<usize> {
    let mut connection = establish_connection();

    diesel::update(sessions.filter(id.eq(s_id)))
        .set((
            selected_world_id.eq(Some(w_id)),
            selected_channel_id.eq(Some(ch_id)),
        ))
        .execute(&mut connection)
}
