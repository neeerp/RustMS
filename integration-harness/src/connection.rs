use crate::error::HarnessError;
use crate::handshake::Handshake;
use crypt::{maple_crypt, MapleAES};
use packet::{Packet, MAX_PACKET_LENGTH};
use std::collections::VecDeque;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone)]
pub struct PacketEnvelope {
    pub packet: Packet,
}

impl PacketEnvelope {
    pub fn opcode(&self) -> i16 {
        self.packet.opcode()
    }
}

pub struct MapleTestConnection {
    endpoint: std::net::SocketAddr,
    buffered_packets: VecDeque<PacketEnvelope>,
    stream: TcpStream,
    handshake: Handshake,
    send_cipher: MapleAES,
    recv_cipher: MapleAES,
    pending_bytes: Vec<u8>,
}

impl MapleTestConnection {
    pub async fn connect(
        endpoint: std::net::SocketAddr,
        phase: &'static str,
    ) -> Result<Self, HarnessError> {
        let mut stream = TcpStream::connect(endpoint)
            .await
            .map_err(|source| HarnessError::io(phase, endpoint, source))?;
        stream
            .set_nodelay(true)
            .map_err(|source| HarnessError::io(phase, endpoint, source))?;

        let handshake = read_handshake(&mut stream, endpoint, phase).await?;
        let send_cipher = MapleAES::new(&handshake.recv_iv, 83);
        let recv_cipher = MapleAES::new(&handshake.send_iv, 83);

        Ok(Self {
            endpoint,
            buffered_packets: VecDeque::new(),
            stream,
            handshake,
            send_cipher,
            recv_cipher,
            pending_bytes: Vec::new(),
        })
    }

    pub fn endpoint(&self) -> std::net::SocketAddr {
        self.endpoint
    }

    pub fn handshake(&self) -> &Handshake {
        &self.handshake
    }

    pub async fn send_packet(
        &mut self,
        mut packet: Packet,
        phase: &'static str,
    ) -> Result<(), HarnessError> {
        let header = self
            .send_cipher
            .gen_packet_header(packet.bytes.len() as i16);
        maple_crypt::encrypt(&mut packet);
        self.send_cipher.crypt(&mut packet.bytes);

        // Write one frame instead of sending a tiny header separately, which
        // can make request/response tests wait for TCP's delayed ACK behavior.
        let mut frame = Vec::with_capacity(header.len() + packet.bytes.len());
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&packet.bytes);
        self.stream
            .write_all(&frame)
            .await
            .map_err(|source| HarnessError::io(phase, self.endpoint, source))?;
        self.stream
            .flush()
            .await
            .map_err(|source| HarnessError::io(phase, self.endpoint, source))
    }

    pub fn push_back_packet(&mut self, packet: PacketEnvelope) {
        self.buffered_packets.push_front(packet);
    }

    pub async fn read_packet(
        &mut self,
        phase: &'static str,
    ) -> Result<PacketEnvelope, HarnessError> {
        if let Some(packet) = self.buffered_packets.pop_front() {
            return Ok(packet);
        }

        loop {
            if self.pending_bytes.len() >= 4 {
                let header = &self.pending_bytes[..4];
                if !self.recv_cipher.check_header(header) {
                    return Err(HarnessError::protocol(
                        phase,
                        self.endpoint,
                        format!("invalid encrypted header {:02X?}", header),
                    ));
                }
                let length = self.recv_cipher.get_packet_length(header);
                if !(2..=MAX_PACKET_LENGTH).contains(&length) {
                    return Err(HarnessError::protocol(
                        phase,
                        self.endpoint,
                        format!("invalid packet length {length}"),
                    ));
                }
                let frame_length = 4 + length as usize;
                if self.pending_bytes.len() >= frame_length {
                    let mut body = self.pending_bytes[4..frame_length].to_vec();
                    self.pending_bytes.drain(..frame_length);
                    self.recv_cipher.crypt(&mut body);
                    maple_crypt::decrypt(&mut body);
                    return Ok(PacketEnvelope {
                        packet: Packet::new(&body),
                    });
                }
            }
            // Unlike read_exact into a temporary buffer, this preserves bytes
            // when a timeout cancels this future partway through a frame.
            if self
                .stream
                .read_buf(&mut self.pending_bytes)
                .await
                .map_err(|source| HarnessError::io(phase, self.endpoint, source))?
                == 0
            {
                return Err(HarnessError::io(
                    phase,
                    self.endpoint,
                    std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "connection closed during packet read",
                    ),
                ));
            }
        }
    }
}

async fn read_handshake(
    stream: &mut TcpStream,
    endpoint: std::net::SocketAddr,
    phase: &'static str,
) -> Result<Handshake, HarnessError> {
    let mut len_buf = [0u8; 2];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|source| HarnessError::io(phase, endpoint, source))?;

    let length = i16::from_le_bytes(len_buf);
    if length <= 0 {
        return Err(HarnessError::protocol(
            phase,
            endpoint,
            format!("invalid handshake length {length}"),
        ));
    }

    let mut payload = vec![0u8; length as usize];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|source| HarnessError::io(phase, endpoint, source))?;

    let mut bytes = len_buf.to_vec();
    bytes.extend_from_slice(&payload);
    Handshake::parse(&bytes).map_err(|message| HarnessError::protocol(phase, endpoint, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::{timeout, Duration};

    fn encode(cipher: &mut MapleAES, plain: &[u8]) -> Vec<u8> {
        let mut packet = Packet::new(plain);
        let mut frame = cipher.gen_packet_header(plain.len() as i16).to_vec();
        maple_crypt::encrypt(&mut packet);
        cipher.crypt(&mut packet.bytes);
        frame.extend_from_slice(&packet.bytes);
        frame
    }

    #[tokio::test]
    async fn timed_out_reads_preserve_partial_frames_and_cipher_sequence() {
        for split in [2, 6] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (resume, paused) = oneshot::channel();
            let first = [0x30, 1, 1, 2, 3, 4];
            let second = [0x30, 1, 5, 6];
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let recv_iv = vec![1, 2, 3, 4];
                let send_iv = vec![5, 6, 7, 8];
                let handshake =
                    net::packet::build::build_handshake_packet(&recv_iv, &send_iv).unwrap();
                stream.write_all(&handshake.bytes).await.unwrap();
                let mut cipher = MapleAES::new(&send_iv, 83);
                let frame = encode(&mut cipher, &first);
                let following = encode(&mut cipher, &second);
                stream.write_all(&frame[..split]).await.unwrap();
                paused.await.unwrap();
                stream.write_all(&frame[split..]).await.unwrap();
                stream.write_all(&following).await.unwrap();
            });
            let mut conn = MapleTestConnection::connect(addr, "test handshake")
                .await
                .unwrap();
            assert!(
                timeout(Duration::from_millis(10), conn.read_packet("partial frame"))
                    .await
                    .is_err()
            );
            assert_eq!(conn.pending_bytes.len(), split);
            resume.send(()).unwrap();
            let packet = timeout(Duration::from_secs(1), conn.read_packet("resumed frame"))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(packet.packet.bytes, first);
            conn.push_back_packet(packet);
            assert_eq!(
                conn.read_packet("buffered envelope")
                    .await
                    .unwrap()
                    .packet
                    .bytes,
                first
            );
            assert_eq!(
                conn.read_packet("following frame")
                    .await
                    .unwrap()
                    .packet
                    .bytes,
                second
            );
            assert!(conn.read_packet("closed peer").await.is_err());
            server.await.unwrap();
        }
    }
}
