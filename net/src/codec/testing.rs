//! Byte-level assertions for golden packet tests.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

/// Walks an encoded buffer, asserting each field in order.
pub struct Expect<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Expect<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(bytes),
        }
    }

    fn at(&self) -> u64 {
        self.cursor.position()
    }

    pub fn byte(&mut self, expected: u8) -> &mut Self {
        let at = self.at();
        assert_eq!(self.cursor.read_u8().expect("byte"), expected, "byte at offset {at}");
        self
    }

    pub fn short(&mut self, expected: i16) -> &mut Self {
        let at = self.at();
        let value = self.cursor.read_i16::<LittleEndian>().expect("short");
        assert_eq!(value, expected, "short at offset {at}");
        self
    }

    pub fn int(&mut self, expected: i32) -> &mut Self {
        let at = self.at();
        let value = self.cursor.read_i32::<LittleEndian>().expect("int");
        assert_eq!(value, expected, "int at offset {at}");
        self
    }

    pub fn long(&mut self, expected: i64) -> &mut Self {
        let at = self.at();
        let value = self.cursor.read_i64::<LittleEndian>().expect("long");
        assert_eq!(value, expected, "long at offset {at}");
        self
    }

    pub fn bytes(&mut self, expected: &[u8]) -> &mut Self {
        let at = self.at();
        let mut actual = vec![0; expected.len()];
        self.cursor.read_exact(&mut actual).expect("bytes");
        assert_eq!(actual, expected, "bytes at offset {at}");
        self
    }

    pub fn string(&mut self, expected: &str) -> &mut Self {
        self.short(expected.len() as i16).bytes(expected.as_bytes())
    }

    /// Skip a field whose value is not under test (e.g. a timestamp).
    pub fn skip(&mut self, len: u64) -> &mut Self {
        self.cursor.set_position(self.at() + len);
        self
    }

    pub fn end(&mut self) {
        let remaining = self.cursor.get_ref().len() as u64 - self.at();
        assert_eq!(remaining, 0, "{remaining} unexpected trailing bytes");
    }
}
