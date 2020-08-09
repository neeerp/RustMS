use crate::io::{PktRead, PktWrite};

use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// A data model for a network packet.
pub struct Packet {
    pub bytes: Vec<u8>,
}

impl Packet {
    /// Instantiate a new packet wrapper.
    pub fn new(buffer: &[u8]) -> Packet {
        if buffer.len() > MAX_PACKET_LENGTH as usize {
            // We should not be reading a buffer so large in the first place!
            panic!(
                "Packet with length {} exceeded max packet length {}",
                buffer.len(),
                MAX_PACKET_LENGTH
            );
        }

        let bytes = buffer.to_vec();
        Packet { bytes }
    }

    pub fn new_empty() -> Packet {
        let bytes = vec![];

        Packet { bytes }
    }

    /// Return the opcode of packet.
    ///
    /// If the packet has no opcode because it is too short or if the
    /// opcode is negative, returns the `INVALID_OPCODE` sentinel.
    pub fn opcode(&self) -> i16 {
        if self.bytes.len() > 1 {
            let opcode: i16 = ((self.bytes[0] as u16) | ((self.bytes[1] as u16) << 8)) as i16;

            if opcode >= 0 {
                opcode
            } else {
                INVALID_OPCODE
            }
        } else {
            INVALID_OPCODE
        }
    }

    /// Return the length of the packet.
    pub fn len(&self) -> i16 {
        self.bytes.len() as i16
    }
}

#[allow(unused_variables)]
impl PktWrite for Packet {
    fn write_byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.write(bytes).unwrap();
    }
    fn write_short(&mut self, short: i16) {
        self.bytes.write_u16::<LittleEndian>(short as u16).unwrap();
    }
    fn write_int(&mut self, int: i32) {
        self.bytes.write_u32::<LittleEndian>(int as u32).unwrap();
    }
    fn write_long(&mut self, long: i64) {
        self.bytes.write_u64::<LittleEndian>(long as u64).unwrap();
    }
    fn write_str(&mut self, string: &str) {
        self.bytes.write(string.as_bytes()).unwrap();
    }
    fn write_str_with_length(&mut self, string: &str) {
        self.write_short(string.len() as i16);
        self.write_str(string);
    }
}

#[allow(unused_variables)]
impl PktRead for Packet {
    fn read_byte(&mut self) -> u8 {
        todo!()
    }
    fn read_bytes(&mut self, length: i16) -> &[u8] {
        todo!()
    }
    fn read_short(&mut self) -> i16 {
        todo!()
    }
    fn read_int(&mut self) -> i32 {
        todo!()
    }
    fn read_long(&mut self) -> i64 {
        todo!()
    }
    fn read_str(&mut self, length: i16) -> &str {
        todo!()
    }
    fn read_str_with_length(&mut self) -> &str {
        todo!()
    }
}

#[cfg(test)]
mod write_tests {
    use super::Packet;
    use crate::io::*;
    use rand::distributions::Alphanumeric;
    use rand::{random, thread_rng, Rng};

    use byteorder::{LittleEndian, ReadBytesExt};

    #[test]
    fn empty_packet_is_empty() {
        let packet = Packet::new_empty();

        assert_eq!(packet.bytes.len(), 0);
    }

    #[test]
    fn write_byte() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let byte: u8 = random();
            packet.write_byte(byte);

            assert_eq!(packet.bytes, [byte]);
        }
    }

    #[test]
    fn write_bytes() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let length = rng.gen_range(1, 10);
            let mut bytes: Vec<u8> = Vec::new();

            for _ in 0..length {
                bytes.push(random())
            }

            packet.write_bytes(&bytes);

            // TODO: Might need to make sure this checks by element
            assert_eq!(packet.bytes, bytes.as_slice());
        }
    }

    #[test]
    fn write_short() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let short: i16 = random();

            packet.write_short(short);

            assert_eq!(packet.bytes[0], (short & 0xFF) as u8);
            assert_eq!(packet.bytes[1], ((short >> 8) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_int() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let integer: i32 = random();

            packet.write_int(integer);

            assert_eq!(packet.bytes[0], (integer & 0xFF) as u8);
            assert_eq!(packet.bytes[1], ((integer >> 8) & 0xFF) as u8);
            assert_eq!(packet.bytes[2], ((integer >> 16) & 0xFF) as u8);
            assert_eq!(packet.bytes[3], ((integer >> 24) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_long() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let long: i64 = random();

            packet.write_long(long);

            assert_eq!(packet.bytes[0], (long & 0xFF) as u8);
            assert_eq!(packet.bytes[1], ((long >> 8) & 0xFF) as u8);
            assert_eq!(packet.bytes[2], ((long >> 16) & 0xFF) as u8);
            assert_eq!(packet.bytes[3], ((long >> 24) & 0xFF) as u8);
            assert_eq!(packet.bytes[4], ((long >> 32) & 0xFF) as u8);
            assert_eq!(packet.bytes[5], ((long >> 40) & 0xFF) as u8);
            assert_eq!(packet.bytes[6], ((long >> 48) & 0xFF) as u8);
            assert_eq!(packet.bytes[7], ((long >> 56) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_ascii() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_str(&test_string);

            assert_eq!(
                String::from_utf8(packet.bytes.to_vec()).unwrap(),
                test_string
            );
        }
    }

    #[test]
    fn write_length_headered_ascii() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_str_with_length(&test_string);

            assert_eq!(
                packet.bytes.as_slice().read_u16::<LittleEndian>().unwrap(),
                length as u16
            );

            assert_eq!(
                String::from_utf8(packet.bytes[2..].to_vec()).unwrap(),
                test_string
            );
        }
    }
}
/// A sentinel for an invalid packet opcode.
pub const INVALID_OPCODE: i16 = 1;

/// The maximum length a network packet may take on.
pub const MAX_PACKET_LENGTH: i16 = i16::MAX;

#[cfg(test)]
pub mod tests {
    use super::{Packet, INVALID_OPCODE, MAX_PACKET_LENGTH};
    use byteorder::{LittleEndian, WriteBytesExt};
    use rand::{thread_rng, Rng};
    use std::iter;

    #[test]
    fn read_correct_opcodes() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let opcode: i16 = rng.gen_range(0, (i16::MAX as usize) + 1) as i16;

            let mut buf = Vec::new();
            buf.write_i16::<LittleEndian>(opcode).unwrap();

            let packet = Packet::new(&buf);

            assert_eq!(packet.opcode(), opcode);
        }
    }

    #[test]
    fn read_negative_opcodes() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let opcode: i16 = rng.gen_range(i16::MIN, 0);

            let mut buf = Vec::new();
            buf.write_i16::<LittleEndian>(opcode).unwrap();

            let packet = Packet::new(&buf);

            assert_eq!(packet.opcode(), INVALID_OPCODE);
        }
    }

    #[test]
    fn empty_buffer_opcode() {
        let packet = Packet::new(&[]);

        assert_eq!(packet.opcode(), INVALID_OPCODE);
    }

    #[test]
    fn short_buffer_opcode() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let packet = Packet::new(&vec![rng.gen()]);

            assert_eq!(packet.opcode(), INVALID_OPCODE);
        }
    }

    #[test]
    fn packet_length_constant() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let length = rng.gen_range(0, (MAX_PACKET_LENGTH as usize) + 1);

            let mut buf: Vec<u8> = iter::repeat(0).take(length).collect();
            rng.fill(&mut buf[..]);

            let packet = Packet::new(&buf);

            assert_eq!(packet.len(), length as i16);
        }
    }
}
