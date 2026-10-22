//! Object ids for the things a field puts on the map. Each kind has its own
//! range so ids never collide (NPCs use 1,000,000,000 and up).

use std::ops::Range;

pub const KITES: Range<i32> = 800_000_000..900_000_000;
pub const SUMMONS: Range<i32> = 900_000_000..1_000_000_000;
pub const MISTS: Range<i32> = 1_100_000_000..1_200_000_000;
pub const REACTORS: Range<i32> = 1_200_000_000..1_500_000_000;
pub const DROPS: Range<i32> = 1_500_000_000..2_000_000_000;
pub const MOBS: Range<i32> = 2_000_000_000..i32::MAX;

/// Hands out the ids of one range in order, wrapping around at its end.
#[derive(Debug)]
pub struct ObjectIds {
    range: Range<i32>,
    next: i32,
}

impl ObjectIds {
    pub const fn new(range: Range<i32>) -> Self {
        Self {
            next: range.start,
            range,
        }
    }

    pub fn next(&mut self) -> i32 {
        let id = self.next;
        self.next = if id + 1 < self.range.end {
            id + 1
        } else {
            self.range.start
        };
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_wrap_around_within_their_range() {
        let mut ids = ObjectIds::new(10..12);
        assert_eq!([ids.next(), ids.next(), ids.next()], [10, 11, 10]);
    }
}
