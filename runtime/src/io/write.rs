use crate::error::RuntimeError;
use crypt::MapleAES;
use packet::Packet;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::OwnedWriteHalf;

/// How long one packet may take to reach the socket. A peer that stops
/// reading would otherwise block its connection's actor forever.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// A write to a peer that has gone only means it left.
fn left(error: std::io::Error) -> RuntimeError {
    if super::peer_left(&error) {
        RuntimeError::ClientDisconnected
    } else {
        error.into()
    }
}

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
        self.writer.write_all(packet).await.map_err(left)?;
        self.writer.flush().await.map_err(left)?;
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
            Ok(result) => result.map_err(left),
            Err(_) => Err(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "peer is not reading",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn writing_to_a_client_that_left_is_a_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (client, server) = tokio::join!(
            TcpStream::connect(listener.local_addr().unwrap()),
            listener.accept()
        );
        let client = client.unwrap();
        let (_, write_half) = server.unwrap().0.into_split();
        let mut writer = PacketWriter::new(write_half, &[1, 2, 3, 4]);
        client.set_zero_linger().unwrap();
        drop(client);
        // The first write may still be accepted before the reset arrives.
        let mut result = Ok(());
        for _ in 0..50 {
            result = writer.send_packet(&mut Packet::new(&[0x0a, 0, 1])).await;
            if result.is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(matches!(result, Err(RuntimeError::ClientDisconnected)));
    }
}
