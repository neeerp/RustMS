use crate::error::RuntimeError;
use crypt::MapleAES;
use packet::Packet;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::OwnedWriteHalf;

/// How long one packet may take to reach the socket. A peer that stops
/// reading would otherwise block its connection's actor forever.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct PacketWriter {
    writer: BufWriter<OwnedWriteHalf>,
    crypt: MapleAES,
}

impl PacketWriter {
    pub fn new(write_half: OwnedWriteHalf, send_iv: &[u8]) -> Self {
        Self {
            writer: BufWriter::new(write_half),
            crypt: MapleAES::new(&send_iv.to_vec(), 83),
        }
    }

    /// Send the handshake packet (unencrypted).
    pub async fn send_handshake(&mut self, packet: &[u8]) -> Result<(), RuntimeError> {
        self.writer.write_all(packet).await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Encrypt and send a packet.
    pub async fn send_packet(&mut self, packet: &mut Packet) -> Result<(), RuntimeError> {
        let length = i16::try_from(packet.bytes.len())
            .map_err(|_| RuntimeError::InvalidPacketLength(i16::MAX))?;
        let mut frame = self.crypt.gen_packet_header(length);

        // Encrypt: first maple_crypt, then AES
        crypt::maple_crypt::encrypt(packet);
        self.crypt.crypt(packet);

        // Send header and body as one write so they leave in one segment.
        frame.extend_from_slice(&packet.bytes);
        let write = async {
            self.writer.write_all(&frame).await?;
            self.writer.flush().await
        };
        match tokio::time::timeout(WRITE_TIMEOUT, write).await {
            Ok(result) => Ok(result?),
            Err(_) => Err(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "peer is not reading",
            ))),
        }
    }
}
