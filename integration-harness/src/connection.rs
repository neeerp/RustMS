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
}

impl MapleTestConnection {
    pub async fn connect(
        endpoint: std::net::SocketAddr,
        phase: &'static str,
    ) -> Result<Self, HarnessError> {
        let mut stream = TcpStream::connect(endpoint)
            .await
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

        self.stream
            .write_all(&header)
            .await
            .map_err(|source| HarnessError::io(phase, self.endpoint, source))?;
        self.stream
            .write_all(&packet.bytes)
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

        let mut header = [0u8; 4];
        self.stream
            .read_exact(&mut header)
            .await
            .map_err(|source| HarnessError::io(phase, self.endpoint, source))?;

        if !self.recv_cipher.check_header(&header) {
            return Err(HarnessError::protocol(
                phase,
                self.endpoint,
                format!("invalid encrypted header {:02X?}", header),
            ));
        }

        let length = self.recv_cipher.get_packet_length(&header);
        if !(2..=MAX_PACKET_LENGTH).contains(&length) {
            return Err(HarnessError::protocol(
                phase,
                self.endpoint,
                format!("invalid packet length {length}"),
            ));
        }

        let mut body = vec![0u8; length as usize];
        self.stream
            .read_exact(&mut body)
            .await
            .map_err(|source| HarnessError::io(phase, self.endpoint, source))?;

        self.recv_cipher.crypt(&mut body);
        maple_crypt::decrypt(&mut body);

        Ok(PacketEnvelope {
            packet: Packet::new(&body),
        })
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
