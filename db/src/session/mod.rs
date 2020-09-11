use crate::{character, schema::sessions};

extern crate ipnetwork;
use character::CharacterDTO;
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

impl Session {
    pub fn new<'a>(
        a_id: i32,
        hardware_id: &'a str,
        ip_addr: IpNetwork,
        session_state: SessionState,
    ) -> QueryResult<SessionWrapper> {
        let new_session = NewSession {
            account_id: a_id,
            hwid: hardware_id,
            ip: ip_addr,
            state: session_state,
        };

        let session = repository::create_session(new_session)?;
        SessionWrapper::new(session)
    }

    pub fn from_account_id(account_id: i32) -> QueryResult<SessionWrapper> {
        let session = repository::get_session_by_accountid(account_id)?;

        SessionWrapper::new(session)
    }
}

#[derive(Insertable)]
#[table_name = "sessions"]
pub struct NewSession<'a> {
    pub account_id: i32,
    pub ip: IpNetwork,
    pub hwid: &'a str,
    pub state: SessionState,
}

pub struct SessionWrapper {
    pub session: Option<Session>,
    character: Option<Rc<RefCell<CharacterDTO>>>,
}

impl SessionWrapper {
    pub fn new(session: Session) -> QueryResult<Self> {
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

    pub fn new_empty() -> Self {
        Self {
            session: None,
            character: None,
        }
    }

    // // TODO: This is really really ugly... I should be able to condense this but
    // // for some reason I'm struggling to do so... Too frustrated to keep trying
    // // as I'm writing this; will do later.
    // pub fn get_character(&mut self) -> QueryResult<Option<Rc<RefCell<CharacterDTO>>>> {
    //     match self.session.take() {
    //         Some(session) => {
    //             let chr = match session.character_id {
    //                 Some(c_id) => {
    //                     if let Some(chr) = self.character.take() {
    //                         self.character = Some(chr.clone());

    //                         Ok(Some(chr))
    //                     } else {
    //                         Ok(Some(self.load_character(c_id)?))
    //                     }
    //                 }
    //                 None => Ok(None),
    //             };

    //             self.session = Some(session);
    //             chr
    //         }
    //         None => Ok(None),
    //     }
    // }

    pub fn get_character(&mut self) -> QueryResult<Rc<RefCell<CharacterDTO>>> {
        self.session
            .as_ref()
            .and_then(|ses| ses.character_id)
            .and_then(|c_id| self.load_character(c_id).ok())
            .ok_or(diesel::result::Error::NotFound)
    }

    fn load_character(&mut self, c_id: i32) -> QueryResult<Rc<RefCell<CharacterDTO>>> {
        let chr = Rc::new(RefCell::new(CharacterDTO::from_character_id(c_id)?));

        self.character = Some(chr.clone());

        Ok(chr.clone())
    }
}
