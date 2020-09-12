use crate::{character, schema::sessions};

extern crate ipnetwork;
use character::CharacterWrapper;
use diesel::QueryResult;
use ipnetwork::IpNetwork;

use std::{cell::RefCell, rc::Rc, time::SystemTime};

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

/// Session creation projection.
#[derive(Insertable)]
#[table_name = "sessions"]
pub struct NewSession<'a> {
    pub account_id: i32,
    pub ip: IpNetwork,
    pub hwid: &'a str,
    pub state: SessionState,
}

impl<'a> NewSession<'a> {
    /// Create and save a new session.
    pub fn create(self) -> QueryResult<SessionWrapper> {
        SessionWrapper::from(repository::create_session(self)?)
    }
}

/// A wrapper that holds a session as well as any additional
/// information pertaining to the session.
pub struct SessionWrapper {
    pub session: Option<Session>,
    character: Option<Rc<RefCell<CharacterWrapper>>>,
}

impl SessionWrapper {
    /// Create a new wrapper with no associated session.
    pub fn new_empty() -> Self {
        Self {
            session: None,
            character: None,
        }
    }

    /// Create a wrapper from an existing session, loading in the associated
    /// character if it exists.
    pub fn from(session: Session) -> QueryResult<Self> {
        let c_id = session.character_id;

        let mut wrapper = Self {
            session: Some(session),
            character: None,
        };

        if let Some(c_id) = c_id {
            wrapper.load_character(c_id)?;
        };

        Ok(wrapper)
    }

    /// Retrieve the character that the session is associated with, or a NotFound
    /// error if there is no character associated.
    pub fn get_character(&mut self) -> QueryResult<Rc<RefCell<CharacterWrapper>>> {
        self.session
            .as_ref()
            .and_then(|ses| ses.character_id)
            .and_then(|c_id| {
                self.character
                    .as_ref()
                    .and_then(|chr| Some(chr.clone()))
                    .or(self.load_character(c_id).ok())
            })
            .ok_or(diesel::result::Error::NotFound)
    }

    /// Attach a character to the session wrapper and return a counted refcell.
    fn load_character(&mut self, c_id: i32) -> QueryResult<Rc<RefCell<CharacterWrapper>>> {
        let chr = Rc::new(RefCell::new(CharacterWrapper::from_character_id(c_id)?));

        self.character = Some(chr.clone());

        Ok(chr.clone())
    }
}
