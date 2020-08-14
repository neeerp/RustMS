use bufstream::BufStream;
use crypt::{maple_crypt, MapleAES};
use packet::Packet;
use std::{
    io::{Result, Write},
    net::TcpStream,
};

/// A container for various pieces of information pertaining to a Session's
/// client.
pub struct MapleClient {
    pub stream: BufStream<TcpStream>,
    pub recv_crypt: MapleAES,
    pub send_crypt: MapleAES,
}

impl MapleClient {
    pub fn new(stream: BufStream<TcpStream>, recv_iv: &Vec<u8>, send_iv: &Vec<u8>) -> Self {
        let recv_crypt = MapleAES::new(recv_iv, 83);
        let send_crypt = MapleAES::new(send_iv, 83);

        MapleClient {
            stream,
            recv_crypt,
            send_crypt,
        }
    }

    /// Encrypt a packet with the custom Maplestory encryption followed by AES,
    /// and then send the packet to the client.
    pub fn send(&mut self, packet: &mut Packet) -> Result<()> {
        let header = self.send_crypt.gen_packet_header(packet.len() + 2);

        maple_crypt::encrypt(packet);
        self.send_crypt.crypt(packet);

        match self.send_without_encryption(&header) {
            Ok(_) => self.send_without_encryption(packet),
            Err(e) => Err(e),
        }
    }

    /// Send data to the client.
    pub fn send_without_encryption(&mut self, data: &[u8]) -> Result<()> {
        match self.stream.write(data) {
            Ok(_) => self.stream.flush(),
            Err(e) => Err(e),
        }
    }
}
