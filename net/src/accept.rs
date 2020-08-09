//! # Packet Acceptor module
//!
//! This module contains the logic used to accept an incoming packet from a
//! client session's `TcpStream`.
//!

use crate::error::NetworkError;
use crypt::{maple_crypt, MapleAES};
use packet::{Packet, MAX_PACKET_LENGTH};

use bufstream::BufStream;
use std::io::Read;
use std::net::TcpStream;

/// Read, decrypt, and wrap a new incoming packet from a stream.
pub fn read_packet(
    stream: &mut BufStream<TcpStream>,
    crypt: &mut MapleAES,
) -> Result<Packet, NetworkError> {
    match read_header(stream, crypt) {
        Ok(data_length) => read_data(data_length, stream, crypt),
        Err(e) => Err(e),
    }
}

/// Read a new packet header from the session stream and use it to return the
/// length of the incoming packet.
fn read_header(
    stream: &mut BufStream<TcpStream>,
    crypt: &mut MapleAES,
) -> Result<i16, NetworkError> {
    let mut header_buf: [u8; 4] = [0u8; 4];

    match stream.read_exact(&mut header_buf) {
        Ok(_) => parse_header(&header_buf, crypt),
        Err(e) => Err(NetworkError::CouldNotReadHeader(e)),
    }
}

/// Read the data of a packet given its length from the session stream and
/// decrypt and wrap the data into a `Packet` struct.
fn read_data(
    data_length: i16,
    stream: &mut BufStream<TcpStream>,
    crypt: &mut MapleAES,
) -> Result<Packet, NetworkError> {
    let mut buf = vec![0u8; data_length as usize];

    match stream.read_exact(&mut buf) {
        Ok(_) => {
            // Decrypt incoming packet
            crypt.crypt(&mut buf[..]);
            maple_crypt::decrypt(&mut buf[..]);

            Ok(Packet::new(&buf[..]))
        }
        Err(_) => Err(NetworkError::InvalidPacket),
    }
}

/// Parse the packet header and return the length of the incoming packet.
fn parse_header(header_buf: &[u8; 4], crypt: &mut MapleAES) -> Result<i16, NetworkError> {
    if crypt.check_header(&header_buf[..]) {
        let length = crypt.get_packet_length(&header_buf[..]);

        validate_packet_length(length)
    } else {
        Err(NetworkError::InvalidHeader)
    }
}

/// Check that the given length value neither exceeds the maximum packet
/// length nor is too short to contain an opcode.
fn validate_packet_length(length: i16) -> Result<i16, NetworkError> {
    if length < 2 || length > MAX_PACKET_LENGTH {
        Err(NetworkError::InvalidPacketLength(length))
    } else {
        Ok(length)
    }
}
