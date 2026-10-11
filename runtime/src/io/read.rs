use crate::error::RuntimeError;
use crypt::MapleAES;
use packet::{Packet, MAX_PACKET_LENGTH};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;

pub struct PacketReader {
    reader: BufReader<OwnedReadHalf>,
    crypt: MapleAES,
    // Keep partial frames across select! cancellation. Cipher state only changes
    // once a complete frame is available.
    pending_bytes: Vec<u8>,
}

impl PacketReader {
    pub fn new(read_half: OwnedReadHalf, recv_iv: &[u8]) -> Self {
        Self {
            reader: BufReader::new(read_half),
            crypt: MapleAES::new(&recv_iv.to_vec(), 83),
            pending_bytes: Vec::new(),
        }
    }

    /// Read and decrypt a packet from the stream.
    pub async fn read_packet(&mut self) -> Result<Packet, RuntimeError> {
        loop {
            if self.pending_bytes.len() >= 4 {
                let header = &self.pending_bytes[..4];
                if !self.crypt.check_header(header) {
                    return Err(RuntimeError::InvalidHeader);
                }
                let length = self.crypt.get_packet_length(header);
                if !(2..=MAX_PACKET_LENGTH).contains(&length) {
                    return Err(RuntimeError::InvalidPacketLength(length));
                }
                let frame_length = 4 + length as usize;
                if self.pending_bytes.len() >= frame_length {
                    let mut bytes = self.pending_bytes[4..frame_length].to_vec();
                    self.pending_bytes.drain(..frame_length);
                    self.crypt.crypt(&mut bytes);
                    crypt::maple_crypt::decrypt(&mut bytes);
                    return Ok(Packet::new(&bytes));
                }
            }
            // read_buf is cancellation safe; it appends bytes to persistent
            // state instead of a temporary header/body buffer.
            if self.reader.read_buf(&mut self.pending_bytes).await? == 0 {
                return Err(RuntimeError::ClientDisconnected);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::time::{timeout, Duration};

    fn encode(cipher: &mut MapleAES, plain: &[u8]) -> Vec<u8> {
        let mut packet = Packet::new(plain);
        let mut frame = cipher.gen_packet_header(plain.len() as i16).to_vec();
        crypt::maple_crypt::encrypt(&mut packet);
        cipher.crypt(&mut packet.bytes);
        frame.extend_from_slice(&packet.bytes);
        frame
    }

    #[tokio::test]
    async fn cancelled_reads_preserve_partial_headers_bodies_and_cipher_state() {
        for split in [2, 6] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let (client, server) = tokio::join!(
                TcpStream::connect(listener.local_addr().unwrap()),
                listener.accept()
            );
            let mut client = client.unwrap();
            let (read_half, _) = server.unwrap().0.into_split();
            let iv = [1, 2, 3, 4];
            let mut reader = PacketReader::new(read_half, &iv);
            let mut cipher = MapleAES::new(&iv.to_vec(), 83);
            let first = [0x3a, 0, 1, 2, 3, 4];
            let second = [0x3c, 0, 0, 1];
            let frame = encode(&mut cipher, &first);
            let following = encode(&mut cipher, &second);
            client.write_all(&frame[..split]).await.unwrap();
            assert!(timeout(Duration::from_millis(10), reader.read_packet())
                .await
                .is_err());
            assert_eq!(reader.pending_bytes.len(), split);
            client.write_all(&frame[split..]).await.unwrap();
            client.write_all(&following).await.unwrap();
            assert_eq!(reader.read_packet().await.unwrap().bytes, first);
            assert_eq!(reader.read_packet().await.unwrap().bytes, second);
        }
    }
}
