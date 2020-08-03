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
