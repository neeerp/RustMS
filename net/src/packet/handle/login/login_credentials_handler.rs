use crate::{
    error::NetworkError,
    helpers::to_hex_string,
    io::client::MapleClient,
    packet::{build, handle::PacketHandler},
};
use crypt::login;
use db::accounts;
use db::models::Account;
use packet::{io::read::PktRead, Packet};
use std::{io::BufReader, time::SystemTime};

pub struct LoginCredentialsHandler {}

// TODO: These methods could use some better error handling.

/// A handler for login attempt packets.
impl LoginCredentialsHandler {
    pub fn new() -> Self {
        LoginCredentialsHandler {}
    }

    /// Attempt to get an account, either by logging into the one corresponding
    /// to the credentials, or creating one if the user name given is not taken.
    fn verify_and_get_account(&self, user: &str, pw: &str) -> Option<Account> {
        match accounts::get_account(&user) {
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
                accounts::create_account(user, &hashed_pw)
            }
            _ => None,
        }
    }

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

    /// Set the session's login state and save the login state in teh database.
    fn try_login(&self, client: &mut MapleClient, acc: &mut Account) -> u8 {
        if acc.banned {
            // TODO: This isn't right...
            println!("Banned account.");
            3
        } else if acc.logged_in {
            println!("Account already logged in.");
            7
        } else {
            acc.logged_in = true;
            acc.last_login_at = Some(SystemTime::now());
            match accounts::login_account(&acc) {
                Ok(_) => {
                    client.logged_in = true;
                    client.user_id = acc.id;
                    0
                }
                _ => 5,
            }
        }
    }

    fn accept_logon(&self, client: &mut MapleClient, acc: Account) -> Result<(), NetworkError> {
        let mut packet = build::login::status::build_successful_login_packet(&acc);

        match client.send(&mut packet) {
            Ok(_) => Ok(()),
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }

    fn reject_logon(&self, client: &mut MapleClient, status: u8) -> Result<(), NetworkError> {
        let mut packet = build::login::status::build_login_status_packet(status);

        match client.send(&mut packet) {
            Ok(_) => Ok(()),
            Err(e) => Err(NetworkError::CouldNotSend(e)),
        }
    }
}

// TODO: Could be cleaned up...
impl PacketHandler for LoginCredentialsHandler {
    fn handle(&self, packet: &mut Packet, client: &mut MapleClient) -> Result<(), NetworkError> {
        println!("Login attempted...");
        let (user, pw, _hwid) = self.read_credentials(packet);
        match self.verify_and_get_account(&user, &pw) {
            Some(mut acc) => {
                let status = self.try_login(client, &mut acc);
                if status == 0 {
                    self.accept_logon(client, acc)
                } else {
                    self.reject_logon(client, status)
                }
            }
            None => self.reject_logon(client, 4),
        }
    }
}
