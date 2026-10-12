use crate::codec::time::{filetime, ZERO_TIME};
use crate::{
    error::NetworkError,
    packet::op::SendOpcode::{GuestIdLogin, LoginStatus},
};
use db::account::Account;
use packet::{io::write::PktWrite, Packet};
use std::time::SystemTime;

/// Result codes carried by the `LoginStatus` packet that the client knows how
/// to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LoginStatusCode {
    Success = 0,
    Banned = 2,
    IncorrectPassword = 4,
    AlreadyLoggedIn = 7,
    /// Prompts the client to accept the terms of service.
    LicenseAgreement = 23,
}

/// Build a login status packet that gets sent upon login failure, relaying the
/// reason.
pub fn build_login_status_packet(status: LoginStatusCode) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let opcode = LoginStatus as i16;

    packet.write_short(opcode)?;
    packet.write_byte(status as u8)?;
    packet.write_byte(0)?;
    packet.write_int(0)?;

    Ok(packet)
}

pub fn build_successful_login_packet(
    acc: &Account,
    pin_required: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let opcode = LoginStatus as i16;

    let account_id = acc.id;
    let gender = acc.gender;
    let account_name = &acc.user_name;

    packet.write_short(opcode)?;
    packet.write_int(0)?;
    packet.write_short(0)?;
    packet.write_int(account_id)?;
    packet.write_byte(gender as u8)?;

    packet.write_byte(0)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?;

    packet.write_str_with_length(account_name)?;
    packet.write_byte(0)?;

    packet.write_byte(0)?; // quiet ban
    packet.write_long(0)?; // quiet ban time
    packet.write_long(0)?; // creation time

    packet.write_int(1)?; // skip the "select a world" tutorial prompt

    // PIN: 0 = enabled, 1 = disabled. PIC: 2 = disabled (matches the char list).
    packet.write_byte(if pin_required { 0 } else { 1 })?;
    packet.write_byte(2)?;

    Ok(packet)
}

pub fn build_guest_login_packet() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let opcode = GuestIdLogin as i16;

    packet.write_short(opcode)?;
    packet.write_short(0x100)?;
    packet.write_int(0)?; // TODO: Should be random
    packet.write_long(0)?;
    packet.write_long(ZERO_TIME)?;
    packet.write_long(filetime(SystemTime::now()))?;
    packet.write_int(0)?;
    packet.write_str_with_length("https://github.com/neeerp")?;

    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use crate::packet::op::SendOpcode;
    use std::time::{Duration, UNIX_EPOCH};

    fn account() -> Account {
        Account {
            id: 4242,
            user_name: "admin".to_string(),
            password: String::new(),
            pin: String::new(),
            pic: String::new(),
            logged_in: false,
            last_login_at: None,
            created_at: UNIX_EPOCH + Duration::from_secs(1_000),
            character_slots: 3,
            gender: 1,
            accepted_tos: true,
            banned: false,
            ban_msg: None,
            gm_level: 0,
        }
    }

    #[test]
    fn login_failure_carries_reason_then_padding() {
        let packet = build_login_status_packet(LoginStatusCode::IncorrectPassword).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::LoginStatus as i16)
            .byte(4)
            .byte(0)
            .int(0);
        expect.end();
    }

    fn expect_successful_login(packet: &Packet, pin_flag: u8) {
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).int(0).short(0);
        expect.int(4242).byte(1); // account id, gender
        expect.byte(0).byte(0).byte(0); // GM flag, admin byte, country code
        expect.string("admin").byte(0);
        expect.byte(0).long(0).long(0); // not quiet-banned, ban time, creation time
        expect.int(1); // skip the "select the world" prompt
        expect.byte(pin_flag); // 0 = PIN system enabled, 1 = disabled
        expect.byte(2); // 2 = PIC disabled
        expect.end();
    }

    #[test]
    fn successful_login_matches_reference() {
        let account = account();
        expect_successful_login(&build_successful_login_packet(&account, false).unwrap(), 1);
        expect_successful_login(&build_successful_login_packet(&account, true).unwrap(), 0);
    }

    #[test]
    fn guest_login_matches_reference() {
        let packet = build_guest_login_packet().unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuestIdLogin as i16).short(0x100);
        expect.skip(4); // guest id
        expect.long(0).long(ZERO_TIME);
        expect.skip(8); // FILETIME now
        expect.int(0);
        expect.string("https://github.com/neeerp");
        expect.end();
    }
}
