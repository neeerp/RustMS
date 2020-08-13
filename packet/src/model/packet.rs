use std::io::{Result, Write};
use std::ops::{Deref, DerefMut};

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

    /// Instantiate a new, empty packet wrapper.
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
        (self.bytes.len() - 2) as i16
    }
}

impl Write for Packet {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.bytes.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.bytes.flush()
    }
}

impl Deref for Packet {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
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
            let length = rng.gen_range(2, (MAX_PACKET_LENGTH as usize) + 1);

            let mut buf: Vec<u8> = iter::repeat(0).take(length).collect();
            rng.fill(&mut buf[..]);

            let packet = Packet::new(&buf);

            assert_eq!(packet.len(), (length - 2) as i16);
        }
    }
}
