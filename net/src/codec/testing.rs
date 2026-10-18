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
        assert_eq!(
            self.cursor.read_u8().expect("byte"),
            expected,
            "byte at offset {at}"
        );
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

    /// Read an int whose value is not fixed in advance (e.g. a random seed),
    /// returning it so the caller can assert relationships between fields.
    pub fn read_int(&mut self) -> i32 {
        self.cursor.read_i32::<LittleEndian>().expect("int")
    }

    /// The v83 stat block (`addCharStats` in HeavenMS), written out from the
    /// reference layout rather than through the encoder.
    pub fn stats(&mut self, s: &game::CharacterStats) -> &mut Self {
        let mut name = s.name.clone().into_bytes();
        name.resize(13, 0);
        self.int(s.id).bytes(&name).byte(s.gender).byte(s.skin);
        self.int(s.face).int(s.hair);
        self.long(0).long(0).long(0); // pet unique ids
        self.byte(s.level).short(s.job);
        self.short(s.str).short(s.dex).short(s.int).short(s.luk);
        self.short(s.hp).short(s.max_hp).short(s.mp).short(s.max_mp);
        self.short(s.ap).short(s.sp).int(s.exp).short(s.fame);
        self.int(0); // gacha exp
        self.int(s.map_id).byte(s.spawn_point).int(0)
    }

    /// The v83 avatar look (`addCharLook` in HeavenMS).
    pub fn look(&mut self, l: &game::AvatarLook, megaphone: bool) -> &mut Self {
        self.byte(l.gender).byte(l.skin).int(l.face);
        self.byte(if megaphone { 0 } else { 1 }).int(l.hair);
        for (&slot, &id) in &l.visible {
            self.byte(slot).int(id);
        }
        self.byte(0xFF);
        for (&slot, &id) in &l.masked {
            self.byte(slot).int(id);
        }
        self.byte(0xFF);
        self.int(l.cash_weapon.unwrap_or(0));
        self.int(l.pets[0]).int(l.pets[1]).int(l.pets[2])
    }

    /// Skip a field whose value is not under test (e.g. a timestamp).
    pub fn skip(&mut self, len: u64) -> &mut Self {
        let at = self.at();
        let total = self.cursor.get_ref().len() as u64;
        assert!(
            at + len <= total,
            "skip of {len} at offset {at} runs past the end ({total})"
        );
        self.cursor.set_position(at + len);
        self
    }

    pub fn end(&mut self) {
        let remaining = self.cursor.get_ref().len() as u64 - self.at();
        assert_eq!(remaining, 0, "{remaining} unexpected trailing bytes");
    }
}
