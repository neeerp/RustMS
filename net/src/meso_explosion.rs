//! Meso Explosion (4211006): a Chief Bandit sets off the mesos lying in
//! front of them, and each hurts the mobs around it by what it is worth
//! (the v83 client's FUN_0096b3fb). The client picks the drops, rolls the
//! damage and sends both in its CLOSE_RANGE_ATTACK; the server takes the
//! drops it may and lets through only the damage they could do.

pub const MESO_EXPLOSION: i32 = 4211006;

/// It sets off at most this many drops, of which only the first 15 hurt
/// anything (FUN_00791fbc).
pub const MOST_DROPS: usize = 20;
const HURTING_DROPS: usize = 15;

/// How far the player may have moved from where the server last saw them.
const SLACK: i32 = 100;

/// What the client sends after its targets: where the player stands, the
/// drops it sets off, cheapest first, each with the targets it hurts (bit
/// j for the j-th), and when its hits land, in ms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explosion {
    pub position: (i16, i16),
    pub drops: Vec<(i32, u8)>,
    pub delay: i16,
}

/// A checked Meso Explosion, for the field to set off: its drops, when its
/// hits land, each target's lines with the index in `drops` of the drop
/// each came from, and the skill's `x`, box (lt, rb, facing left), and the
/// player's facing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blast {
    pub drops: Vec<i32>,
    pub delay: i16,
    pub lines: Vec<Vec<(usize, i32)>>,
    pub x: i32,
    pub lt: (i32, i32),
    pub rb: (i32, i32),
    pub left: bool,
}

/// What share of 50·x one line from a drop of `mesos` deals at most
/// (0x7920a3).
fn ratio(mesos: i32) -> f64 {
    let mesos = f64::from(mesos);
    if mesos <= 1000.0 {
        (mesos * 0.82 + 28.0) / 5300.0
    } else {
        mesos / (mesos + 5250.0)
    }
}

/// The most one line from a drop of `mesos` deals with the skill's `x`: its
/// client rolls 50·x times the ratio times a number from 0.5 up to 1.
pub fn line_cap(x: i32, mesos: i32) -> i32 {
    (f64::from(50 * x) * ratio(mesos)).ceil() as i32
}

/// Which drop each of the targets' `lines` came from: the k-th line of
/// target j from the k-th of the first 15 drops whose mask has bit j. None
/// if a mask names a target that isn't there, or a target's lines don't
/// match its drops.
pub fn sources(drops: &[(i32, u8)], lines: &[usize]) -> Option<Vec<Vec<usize>>> {
    if drops
        .iter()
        .any(|&(_, mask)| u32::from(mask) >> lines.len() != 0)
    {
        return None;
    }
    let sources: Vec<Vec<usize>> = (0..lines.len())
        .map(|target| {
            (drops.iter().enumerate().take(HURTING_DROPS))
                .filter(|(_, &(_, mask))| mask & (1 << target) != 0)
                .map(|(index, _)| index)
                .collect()
        })
        .collect();
    (sources.iter().map(Vec::len).eq(lines.iter().copied())).then_some(sources)
}

/// Whether a drop at `drop` lies in the skill's box (`lt`, `rb`, as facing
/// left) around the player at `user`: mirrored when they face right, and
/// `SLACK` wider all round.
pub fn reaches(
    lt: (i32, i32),
    rb: (i32, i32),
    left: bool,
    user: (i16, i16),
    drop: (i16, i16),
) -> bool {
    let (left_x, right_x) = if left { (lt.0, rb.0) } else { (-rb.0, -lt.0) };
    let dx = i32::from(drop.0) - i32::from(user.0);
    let dy = i32::from(drop.1) - i32::from(user.1);
    (left_x - SLACK..right_x + SLACK).contains(&dx) && (lt.1 - SLACK..rb.1 + SLACK).contains(&dy)
}

/// When the drop at `index` goes off, `delay` ms after the client's hits:
/// a tenth of a second later for each of the next four, then again
/// (Cosmic's spread), never more than a second.
pub fn delay(delay: i16, index: usize) -> i16 {
    delay
        .saturating_add((index % 5) as i16 * 100)
        .clamp(0, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_worth_more_the_more_mesos() {
        // At level 30 (x 1000), as the report works out.
        assert_eq!(line_cap(1000, 1000), 8000);
        assert_eq!(line_cap(1000, 50_000), 45_249);
        assert_eq!(line_cap(500, 1), 136);
        assert!(line_cap(1000, i32::MAX) <= 50_000);
    }

    #[test]
    fn lines_come_from_the_drops_that_hit_their_target() {
        // Two targets: the first hit by drops 0 and 2, the second by 1 and 2.
        let drops = [(10, 0b01), (11, 0b10), (12, 0b11), (13, 0)];
        assert_eq!(sources(&drops, &[2, 2]), Some(vec![vec![0, 2], vec![1, 2]]));
        assert_eq!(sources(&drops, &[2, 1]), None);
        // A mask naming a third target.
        assert_eq!(sources(&[(10, 0b100)], &[0, 0]), None);
        // No targets, drops alone.
        assert_eq!(sources(&[(10, 0)], &[]), Some(vec![]));
    }

    #[test]
    fn only_the_first_fifteen_drops_hurt() {
        let drops: Vec<(i32, u8)> = (0..20).map(|oid| (oid, 1)).collect();
        assert_eq!(
            sources(&drops, &[15]).unwrap()[0],
            (0..15).collect::<Vec<_>>()
        );
        assert_eq!(sources(&drops, &[20]), None);
    }

    #[test]
    fn the_box_turns_with_the_player() {
        let (lt, rb) = ((-300, -120), (0, 50));
        assert!(reaches(lt, rb, true, (0, 0), (-350, 0)));
        assert!(!reaches(lt, rb, true, (0, 0), (150, 0)));
        assert!(reaches(lt, rb, false, (0, 0), (350, 0)));
        assert!(!reaches(lt, rb, false, (0, 0), (-150, 0)));
        assert!(!reaches(lt, rb, false, (0, 0), (100, 200)));
    }

    #[test]
    fn drops_go_off_a_tenth_of_a_second_apart_in_fives() {
        let delays: Vec<i16> = (0..7).map(|index| delay(500, index)).collect();
        assert_eq!(delays, [500, 600, 700, 800, 900, 500, 600]);
        assert_eq!(delay(900, 4), 1000);
        assert_eq!(delay(-5, 0), 0);
    }
}
