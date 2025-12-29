use crate::error::RuntimeError;
use crypt::MapleAES;
use packet::Packet;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::OwnedWriteHalf;

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
        let header = self.crypt.gen_packet_header(packet.len() + 2);

        // Encrypt: first maple_crypt, then AES
        crypt::maple_crypt::encrypt(packet);
        self.crypt.crypt(packet);

        // Send header then body
        self.writer.write_all(&header).await?;
        self.writer.write_all(&packet.bytes).await?;
        self.writer.flush().await?;

        Ok(())
    }
}
