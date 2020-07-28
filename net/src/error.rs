use std::fmt;
use std::io;

#[derive(Debug)]
pub enum NetworkError {
    InvalidHeader,
    InvalidPacketLength(i16),
    NoData,
    PacketLengthDiscrepancy(i16, i16),
    InvalidPacket,
    CouldNotReadHeader(io::Error),
    CouldNotReadPacket(io::Error),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::InvalidPacketLength(length)=> {
                write!(f, "Packet length {} according to header is invalid", length)
            },
            NetworkError::PacketLengthDiscrepancy(actual, expected) => {
                write!(f, "Actual length {} does not match reported length {}", actual, expected)
            },
            NetworkError::CouldNotReadPacket(e) => {
                write!(f, "Error reading packet: {}", e)
            },
            NetworkError::CouldNotReadHeader(e) => {
                write!(f, "Error reading header: {}", e)
            },
            e => {
                write!(f, "{}", e)
            },
        }
    }
}