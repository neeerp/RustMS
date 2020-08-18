use crate::{
    error::NetworkError,
    helpers::to_hex_string,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use crypt::login;
use db::account::{self, Account};
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
    fn read_credentials(&self, packet: &mut Packet) -> (String, String, String) {
        let mut reader = BufReader::new(&**packet);

        reader.read_short().unwrap(); // prune opcode

        let user = reader.read_str_with_length().unwrap();
        let pw = reader.read_str_with_length().unwrap();

        reader.read_bytes(6).unwrap(); // prune padding

        let hwid = to_hex_string(&reader.read_bytes(4).unwrap());

        (user, pw, hwid)
    }

    /////// Get Account ///////

    /// Attempt to get an account, either by logging into the one corresponding
    /// to the credentials, or creating one if the user name given is not taken.
    fn verify_and_get_account(&self, user: &str, pw: &str) -> Option<Account> {
        match account::get_account(&user) {
            Some(acc) => self.verify_password(acc, pw),
            None => self.create_account(user, pw),
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
    fn create_account(&self, user: &str, pw: &str) -> Option<Account> {
        match crypt::login::hash_password(pw) {
            Ok(hashed_pw) => {
                println!("Attempting to create account with user '{}'", user);
                account::create_account(user, &hashed_pw)
            }
            _ => None,
        }
    }

    /// Log the user in.
    fn accept_logon(&self, client: &mut MapleClient, acc: Account) -> Result<(), NetworkError> {
        let mut packet = build::login::status::build_successful_login_packet(&acc);

        match client.send(&mut packet) {
            Ok(_) => Ok(()),
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }

    /// Return a reason for why the user could not log in.
    fn reject_logon(&self, client: &mut MapleClient, status: u8) -> Result<(), NetworkError> {
        let mut packet = build::login::status::build_login_status_packet(status);

        match client.send(&mut packet) {
            Ok(_) => Ok(()),
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }
}

impl PacketHandler for LoginCredentialsHandler {
    // For simplicity's sake, we're going to ignore the PIC and PIN and not
    // worry about whether the account is already logged in or anything like
    // that. We can deal with that later.
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Login attempted...");
        let (user, pw, _hwid) = self.read_credentials(packet);
        match self.verify_and_get_account(&user, &pw) {
            Some(acc) => self.accept_logon(client, acc),
            None => self.reject_logon(client, 4),
        }
    }
}
