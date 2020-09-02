use crate::{
    error::NetworkError,
    helpers::to_hex_string,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use crypt::login;
use db::{
    account::{self, Account},
    session::SessionState,
};
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct LoginCredentialsHandler {}

// TODO: These methods could use some better error handling.

/// A handler for login attempt packets.
impl LoginCredentialsHandler {
    pub fn new() -> Self {
        LoginCredentialsHandler {}
    }

    /////// IO ///////

    /// Read the username, password, and HWID from the packet.
    fn read_credentials(
        &self,
        packet: &mut Packet,
    ) -> Result<(String, String, String), NetworkError> {
        let mut reader = BufReader::new(&**packet);

        reader.read_short()?; // prune opcode

        let user = reader.read_str_with_length()?;
        let pw = reader.read_str_with_length()?;

        reader.read_bytes(6)?; // prune padding

        let hwid = to_hex_string(&reader.read_bytes(4)?);

        Ok((user, pw, hwid))
    }

    /////// Get Account ///////

    /// Attempt to get an account, either by logging into the one corresponding
    /// to the credentials, or creating one if the user name given is not taken.
    fn verify_and_get_account(
        &self,
        user: &str,
        pw: &str,
    ) -> Result<Option<Account>, NetworkError> {
        match account::get_account(&user) {
            Ok(acc) => Ok(self.verify_password(acc, pw)),
            Err(db::Error::NotFound) => self.create_account(user, pw),
            Err(e) => Err(e.into()),
        }
    }

    /// Return the given account iff the given password matches.
    fn verify_password(&self, acc: Account, pw: &str) -> Option<Account> {
        match login::validate_against_hash(pw, &acc.password) {
            Ok(true) => {
                println!("Verified account with user '{}'", &acc.user_name);
                Some(acc)
            }
            _ => None,
        }
    }

    /// Create and save new account with the given username and password.
    fn create_account(&self, user: &str, pw: &str) -> Result<Option<Account>, NetworkError> {
        let pw = crypt::login::hash_password(pw)?;
        let acc = account::create_account(user, &pw)?;

        println!("Created user '{}'", user);

        Ok(Some(acc))
    }

    /// Attempt to log the user in.
    fn attempt_logon(
        &self,
        client: &mut MapleClient,
        acc: Account,
        hwid: &str,
    ) -> Result<(), NetworkError> {
        match self.check_account_status(&acc) {
            0 => self.accept_logon(client, acc, hwid),
            23 => self.send_tos(client, acc, hwid),
            status => self.reject_logon(client, status),
        }
    }

    fn check_account_status(&self, acc: &Account) -> u8 {
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

    /// Log the user in.
    fn accept_logon(
        &self,
        client: &mut MapleClient,
        acc: Account,
        hwid: &str,
    ) -> Result<(), NetworkError> {
        let mut packet = &mut build::login::status::build_successful_login_packet(&acc)?;

        client.login(acc.id, hwid, SessionState::AfterLogin)?;

        client.send(&mut packet)
    }

    /// Have the user accept the TOS.
    fn send_tos(
        &self,
        client: &mut MapleClient,
        acc: Account,
        hwid: &str,
    ) -> Result<(), NetworkError> {
        client.login(acc.id, hwid, SessionState::BeforeLogin)?;

        client.send(&mut build::login::status::build_login_status_packet(23)?)
    }

    /// Return a reason for why the user could not log in.
    fn reject_logon(&self, client: &mut MapleClient, status: u8) -> Result<(), NetworkError> {
        client.send(&mut build::login::status::build_login_status_packet(
            status,
        )?)
    }
}

impl PacketHandler for LoginCredentialsHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Login attempted...");
        let (user, pw, hwid) = self.read_credentials(packet)?;
        match self.verify_and_get_account(&user, &pw)? {
            Some(acc) => self.attempt_logon(client, acc, &hwid),
            None => self.reject_logon(client, 4),
        }
    }
}
