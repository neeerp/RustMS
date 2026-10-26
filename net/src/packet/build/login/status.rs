use crate::codec::time::{expiration, filetime, ZERO_TIME};
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

/// The refusal for a blocked account: v83 reads the reason (BlockReason.img's
/// number, whose `msg` it shows) and the end after the usual padding, and
/// takes an end three or more years away for good (FUN_005f83ee, report
/// b003).
pub fn build_blocked_packet(reason: u8, until: Option<SystemTime>) -> Result<Packet, NetworkError> {
    let mut packet = build_login_status_packet(LoginStatusCode::Banned)?;
    packet.write_byte(reason)?;
    packet.write_long(expiration(until))?;
    Ok(packet)
}

/// The grade's GM bit: the client sends the '/' lines it doesn't know as
/// chat (report b169), and refuses trades and stores.
const GRADE_GM: u8 = 0x01;
/// With the GM bit, v83's admin account, which can't drop items or mesos
/// either (report b156).
const GRADE_ADMIN: u8 = 0x11;

/// The client's grade for `acc`: an admin GM's (row 205), or the admin
/// account the account is flagged as.
fn grade(acc: &Account) -> u8 {
    let mut grade = 0;
    if crate::policy::client_gm(acc.gm_level) {
        grade |= GRADE_GM;
    }
    if acc.test_admin {
        grade |= GRADE_ADMIN;
    }
    grade
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

    packet.write_byte(grade(acc))?;
    // The sub-grade stays 0: its 0x80 bit refuses every attack ("Not
    // available with an MWLB account"), and its 0x20/0x40 only rank a GM
    // the grade's bit 0 already ranks.
    packet.write_byte(0)?;
    packet.write_byte(0)?; // country

    packet.write_str_with_length(account_name)?;
    packet.write_byte(0)?;

    // A chat block: Etc/ChatBlockReason.img's number and its end, which the
    // client shows on entering a field and keeps until the next login
    // (report b180). It never checks the end itself.
    match acc.chat_blocked_at(SystemTime::now()) {
        Some((reason, until)) => {
            packet.write_byte(reason)?;
            packet.write_long(filetime(until))?;
        }
        None => {
            packet.write_byte(0)?;
            packet.write_long(0)?;
        }
    }
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
            nx_credit: 0,
            maple_points: 0,
            nx_prepaid: 0,
            storage_slots: 4,
            storage_meso: 0,
            test_admin: false,
            ban_reason: 0,
            ban_until: None,
            chat_block_reason: 0,
            chat_block_until: None,
        }
    }

    #[test]
    fn a_blocked_account_hears_why_and_until_when() {
        let until = UNIX_EPOCH + Duration::from_secs(1_000);
        let packet = build_blocked_packet(3, Some(until)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::LoginStatus as i16)
            .byte(2)
            .byte(0)
            .int(0);
        expect.byte(3).long(filetime(until));
        expect.end();

        let packet = build_blocked_packet(0, None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::LoginStatus as i16)
            .byte(2)
            .byte(0)
            .int(0);
        expect.byte(0).long(crate::codec::time::DEFAULT_TIME);
        expect.end();
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
        expect_login(packet, pin_flag, 0);
    }

    fn expect_login(packet: &Packet, pin_flag: u8, grade: u8) {
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).int(0).short(0);
        expect.int(4242).byte(1); // account id, gender
        expect.byte(grade).byte(0).byte(0); // grade, admin byte, country code
        expect.string("admin").byte(0);
        expect.byte(0).long(0).long(0); // not quiet-banned, ban time, creation time
        expect.int(1); // skip the "select the world" prompt
        expect.byte(pin_flag); // 0 = PIN system enabled, 1 = disabled
        expect.byte(2); // 2 = PIC disabled
        expect.end();
    }

    #[test]
    fn a_chat_block_reaches_the_client_until_it_ends() {
        let until = SystemTime::now() + Duration::from_secs(600);
        let muted = Account {
            chat_block_reason: 2,
            chat_block_until: Some(until),
            ..account()
        };
        let packet = build_successful_login_packet(&muted, false).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).int(0).short(0);
        expect.int(4242).byte(1).byte(0).byte(0).byte(0);
        expect.string("admin").byte(0);
        expect.byte(2).long(filetime(until)).long(0);

        let ended = Account {
            chat_block_until: Some(SystemTime::now() - Duration::from_secs(1)),
            ..muted
        };
        expect_successful_login(&build_successful_login_packet(&ended, false).unwrap(), 1);
    }

    #[test]
    fn successful_login_matches_reference() {
        let player = account();
        expect_successful_login(&build_successful_login_packet(&player, false).unwrap(), 1);
        expect_successful_login(&build_successful_login_packet(&player, true).unwrap(), 0);
        let helper = Account {
            gm_level: 2,
            ..account()
        };
        expect_login(
            &build_successful_login_packet(&helper, false).unwrap(),
            1,
            0,
        );
        let gm = Account {
            gm_level: 3,
            ..account()
        };
        expect_login(&build_successful_login_packet(&gm, false).unwrap(), 1, 1);
        let admin = Account {
            test_admin: true,
            ..player
        };
        expect_login(
            &build_successful_login_packet(&admin, false).unwrap(),
            1,
            0x11,
        );
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
