use crate::error::NetworkError;
use crate::handler::{HandlerResult, LoginAction, LoginContext, LoginHandler, LoginResult};
use crate::helpers::to_hex_string;
use crate::packet::build;
use crate::packet::build::login::status::LoginStatusCode;
use crypt::login;
use db::{
    account::{self, Account},
    session::{self, SessionState},
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

    fn account_status(acc: &Account) -> Result<LoginStatusCode, NetworkError> {
        Ok(if acc.banned {
            LoginStatusCode::Banned
        } else if session::account_is_online(acc.id)? {
            LoginStatusCode::AlreadyLoggedIn
        } else if !acc.accepted_tos {
            LoginStatusCode::LicenseAgreement
        } else {
            LoginStatusCode::Success
        })
    }
}

impl LoginHandler for LoginCredentialsHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &LoginContext) -> LoginResult {
        let (user, pw, hwid) = Self::read_credentials(packet)?;

        let Some(acc) = Self::verify_and_get_account(&user, &pw)? else {
            let reject = build::login::status::build_login_status_packet(
                LoginStatusCode::IncorrectPassword,
            )?;
            return Ok(HandlerResult::reply(reject));
        };

        match Self::account_status(&acc)? {
            LoginStatusCode::Success => {
                let login_packet = build::login::status::build_successful_login_packet(&acc)?;
                Ok(HandlerResult::empty()
                    .with(LoginAction::CreateSession {
                        account_id: acc.id,
                        hwid,
                        state: SessionState::AfterLogin,
                    })
                    .with_reply(login_packet))
            }
            LoginStatusCode::LicenseAgreement => {
                let tos_packet = build::login::status::build_login_status_packet(
                    LoginStatusCode::LicenseAgreement,
                )?;
                Ok(HandlerResult::empty()
                    .with(LoginAction::CreateSession {
                        account_id: acc.id,
                        hwid,
                        state: SessionState::BeforeLogin,
                    })
                    .with_reply(tos_packet))
            }
            rejected => Ok(HandlerResult::reply(
                build::login::status::build_login_status_packet(rejected)?,
            )),
        }
    }
}
