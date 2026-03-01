use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] db::Error),

    #[error("Encryption error: {0}")]
    Crypt(#[from] crypt::BcryptError),

    #[error("Client disconnected")]
    ClientDisconnected,

    #[error("Invalid packet header")]
    InvalidHeader,

    #[error("Invalid packet length: {0}")]
    InvalidPacketLength(i16),

    #[error("Unsupported opcode: {0}")]
    UnsupportedOpcode(i16),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Handler error: {0}")]
    Handler(String),
}
