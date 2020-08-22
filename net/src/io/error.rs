use config::ConfigError;
use std::fmt;
use std::{io, time::SystemTimeError};

#[derive(Debug)]
pub enum NetworkError {
    InvalidHeader,
    InvalidPacketLength(i16),
    NoData,
    PacketLengthDiscrepancy(i16, i16),
    InvalidPacket,
    CouldNotReadHeader(io::Error),
    CouldNotReadPacket(io::Error),
    PacketHandlerError(&'static str), // TODO: Ideally we make a separate error enum
    UnsupportedOpcodeError(i16),
    CouldNotSend(io::Error),
    IoError(io::Error),
    CouldNotEstablishConnection(io::Error),
    SystemTimeError(SystemTimeError),
    ConfigLoadError(ConfigError),
    DbError(db::Error),
    CryptError(crypt::BcryptError),
    ClientDisconnected,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::InvalidPacketLength(length) => {
                write!(f, "Packet length {} according to header is invalid", length)
            }
            NetworkError::PacketLengthDiscrepancy(actual, expected) => write!(
                f,
                "Actual length {} does not match reported length {}",
                actual, expected
            ),
            NetworkError::CouldNotReadPacket(e) => write!(f, "Error reading packet: {}", e),
            NetworkError::CouldNotReadHeader(e) => write!(f, "Error reading header: {}", e),
            NetworkError::PacketHandlerError(msg) => write!(f, "Error handling packet: {}", msg),
            NetworkError::UnsupportedOpcodeError(op) => write!(f, "Unsupported Opcode: {}", op),
            NetworkError::CouldNotSend(e) => write!(f, "Error Sending Packet: {}", e),
            NetworkError::ClientDisconnected => write!(f, "Client Disconnected."),
            NetworkError::CouldNotEstablishConnection(e) => {
                write!(f, "Could not establish connection: {}", e)
            }
            NetworkError::SystemTimeError(e) => write!(f, "System Time Conversion Error: {}", e),
            NetworkError::ConfigLoadError(e) => {
                write!(f, "Error loading configuration from file: {}", e)
            }
            NetworkError::DbError(e) => write!(f, "Database Error: {}", e),
            NetworkError::CryptError(e) => write!(f, "Error applying encryption: {}", e),
            e => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
        match error {
            ref e if e.kind() == io::ErrorKind::UnexpectedEof => NetworkError::ClientDisconnected,
            _ => NetworkError::IoError(error),
        }
    }
}

impl From<SystemTimeError> for NetworkError {
    fn from(e: SystemTimeError) -> Self {
        NetworkError::SystemTimeError(e)
    }
}

impl From<ConfigError> for NetworkError {
    fn from(e: ConfigError) -> Self {
        NetworkError::ConfigLoadError(e)
    }
}

impl From<db::Error> for NetworkError {
    fn from(e: db::Error) -> Self {
        NetworkError::DbError(e)
    }
}

impl From<crypt::BcryptError> for NetworkError {
    fn from(e: crypt::BcryptError) -> Self {
        NetworkError::CryptError(e)
    }
}
