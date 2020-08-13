use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Result};

pub trait PktRead: ReadBytesExt {
    /// Read a byte.
    fn read_byte(&mut self) -> Result<u8> {
        self.read_u8()
    }

    /// Read a byte array of a given `length`.
    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        match self.read_exact(&mut buf) {
            Ok(_) => Ok(buf),
            Err(e) => Err(e),
        }
    }

    /// Read a short integer.
    fn read_short(&mut self) -> Result<i16> {
        self.read_i16::<LittleEndian>()
    }

    /// Read an integer.
    fn read_int(&mut self) -> Result<i32> {
        self.read_i32::<LittleEndian>()
    }

    /// Read a long integer.
    fn read_long(&mut self) -> Result<i64> {
        self.read_i64::<LittleEndian>()
    }

    /// Read a string of a given `length` starting at the `pos`th byte of the packet.
    fn read_str(&mut self, length: usize) -> Result<String> {
        let mut buf = vec![0u8; length];
        match self.read_exact(&mut buf) {
            Ok(_) => match String::from_utf8(buf) {
                Ok(string) => Ok(string),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            },
            Err(e) => Err(e),
        }
    }

    /// Read a length-headered string starting at the `pos`th byte of the packet.
    fn read_str_with_length(&mut self) -> Result<String> {
        match self.read_short() {
            Ok(length) => self.read_str(length as usize),
            Err(e) => Err(e),
        }
    }
}

impl<R: Read> PktRead for R {}

#[cfg(test)]
mod read_tests {
    use super::PktRead;
    use std::io::BufReader;

    use std::io::Write;

    use crate::model::packet::Packet;
    use byteorder::{LittleEndian, WriteBytesExt};
    use rand::distributions::Alphanumeric;
    use rand::{random, thread_rng, Rng};

    #[test]
    fn read_byte() {
        for _ in 0..100 {
            let byte: u8 = random();

            let packet = Packet::new(&[byte]);
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_byte().unwrap(), byte);
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
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_bytes(length).unwrap(), bytes.as_slice());
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
            let mut reader = BufReader::new(&*packet);

            for i in 0..length {
                assert_eq!(reader.read_byte().unwrap(), bytes[i]);
            }
        }
    }

    #[test]
    fn read_short() {
        for _ in 0..100 {
            let short: i16 = random();

            let mut buf = Vec::new();
            buf.write_i16::<LittleEndian>(short).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_short().unwrap(), short);
        }
    }

    #[test]
    fn read_int() {
        for _ in 0..100 {
            let integer: i32 = random();

            let mut buf = Vec::new();
            buf.write_i32::<LittleEndian>(integer).unwrap();

            let packet = Packet::new(&buf.as_slice());
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_int().unwrap(), integer);
        }
    }

    #[test]
    fn read_long() {
        for _ in 0..100 {
            let long: i64 = random();

            let mut buf = Vec::new();
            buf.write_i64::<LittleEndian>(long).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_long().unwrap(), long);
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

            let mut buf = Vec::new();
            buf.write_i16::<LittleEndian>(short).unwrap();
            buf.write_i32::<LittleEndian>(integer).unwrap();
            buf.write_i64::<LittleEndian>(long).unwrap();
            buf.write_i32::<LittleEndian>(integer2).unwrap();
            buf.write_i16::<LittleEndian>(short2).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_short().unwrap(), short);
            assert_eq!(reader.read_int().unwrap(), integer);
            assert_eq!(reader.read_long().unwrap(), long);
            assert_eq!(reader.read_int().unwrap(), integer2);
            assert_eq!(reader.read_short().unwrap(), short2);
        }
    }

    #[test]
    fn read_string() {
        for _ in 0..100 {
            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            let mut buf = Vec::new();
            buf.write(test_string.as_bytes()).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            assert_eq!(reader.read_str(test_string.len()).unwrap(), test_string);
        }
    }

    #[test]
    fn read_str_with_length() {
        for _ in 0..100 {
            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            let mut buf = Vec::new();
            buf.write_u16::<LittleEndian>(length as u16).unwrap();
            buf.write(test_string.as_bytes()).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            // assert_eq!(packet.read_short(0), length as i16);
            // assert_eq!(packet.read_str(2, test_string.len()), test_string);
            assert_eq!(reader.read_str_with_length().unwrap(), test_string);
        }
    }

    #[test]
    fn read_str_with_length_between_two_fixed_strs() {
        for _ in 0..100 {
            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            let hello = "Hello world!";
            let test = "Test!";

            let mut buf = Vec::new();
            buf.write_u16::<LittleEndian>(hello.len() as u16).unwrap();
            buf.write(hello.as_bytes()).unwrap();

            buf.write_u16::<LittleEndian>(length as u16).unwrap();
            buf.write(test_string.as_bytes()).unwrap();

            buf.write(test.as_bytes()).unwrap();

            let packet = Packet::new(&buf);
            let mut reader = BufReader::new(&*packet);

            // assert_eq!(packet.read_short().unwrap(), hello.len() as i16);
            assert_eq!(reader.read_str_with_length().unwrap(), hello);
            // assert_eq!(packet.read_short().unwrap(), length as i16);
            assert_eq!(reader.read_str_with_length().unwrap(), test_string);
            assert_eq!(reader.read_str(test.len() as usize).unwrap(), test);
        }
    }
}
