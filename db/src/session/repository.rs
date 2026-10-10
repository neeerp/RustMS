use super::{NewSession, Session, SessionState};
use crate::schema::sessions::dsl::*;
use diesel::expression_methods::*;
use diesel::{QueryDsl, RunQueryDsl, SaveChangesDsl};
use ipnetwork::IpNetwork;

pub fn get_session_by_accountid(a_id: i32) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(sessions
        .filter(account_id.eq(a_id))
        .first::<Session>(&mut connection)?)
}

pub fn get_session_by_characterid(c_id: i32) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(sessions
        .filter(character_id.eq(c_id))
        .first::<Session>(&mut connection)?)
}

pub fn get_session_to_reattach(c_id: i32, ip_addr: IpNetwork) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(sessions
        .filter(character_id.eq(c_id))
        .filter(ip.eq(ip_addr))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&mut connection)?)
}

pub fn get_transition_session_by_character_id(c_id: i32) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(sessions
        .filter(character_id.eq(c_id))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&mut connection)?)
}

pub fn get_transition_session_by_character_channel_ip(
    c_id: i32,
    ch_id: i16,
    ip_addr: IpNetwork,
) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(sessions
        .filter(character_id.eq(c_id))
        .filter(selected_channel_id.eq(Some(ch_id)))
        .filter(ip.eq(ip_addr))
        .filter(state.eq(SessionState::Transition))
        .first::<Session>(&mut connection)?)
}

pub fn create_session(new_session: NewSession) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(diesel::insert_into(sessions)
        .values(&new_session)
        .get_result::<Session>(&mut connection)?)
}

pub fn update_session(ses: &Session) -> crate::Result<Session> {
    let mut connection = crate::connection()?;

    Ok(ses.save_changes(&mut connection)?)
}

pub fn delete_session_by_id(s_id: i32) -> crate::Result<usize> {
    let mut connection = crate::connection()?;

    Ok(diesel::delete(sessions.filter(id.eq(s_id))).execute(&mut connection)?)
}

/// Alias for get_session_by_characterid
pub fn get_session_by_character_id(c_id: i32) -> crate::Result<Session> {
    get_session_by_characterid(c_id)
}

/// Update the character_id for a session
pub fn update_session_character(s_id: i32, c_id: i32) -> crate::Result<usize> {
    let mut connection = crate::connection()?;

    Ok(diesel::update(sessions.filter(id.eq(s_id)))
        .set(character_id.eq(Some(c_id)))
        .execute(&mut connection)?)
}

pub fn update_session_state(s_id: i32, new_state: SessionState) -> crate::Result<usize> {
    let mut connection = crate::connection()?;

    Ok(diesel::update(sessions.filter(id.eq(s_id)))
        .set(state.eq(new_state))
        .execute(&mut connection)?)
}

pub fn update_session_selection(s_id: i32, w_id: i16, ch_id: i16) -> crate::Result<usize> {
    let mut connection = crate::connection()?;

    Ok(diesel::update(sessions.filter(id.eq(s_id)))
        .set((
            selected_world_id.eq(Some(w_id)),
            selected_channel_id.eq(Some(ch_id)),
        ))
        .execute(&mut connection)?)
}
