use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::helpers::to_hex_string;
use crate::packet::build;
use crypt::login;
use db::{
    account::{self, Account},
    session::SessionState,
};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct LoginCredentialsHandler;

impl LoginCredentialsHandler {
    pub fn new() -> Self {
        Self
    }

    fn read_credentials(packet: &mut Packet) -> Result<(String, String, String), NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?; // prune opcode
        let user = reader.read_str_with_length()?;
        let pw = reader.read_str_with_length()?;
        reader.read_bytes(6)?; // prune padding
        let hwid = to_hex_string(&reader.read_bytes(4)?);
        Ok((user, pw, hwid))
    }

    fn verify_and_get_account(user: &str, pw: &str) -> Result<Option<Account>, NetworkError> {
        match account::get_account(user) {
            Ok(acc) => Ok(Self::verify_password(acc, pw)),
            Err(db::Error::NotFound) => Self::create_account(user, pw),
            Err(e) => Err(e.into()),
        }
    }

    fn verify_password(acc: Account, pw: &str) -> Option<Account> {
        match login::validate_against_hash(pw, &acc.password) {
            Ok(true) => {
                println!("Verified account with user '{}'", &acc.user_name);
                Some(acc)
            }
            _ => None,
        }
    }

    fn create_account(user: &str, pw: &str) -> Result<Option<Account>, NetworkError> {
        let pw = crypt::login::hash_password(pw)?;
        let acc = account::create_account(user, &pw)?;
        println!("Created user '{}'", user);
        Ok(Some(acc))
    }

    fn check_account_status(acc: &Account) -> u8 {
        if acc.banned {
            2
        } else if acc.logged_in {
            7
        } else if !acc.accepted_tos {
            23
        } else {
            0
        }
    }
}

impl PacketHandler for LoginCredentialsHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        println!("Login attempted...");
        let (user, pw, hwid) = Self::read_credentials(packet)?;

        match Self::verify_and_get_account(&user, &pw)? {
            Some(acc) => {
                let status = Self::check_account_status(&acc);
                match status {
                    0 => {
                        // Successful login
                        let login_packet =
                            build::login::status::build_successful_login_packet(&acc)?;
                        Ok(HandlerResult::empty()
                            .with_create_session(acc.id, hwid, SessionState::AfterLogin)
                            .with_reply(login_packet))
                    }
                    23 => {
                        // Need to accept TOS
                        let tos_packet = build::login::status::build_login_status_packet(23)?;
                        Ok(HandlerResult::empty()
                            .with_create_session(acc.id, hwid, SessionState::BeforeLogin)
                            .with_reply(tos_packet))
                    }
                    _ => {
                        // Reject login
                        let reject_packet =
                            build::login::status::build_login_status_packet(status)?;
                        Ok(HandlerResult::reply(reject_packet))
                    }
                }
            }
            None => {
                // Invalid credentials
                let reject_packet = build::login::status::build_login_status_packet(4)?;
                Ok(HandlerResult::reply(reject_packet))
            }
        }
    }
}
