use crate::{
    error::NetworkError,
    packet::op::SendOpcode::{GuestIdLogin, LoginStatus},
    settings::Settings,
};
use db::account::Account;
use packet::{io::write::PktWrite, Packet};
use crate::codec::time::{filetime, ZERO_TIME};
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

pub fn build_successful_login_packet(acc: &Account) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let opcode = LoginStatus as i16;

    let settings = Settings::new()?;

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
    packet.write_byte(if settings.login.pin_required { 0 } else { 1 })?;
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
    use std::sync::Mutex;
    use std::time::{Duration, UNIX_EPOCH};

    /// `Settings::new` reads `config/login_server_config` relative to the
    /// working directory, which is the crate directory under `cargo test`.
    /// Point it at the workspace root while the closure runs.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn at_workspace_root<T>(f: impl FnOnce() -> T) -> T {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
        let result = f();
        std::env::set_current_dir(previous).unwrap();
        result
    }

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

    fn successful_login() -> (Packet, bool) {
        at_workspace_root(|| {
            let pin_required = Settings::new().unwrap().login.pin_required;
            (build_successful_login_packet(&account()).unwrap(), pin_required)
        })
    }

    #[test]
    fn login_failure_carries_reason_then_padding() {
        let packet = build_login_status_packet(LoginStatusCode::IncorrectPassword).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).byte(4).byte(0).int(0);
        expect.end();
    }

    #[test]
    fn successful_login_matches_reference_up_to_pin_and_pic() {
        let (packet, _) = successful_login();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).int(0).short(0);
        expect.int(4242).byte(1); // account id, gender
        expect.byte(0).byte(0).byte(0); // GM flag, admin byte, country code
        expect.string("admin").byte(0);
        expect.byte(0).long(0); // not quiet-banned, no mute timestamp
        expect.skip(8); // creation timestamp (reference sends 0, see below)
        expect.int(1); // skip "select the world" prompt
        expect.skip(2); // PIN and PIC flags, see the ignored test
        expect.end();
    }

    #[test]
    fn successful_login_pin_pic_and_creation_time_match_reference() {
        let (packet, pin_required) = successful_login();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::LoginStatus as i16).int(0).short(0);
        expect.int(4242).byte(1);
        expect.byte(0).byte(0).byte(0);
        expect.string("admin").byte(0);
        expect.byte(0).long(0).long(0);
        expect.int(1);
        expect.byte(!pin_required as u8); // 0 = PIN system enabled, 1 = disabled
        expect.byte(2); // 2 = PIC disabled
        expect.end();
    }

    #[test]
    fn guest_login_layout_apart_from_timestamps() {
        let packet = build_guest_login_packet().unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuestIdLogin as i16).short(0x100);
        expect.skip(4); // guest id
        expect.skip(8 * 3); // zero, "zero time" and now; see the ignored test
        expect.int(0);
        expect.string("https://github.com/neeerp");
        expect.end();
    }

    #[test]
    fn guest_login_timestamps_match_reference() {
        let packet = build_guest_login_packet().unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GuestIdLogin as i16).short(0x100);
        expect.skip(4);
        expect.long(0).long(ZERO_TIME);
        expect.skip(8); // FILETIME now
        expect.int(0);
        expect.string("https://github.com/neeerp");
        expect.end();
    }
}
