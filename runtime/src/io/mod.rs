pub mod read;
pub mod write;

use std::io;

pub use read::PacketReader;
pub use write::PacketWriter;

/// Whether an I/O error only means the peer has gone: a client that is
/// killed, or closes with data still unread, resets the connection, and a
/// write after it left breaks the pipe. It has still only left.
pub(crate) fn peer_left(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
    )
}
