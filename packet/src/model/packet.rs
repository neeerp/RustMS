use crate::io::{PktRead, PktWrite};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Write;

/// A sentinel for an invalid packet opcode.
pub const INVALID_OPCODE: i16 = 1;

/// The maximum length a network packet may take on.
pub const MAX_PACKET_LENGTH: i16 = i16::MAX;

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

impl PktRead for Packet {
    fn read_byte(&self, pos: usize) -> u8 {
        self.bytes[pos]
    }
    fn read_bytes(&self, pos: usize, length: usize) -> &[u8] {
        &self.bytes[pos..(pos + length)]
    }
    fn read_short(&self, pos: usize) -> i16 {
        let mut slice: &[u8] = &self.bytes[pos..];
        slice.read_i16::<LittleEndian>().unwrap()
    }
    fn read_int(&self, pos: usize) -> i32 {
        let mut slice: &[u8] = &self.bytes[pos..];
        slice.read_i32::<LittleEndian>().unwrap()
    }
    fn read_long(&self, pos: usize) -> i64 {
        let mut slice: &[u8] = &self.bytes[pos..];
        slice.read_i64::<LittleEndian>().unwrap()
    }
    fn read_str(&self, pos: usize, length: usize) -> String {
        let chars = self.bytes[pos..(pos + length)].to_vec();
        String::from_utf8(chars).unwrap()
    }
    fn read_str_with_length(&self, pos: usize) -> String {
        let length = self.read_short(pos) as usize;

        self.read_str(pos + 2, length)
    }
}

#[cfg(test)]
mod read_tests {
    use super::Packet;
    use crate::io::*;
    use rand::distributions::Alphanumeric;
    use rand::{random, thread_rng, Rng};

    #[test]
    fn read_byte() {
        for _ in 0..100 {
            let byte: u8 = random();
            let packet = Packet::new(&[byte]);

            assert_eq!(packet.read_byte(0), byte);
        }
    }

    #[test]
    fn read_bytes() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let length = rng.gen_range(1, 10);
            let mut bytes: Vec<u8> = Vec::new();

            for _ in 0..length {
                bytes.push(random())
            }

            let packet = Packet::new(&bytes);

            assert_eq!(packet.read_bytes(0, length), bytes.as_slice());
        }
    }

    #[test]
    fn read_bytes_one_at_a_time() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let length = rng.gen_range(1, 10);
            let mut bytes: Vec<u8> = Vec::new();

            for _ in 0..length {
                bytes.push(random())
            }

            let packet = Packet::new(&bytes);

            for i in 0..length {
                assert_eq!(packet.read_byte(i), bytes[i]);
            }
        }
    }

    #[test]
    fn read_short() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let short: i16 = random();

            packet.write_short(short);

            assert_eq!(packet.read_short(0), short);
        }
    }

    #[test]
    fn read_int() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let integer: i32 = random();

            packet.write_int(integer);

            assert_eq!(packet.read_int(0), integer);
        }
    }

    #[test]
    fn read_long() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let long: i64 = random();

            packet.write_long(long);

            assert_eq!(packet.read_long(0), long);
        }
    }

    #[test]
    fn read_numbers() {
        for _ in 0..100 {
            let short: i16 = random();
            let integer: i32 = random();
            let long: i64 = random();
            let integer2: i32 = random();
            let short2: i16 = random();

            let mut packet = Packet::new_empty();
            packet.write_short(short);
            packet.write_int(integer);
            packet.write_long(long);
            packet.write_int(integer2);
            packet.write_short(short2);

            assert_eq!(packet.read_short(0), short);
            assert_eq!(packet.read_int(2), integer);
            assert_eq!(packet.read_long(6), long);
            assert_eq!(packet.read_int(14), integer2);
            assert_eq!(packet.read_short(18), short2);
        }
    }

    #[test]
    fn read_string() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_str(&test_string);

            assert_eq!(packet.read_str(0, test_string.len()), test_string);
        }
    }

    #[test]
    fn read_str_with_length() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_str_with_length(&test_string);

            assert_eq!(packet.read_short(0), length as i16);
            assert_eq!(packet.read_str(2, test_string.len()), test_string);
            assert_eq!(packet.read_str_with_length(0), test_string);
        }
    }

    #[test]
    fn read_str_with_length_between_two_fixed_strs() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            let hello = "Hello world!";
            let test = "Test!";

            packet.write_str_with_length(hello);
            packet.write_str_with_length(&test_string);
            packet.write_str(test);

            assert_eq!(packet.read_short(0), hello.len() as i16);
            assert_eq!(packet.read_str_with_length(0), hello);
            assert_eq!(packet.read_short(2 + hello.len()), length as i16);
            assert_eq!(packet.read_str_with_length(2 + hello.len()), test_string);
            assert_eq!(
                packet.read_str(4 + hello.len() + length, test.len() as usize),
                test
            );
        }
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
    fn write_str() {
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
    fn write_str_with_length() {
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

#[cfg(test)]
mod test_packet {
    use super::{Packet, INVALID_OPCODE, MAX_PACKET_LENGTH};
    use byteorder::{LittleEndian, WriteBytesExt};
    use rand::{thread_rng, Rng};
    use std::iter;

    #[test]
    fn empty_packet_is_empty() {
        let packet = Packet::new_empty();

        assert_eq!(packet.bytes.len(), 0);
    }

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
