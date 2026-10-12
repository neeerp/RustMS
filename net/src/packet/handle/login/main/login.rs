use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginAction, LoginContext, LoginHandler, LoginResult};
use crate::helpers::to_hex_string;
use crate::packet::build;
use crate::packet::build::login::status::LoginStatusCode;
use crypt::login;
use db::{
    account::{self, Account},
    session::{self, NewSession, Session, SessionState},
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::info;

pub struct LoginCredentialsHandler;

impl LoginCredentialsHandler {
    fn read_credentials(packet: &Packet) -> Result<(String, String, String), NetworkError> {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let user = reader.read_str_with_length()?;
        let pw = reader.read_str_with_length()?;
        reader.read_bytes(6)?;
        let hwid = to_hex_string(&reader.read_bytes(4)?);
        Ok((user, pw, hwid))
    }

    fn verify_and_get_account(user: &str, pw: &str) -> Result<Option<Account>, NetworkError> {
        match account::get_account(user) {
            Ok(acc) => Ok(login::validate_against_hash(pw, &acc.password)
                .unwrap_or(false)
                .then_some(acc)),
            // Unknown usernames register a new account (development convenience).
            Err(db::Error::NotFound) => {
                let hash = login::hash_password(pw)?;
                let acc = account::create_account(user, &hash)?;
                info!(user, "Created account");
                Ok(Some(acc))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn account_status(acc: &Account) -> LoginStatusCode {
        if acc.banned {
            LoginStatusCode::Banned
        } else if !acc.accepted_tos {
            LoginStatusCode::LicenseAgreement
        } else {
            LoginStatusCode::Success
        }
    }

    /// Release this connection's previous session (a re-login on the same
    /// connection) and atomically open a new one. `None` means the account
    /// is online elsewhere.
    fn acquire(
        ctx: &LoginContext,
        account_id: i32,
        hwid: &str,
        state: SessionState,
    ) -> Result<Option<Session>, NetworkError> {
        if let Some(previous) = ctx.session {
            session::delete_session(previous.id)?;
        }
        Ok(session::acquire_session(NewSession {
            account_id,
            ip: ctx.peer_ip.into(),
            hwid,
            state,
        })?)
    }
}

impl LoginHandler for LoginCredentialsHandler {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult {
        let (user, pw, hwid) = Self::read_credentials(packet)?;

        let Some(acc) = Self::verify_and_get_account(&user, &pw)? else {
            let reject = build::login::status::build_login_status_packet(
                LoginStatusCode::IncorrectPassword,
            )?;
            return Ok(HandlerResult::reply(reject));
        };

        let (state, reply) = match Self::account_status(&acc) {
            LoginStatusCode::Success => (
                SessionState::AfterLogin,
                build::login::status::build_successful_login_packet(&acc)?,
            ),
            LoginStatusCode::LicenseAgreement => (
                SessionState::BeforeLogin,
                build::login::status::build_login_status_packet(
                    LoginStatusCode::LicenseAgreement,
                )?,
            ),
            rejected => {
                return Ok(HandlerResult::reply(
                    build::login::status::build_login_status_packet(rejected)?,
                ))
            }
        };

        match Self::acquire(ctx, acc.id, &hwid, state)? {
            Some(session) => Ok(HandlerResult::empty()
                .with(LoginAction::SetSession(Some(session)))
                .with_reply(reply)),
            None => Ok(HandlerResult::empty()
                .with(LoginAction::SetSession(None))
                .with_reply(build::login::status::build_login_status_packet(
                    LoginStatusCode::AlreadyLoggedIn,
                )?)),
        }
    }
}
