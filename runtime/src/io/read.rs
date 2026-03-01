use crate::error::RuntimeError;
use crypt::MapleAES;
use packet::{Packet, MAX_PACKET_LENGTH};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;

pub struct PacketReader {
    reader: BufReader<OwnedReadHalf>,
    crypt: MapleAES,
}

impl PacketReader {
    pub fn new(read_half: OwnedReadHalf, recv_iv: &[u8]) -> Self {
        Self {
            reader: BufReader::new(read_half),
            crypt: MapleAES::new(&recv_iv.to_vec(), 83),
        }
    }

    /// Read and decrypt a packet from the stream.
    pub async fn read_packet(&mut self) -> Result<Packet, RuntimeError> {
        let length = self.read_header().await?;
        self.read_data(length).await
    }

    async fn read_header(&mut self) -> Result<i16, RuntimeError> {
        let mut header_buf = [0u8; 4];

        match self.reader.read_exact(&mut header_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(RuntimeError::ClientDisconnected);
            }
            Err(e) => return Err(RuntimeError::Io(e)),
        }

        if !self.crypt.check_header(&header_buf) {
            return Err(RuntimeError::InvalidHeader);
        }

        let length = self.crypt.get_packet_length(&header_buf);

        if length < 2 || length > MAX_PACKET_LENGTH {
            return Err(RuntimeError::InvalidPacketLength(length));
        }

        Ok(length)
    }

    async fn read_data(&mut self, length: i16) -> Result<Packet, RuntimeError> {
        let mut buf = vec![0u8; length as usize];

        match self.reader.read_exact(&mut buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(RuntimeError::ClientDisconnected);
            }
            Err(e) => return Err(RuntimeError::Io(e)),
        }

        // Decrypt: first AES, then maple_crypt
        self.crypt.crypt(&mut buf);
        crypt::maple_crypt::decrypt(&mut buf);

        Ok(Packet::new(&buf))
    }
}
