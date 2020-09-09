use crate::error::NetworkError;
use bufstream::BufStream;
use crypt::{maple_crypt, MapleAES};
use db::{
    account::{self, Account},
    session::{self, Session, SessionState},
};
use packet::Packet;
use session::SessionWrapper;
use std::{io::Write, net::TcpStream, time::SystemTime};

/// A container for various pieces of information pertaining to a Session's
/// client.
pub struct MapleClient {
    pub stream: BufStream<TcpStream>,
    pub recv_crypt: MapleAES,
    pub send_crypt: MapleAES,
    pub session: SessionWrapper,
}

impl MapleClient {
    pub fn new(stream: BufStream<TcpStream>, recv_iv: &Vec<u8>, send_iv: &Vec<u8>) -> Self {
        let recv_crypt = MapleAES::new(recv_iv, 83);
        let send_crypt = MapleAES::new(send_iv, 83);

        MapleClient {
            stream,
            recv_crypt,
            send_crypt,
            session: SessionWrapper::new_empty(),
        }
    }

    /// Encrypt a packet with the custom Maplestory encryption followed by AES,
    /// and then send the packet to the client.
    pub fn send(&mut self, packet: &mut Packet) -> Result<(), NetworkError> {
        let header = self.send_crypt.gen_packet_header(packet.len() + 2);

        maple_crypt::encrypt(packet);
        self.send_crypt.crypt(packet);

        self.send_without_encryption(&header)?;
        self.send_without_encryption(packet)
    }

    /// Send data to the client.
    pub fn send_without_encryption(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        match self.stream.write(data) {
            Ok(_) => match self.stream.flush() {
                Ok(_) => Ok(()),
                Err(e) => Err(NetworkError::CouldNotSend(e)),
            },
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }

    /// Retrieve the account associated with the client session.
    pub fn get_account(&self) -> Option<Account> {
        match &self.session.session {
            Some(session) => account::get_account_by_id(session.account_id).ok(),
            None => None,
        }
    }

    // TODO: Move session logic into Session/Session Wrapper objects.

    pub fn login(
        &mut self,
        account_id: i32,
        hwid: &str,
        state: SessionState,
    ) -> Result<(), NetworkError> {
        let ip = self.stream.get_ref().peer_addr()?.ip();
        let ses = Session::new(account_id, &hwid, ip.into(), state)?;

        self.session = ses;
        Ok(())
    }

    pub fn complete_login(&mut self) -> Result<(), NetworkError> {
        match self.session.session.take() {
            Some(mut ses) => {
                ses.state = SessionState::AfterLogin;
                ses.updated_at = SystemTime::now();

                self.session.session = Some(session::update_session(&ses)?);
                Ok(())
            }
            None => Err(NetworkError::NotLoggedIn),
        }
    }

    pub fn transition(&mut self, character_id: i32) -> Result<(), NetworkError> {
        match self.session.session.take() {
            Some(mut ses) => {
                ses.state = SessionState::Transition;
                ses.character_id = Some(character_id);
                ses.updated_at = SystemTime::now();

                session::update_session(&ses)?;
                Ok(())
            }
            None => Err(NetworkError::NotLoggedIn),
        }
    }

    pub fn reattach(&mut self, character_id: i32) -> Result<(), NetworkError> {
        let ip = self.stream.get_ref().peer_addr()?.ip();

        let mut ses = session::get_session_to_reattach(character_id, ip.into())?;
        ses.state = SessionState::InGame;
        ses.updated_at = SystemTime::now();

        self.session.session = Some(session::update_session(&ses)?);

        Ok(())
    }

    pub fn logout(&mut self) -> Result<(), NetworkError> {
        match self.session.session.take() {
            Some(session) => {
                session::delete_session_by_id(session.id)?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}
