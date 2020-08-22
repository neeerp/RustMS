use crate::{
    error::NetworkError,
    packet::op::SendOpcode::{GuestIdLogin, LoginStatus},
    settings::Settings,
};
use db::account::Account;
use packet::{io::write::PktWrite, Packet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Build a login status packet that gets sent upon login failure, relaying the
/// reason.
pub fn build_login_status_packet(status: u8) -> Result<Packet, NetworkError> {
    // TODO: Need to create an enum for the status...
    let mut packet = Packet::new_empty();
    let opcode = LoginStatus as i16;

    packet.write_short(opcode)?;
    packet.write_byte(status)?;
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
    let created_at: i64 = acc.created_at.duration_since(UNIX_EPOCH)?.as_secs() as i64;

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

    packet.write_byte(0)?;
    packet.write_long(0)?;
    packet.write_long(created_at)?;

    packet.write_int(1)?;

    // PIN/PIC?
    packet.write_byte(settings.login.pin_required as u8)?;
    packet.write_byte(1)?;

    Ok(packet)
}

pub fn build_guest_login_packet() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    let opcode = GuestIdLogin as i16;

    let now: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as i64;

    packet.write_short(opcode)?;
    packet.write_short(0x100)?;
    packet.write_int(0)?; // TODO: Should be random
    packet.write_long(0)?;
    packet.write_long(0)?;
    packet.write_long(now)?;
    packet.write_int(0)?;
    packet.write_str_with_length("https://github.com/neeerp")?;

    Ok(packet)
}
