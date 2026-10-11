use super::{NewSession, Session, SessionState};
use crate::schema::characters;
use crate::schema::sessions::dsl::*;
use diesel::dsl::{exists, now, IntervalDsl};
use diesel::expression_methods::*;
use diesel::{select, QueryDsl, RunQueryDsl};
use ipnetwork::IpNetwork;
use std::time::Duration;

/// How long a transition ticket stays valid for entering a channel.
pub const TRANSITION_TTL: Duration = Duration::from_secs(60);

fn ttl_seconds() -> i32 {
    TRANSITION_TTL.as_secs() as i32
}

pub fn create_session(new_session: NewSession) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    Ok(diesel::insert_into(sessions)
        .values(&new_session)
        .get_result::<Session>(&mut connection)?)
}

pub fn set_session_state(s_id: i32, new_state: SessionState) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    Ok(diesel::update(sessions.find(s_id))
        .set((state.eq(new_state), updated_at.eq(now)))
        .get_result(&mut connection)?)
}

pub fn update_session_selection(s_id: i32, w_id: i16, ch_id: i16) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    Ok(diesel::update(sessions.find(s_id))
        .set((
            selected_world_id.eq(Some(w_id)),
            selected_channel_id.eq(Some(ch_id)),
            updated_at.eq(now),
        ))
        .get_result(&mut connection)?)
}

/// Issue a transition ticket for `c_id`, but only if that character belongs
/// to the session's account. Returns `NotFound` otherwise.
pub fn begin_world_transition(s_id: i32, c_id: i32) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    let owned = characters::table
        .filter(characters::id.eq(c_id))
        .filter(characters::accountid.eq(account_id));
    Ok(diesel::update(sessions.find(s_id).filter(exists(owned)))
        .set((
            character_id.eq(Some(c_id)),
            state.eq(SessionState::Transition),
            updated_at.eq(now),
        ))
        .get_result(&mut connection)?)
}

/// Reissue a transition ticket so an in-game session can reconnect to
/// another channel.
pub fn begin_channel_transition(s_id: i32, ch_id: i16) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    Ok(diesel::update(sessions.find(s_id).filter(state.eq(SessionState::InGame)))
        .set((
            selected_channel_id.eq(Some(ch_id)),
            state.eq(SessionState::Transition),
            updated_at.eq(now),
        ))
        .get_result(&mut connection)?)
}

/// Atomically consume a fresh transition ticket for entering `ch_id` from
/// `ip_addr`, moving it to `InGame`. A ticket can be used exactly once.
pub fn claim_world_transition(c_id: i32, ch_id: i16, ip_addr: IpNetwork) -> crate::Result<Session> {
    let mut connection = crate::connection()?;
    Ok(diesel::update(
        sessions
            .filter(character_id.eq(Some(c_id)))
            .filter(selected_channel_id.eq(Some(ch_id)))
            .filter(ip.eq(ip_addr))
            .filter(state.eq(SessionState::Transition))
            .filter(updated_at.gt(now - ttl_seconds().seconds())),
    )
    .set((state.eq(SessionState::InGame), updated_at.eq(now)))
    .get_result(&mut connection)?)
}

pub fn delete_session(s_id: i32) -> crate::Result<usize> {
    let mut connection = crate::connection()?;
    Ok(diesel::delete(sessions.find(s_id)).execute(&mut connection)?)
}

/// Whether the account currently has a live connection (or a transition
/// ticket that is still valid).
pub fn account_is_online(a_id: i32) -> crate::Result<bool> {
    let mut connection = crate::connection()?;
    Ok(select(exists(
        sessions.filter(account_id.eq(a_id)).filter(
            state
                .ne(SessionState::Transition)
                .or(updated_at.gt(now - ttl_seconds().seconds())),
        ),
    ))
    .get_result(&mut connection)?)
}

/// Remove sessions that a restarted login server can no longer own.
pub fn clear_login_sessions() -> crate::Result<usize> {
    let mut connection = crate::connection()?;
    Ok(diesel::delete(sessions.filter(
        state
            .eq(SessionState::BeforeLogin)
            .or(state.eq(SessionState::AfterLogin))
            .or(state
                .eq(SessionState::Transition)
                .and(updated_at.le(now - ttl_seconds().seconds()))),
    ))
    .execute(&mut connection)?)
}

/// Remove in-game sessions for channels a restarted world server hosts.
pub fn clear_world_sessions(channel_ids: &[i16]) -> crate::Result<usize> {
    let mut connection = crate::connection()?;
    Ok(diesel::delete(
        sessions
            .filter(state.eq(SessionState::InGame))
            .filter(selected_channel_id.eq_any(channel_ids.iter().map(|&c| Some(c)))),
    )
    .execute(&mut connection)?)
}
