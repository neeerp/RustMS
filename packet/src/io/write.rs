use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Result, Write};

pub trait PktWrite: WriteBytesExt {
    /// Write a byte to the end of a packet.
    fn write_byte(&mut self, byte: u8) -> Result<usize> {
        self.write(&[byte])
    }

    /// Write a byte array to the end of a packet.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<usize> {
        self.write(bytes)
    }

    /// Write a short integer to the end of a packet in Little Endian format.
    fn write_short(&mut self, short: i16) -> Result<()> {
        self.write_u16::<LittleEndian>(short as u16)
    }

    /// Write an integer to the end of a packet in Little Endian format.
    fn write_int(&mut self, int: i32) -> Result<()> {
        self.write_u32::<LittleEndian>(int as u32)
    }

    /// Write a long integer to the end of a packet in Little Endian format.
    fn write_long(&mut self, long: i64) -> Result<()> {
        self.write_u64::<LittleEndian>(long as u64)
    }

    /// Write a string to the end of a packet.
    fn write_str(&mut self, string: &str) -> Result<usize> {
        self.write(string.as_bytes())
    }

    /// Write a string's length followed by the string itself to the end of a packet.
    fn write_str_with_length(&mut self, string: &str) -> Result<usize> {
        match self.write_short(string.len() as i16) {
            Ok(_) => self.write_str(string),
            Err(e) => Err(e),
        }
    }
}

impl<W: Write> PktWrite for W {}

#[cfg(test)]
mod write_tests {
    use super::PktWrite;

    use crate::Packet;
    use rand::distributions::Alphanumeric;
    use rand::{random, thread_rng, Rng};

    use byteorder::{LittleEndian, ReadBytesExt};

    #[test]
    fn write_byte() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let byte: u8 = random();
            packet.write_byte(byte).unwrap();

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

            packet.write_bytes(&bytes).unwrap();

            // TODO: Might need to make sure this checks by element
            assert_eq!(packet.bytes, bytes.as_slice());
        }
    }

    #[test]
    fn write_short() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let short: i16 = random();

            packet.write_short(short).unwrap();

            assert_eq!(packet.bytes[0], (short & 0xFF) as u8);
            assert_eq!(packet.bytes[1], ((short >> 8) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_int() {
        for _ in 0..100 {
            let mut packet = Packet::new_empty();
            let integer: i32 = random();

            packet.write_int(integer).unwrap();

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

            packet.write_long(long).unwrap();

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

            packet.write_str(&test_string).unwrap();

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

            packet.write_str_with_length(&test_string).unwrap();

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
