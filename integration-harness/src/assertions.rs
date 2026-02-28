use crate::config::HarnessConfig;
use crate::connection::PacketEnvelope;
use crate::error::HarnessError;
use crate::packets::{
    decode_login_status, decode_server_list, opcode_name, LoginStatusPacket, ServerListPacket,
};
use std::net::Ipv4Addr;

pub fn assert_handshake(
    phase: &'static str,
    endpoint: impl ToString,
    version: i16,
    locale: u8,
) -> Result<(), HarnessError> {
    let endpoint = endpoint.to_string();
    if version != 83 {
        return Err(HarnessError::protocol(
            phase,
            &endpoint,
            format!("expected handshake version 83 but got {version}"),
        ));
    }
    if locale != 8 {
        return Err(HarnessError::protocol(
            phase,
            &endpoint,
            format!("expected locale 8 but got {locale}"),
        ));
    }
    Ok(())
}

pub fn assert_login_success(
    packet: &PacketEnvelope,
    endpoint: impl ToString,
) -> Result<LoginStatusPacket, HarnessError> {
    let endpoint = endpoint.to_string();
    let status = decode_login_status(&packet.packet)
        .map_err(|message| HarnessError::protocol("login credentials", &endpoint, message))?;
    if status.status != 0 {
        return Err(HarnessError::protocol(
            "login credentials",
            &endpoint,
            format!("expected successful login status 0 but got {}", status.status),
        ));
    }
    Ok(status)
}

pub fn assert_server_list_kind(
    phase: &'static str,
    packet: &PacketEnvelope,
    endpoint: impl ToString,
    expected: ServerListPacket,
) -> Result<(), HarnessError> {
    let endpoint = endpoint.to_string();
    let actual = decode_server_list(&packet.packet)
        .map_err(|message| HarnessError::protocol(phase, &endpoint, message))?;
    if actual != expected {
        return Err(HarnessError::protocol(
            phase,
            &endpoint,
            format!("expected server list packet {:?} but got {:?}", expected, actual),
        ));
    }
    Ok(())
}

pub fn assert_opcode(
    phase: &'static str,
    packet: &PacketEnvelope,
    endpoint: impl ToString,
    expected: i16,
) -> Result<(), HarnessError> {
    let endpoint = endpoint.to_string();
    let actual = packet.opcode();
    if actual != expected {
        return Err(HarnessError::protocol(
            phase,
            &endpoint,
            format!(
                "expected opcode {} ({}) but got {} ({})",
                expected,
                opcode_name(expected),
                actual,
                opcode_name(actual)
            ),
        ));
    }
    Ok(())
}

pub fn assert_redirect_target(
    config: &HarnessConfig,
    endpoint: impl ToString,
    actual_ip: Ipv4Addr,
    actual_port: u16,
) -> Result<(), HarnessError> {
    let endpoint = endpoint.to_string();
    let expected_ip = match config.world_addr.ip() {
        std::net::IpAddr::V4(ip) => ip,
        std::net::IpAddr::V6(_) => {
            return Err(HarnessError::protocol(
                "server redirect",
                &endpoint,
                "expected an IPv4 world address for the current server redirect".to_string(),
            ));
        }
    };

    if actual_ip != expected_ip {
        return Err(HarnessError::protocol(
            "server redirect",
            &endpoint,
            format!("expected redirect ip {expected_ip} but got {actual_ip}"),
        ));
    }

    if actual_port != config.world_addr.port() {
        return Err(HarnessError::protocol(
            "server redirect",
            &endpoint,
            format!(
                "expected redirect port {} but got {}",
                config.world_addr.port(),
                actual_port
            ),
        ));
    }

    Ok(())
}
