mod model;

pub use self::model::packet::{Packet, INVALID_OPCODE, MAX_PACKET_LENGTH};

pub use self::pkt::MaplePacket;

pub mod pkt {
    use byteorder::{LittleEndian, WriteBytesExt};
    use std::io::Write;

    pub struct MaplePacket {
        bytes: Vec<u8>,
    }

    impl MaplePacket {
        pub fn new() -> MaplePacket {
            MaplePacket { bytes: vec![] }
        }

        pub fn get_bytes(&self) -> &[u8] {
            self.bytes.as_slice()
        }

        pub fn write_byte(&mut self, byte: u8) {
            self.bytes.push(byte);
        }

        pub fn write_bytes(&mut self, bytes: &[u8]) {
            self.bytes.write(bytes).unwrap();
        }

        pub fn write_short(&mut self, short: i16) {
            self.bytes.write_u16::<LittleEndian>(short as u16).unwrap();
        }

        pub fn write_int(&mut self, int: i32) {
            self.bytes.write_u32::<LittleEndian>(int as u32).unwrap();
        }

        pub fn write_long(&mut self, long: i64) {
            self.bytes.write_u64::<LittleEndian>(long as u64).unwrap();
        }

        pub fn write_ascii(&mut self, string: &str) {
            self.bytes.write(string.as_bytes()).unwrap();
        }

        pub fn write_maple_ascii(&mut self, string: &str) {
            self.write_short(string.len() as i16);
            self.write_ascii(string);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::MaplePacket;
    use rand::distributions::Alphanumeric;
    use rand::{random, thread_rng, Rng};

    use byteorder::{LittleEndian, ReadBytesExt};

    #[test]
    fn empty_packet_is_empty() {
        let packet = MaplePacket::new();

        assert_eq!(packet.get_bytes().len(), 0);
    }

    #[test]
    fn write_byte() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();
            let byte: u8 = random();
            packet.write_byte(byte);

            assert_eq!(packet.get_bytes(), [byte]);
        }
    }

    #[test]
    fn write_bytes() {
        let mut rng = thread_rng();
        for _ in 0..100 {
            let mut packet = MaplePacket::new();
            let length = rng.gen_range(1, 10);
            let mut bytes: Vec<u8> = Vec::new();

            for _ in 0..length {
                bytes.push(random())
            }

            packet.write_bytes(&bytes);

            // TODO: Might need to make sure this checks by element
            assert_eq!(packet.get_bytes(), bytes.as_slice());
        }
    }

    #[test]
    fn write_short() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();
            let short: i16 = random();

            packet.write_short(short);

            assert_eq!(packet.get_bytes()[0], (short & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[1], ((short >> 8) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_int() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();
            let integer: i32 = random();

            packet.write_int(integer);

            assert_eq!(packet.get_bytes()[0], (integer & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[1], ((integer >> 8) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[2], ((integer >> 16) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[3], ((integer >> 24) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_long() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();
            let long: i64 = random();

            packet.write_long(long);

            assert_eq!(packet.get_bytes()[0], (long & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[1], ((long >> 8) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[2], ((long >> 16) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[3], ((long >> 24) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[4], ((long >> 32) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[5], ((long >> 40) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[6], ((long >> 48) & 0xFF) as u8);
            assert_eq!(packet.get_bytes()[7], ((long >> 56) & 0xFF) as u8);
        }
    }

    #[test]
    fn write_ascii() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_ascii(&test_string);

            assert_eq!(
                String::from_utf8(packet.get_bytes().to_vec()).unwrap(),
                test_string
            );
        }
    }

    #[test]
    fn write_length_headered_ascii() {
        for _ in 0..100 {
            let mut packet = MaplePacket::new();

            let length = rand::thread_rng().gen_range(0, 255);
            let test_string = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(length)
                .collect::<String>();

            packet.write_maple_ascii(&test_string);

            assert_eq!(
                packet.get_bytes().read_u16::<LittleEndian>().unwrap(),
                length as u16
            );

            assert_eq!(
                String::from_utf8(packet.get_bytes()[2..].to_vec()).unwrap(),
                test_string
            );
        }
    }
}
