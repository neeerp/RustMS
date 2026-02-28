use packet::io::read::PktRead;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Handshake {
    pub version: i16,
    pub recv_iv: Vec<u8>,
    pub send_iv: Vec<u8>,
    pub locale: u8,
}

impl Handshake {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(bytes);
        let length = cursor
            .read_short()
            .map_err(|e| format!("failed to read handshake length: {e}"))?;
        if length != 0x0E {
            return Err(format!("unexpected handshake length header {length}"));
        }

        let version = cursor
            .read_short()
            .map_err(|e| format!("failed to read handshake version: {e}"))?;
        let _sub_version = cursor
            .read_short()
            .map_err(|e| format!("failed to read handshake sub-version: {e}"))?;
        let _server_type = cursor
            .read_byte()
            .map_err(|e| format!("failed to read handshake server type: {e}"))?;
        let recv_iv = cursor
            .read_bytes(4)
            .map_err(|e| format!("failed to read handshake recv IV: {e}"))?;
        let send_iv = cursor
            .read_bytes(4)
            .map_err(|e| format!("failed to read handshake send IV: {e}"))?;
        let locale = cursor
            .read_byte()
            .map_err(|e| format!("failed to read handshake locale: {e}"))?;

        Ok(Self {
            version,
            recv_iv,
            send_iv,
            locale,
        })
    }
}
