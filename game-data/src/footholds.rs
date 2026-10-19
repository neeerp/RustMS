//! A map's footholds: the lines that players, mobs, and drops stand on.

/// How far above a drop's start the ground below it is searched from, as
/// v83 servers place drops.
const DROP_SEARCH_HEIGHT: i16 = 50;
/// How far a floor may be from a point's height and still count as the
/// floor that point stands on.
const FLOOR_TOLERANCE: i32 = 5;

/// One foothold line, `Map.nx` `foothold/<layer>/<group>/<id>`. Lines with
/// the same x at both ends are walls; the rest are floors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Foothold {
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
}

impl Foothold {
    pub fn is_wall(&self) -> bool {
        self.x1 == self.x2
    }

    /// The floor's height at `x`, if it is a floor that covers `x`.
    fn ground_at(&self, x: i16) -> Option<i32> {
        if self.is_wall() || x < self.x1.min(self.x2) || x > self.x1.max(self.x2) {
            return None;
        }
        let (x1, y1, x2, y2) = (
            i32::from(self.x1),
            i32::from(self.y1),
            i32::from(self.x2),
            i32::from(self.y2),
        );
        Some(y1 + (y2 - y1) * (i32::from(x) - x1) / (x2 - x1))
    }

    /// Whether this wall spans any height from `top` to `bottom`.
    fn wall_overlaps(&self, top: i32, bottom: i32) -> bool {
        let (y1, y2) = (i32::from(self.y1), i32::from(self.y2));
        self.is_wall() && y1.min(y2) <= bottom && y1.max(y2) >= top
    }
}

#[derive(Clone, Debug, Default)]
pub struct Footholds {
    lines: Vec<Foothold>,
}

impl Footholds {
    pub fn new(lines: Vec<Foothold>) -> Self {
        Self { lines }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The map's bounds as the v83 client works them out from its footholds
    /// (r25 0xa44889): their extent, 30 px in from each side, 300 px above
    /// the highest and 10 px below the lowest, as (left, top, right,
    /// bottom). None without footholds.
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let xs = || {
            self.lines
                .iter()
                .flat_map(|fh| [fh.x1, fh.x2])
                .map(i32::from)
        };
        let ys = || {
            self.lines
                .iter()
                .flat_map(|fh| [fh.y1, fh.y2])
                .map(i32::from)
        };
        Some((
            xs().min()? + 30,
            ys().min()? - 300,
            xs().max()? - 30,
            ys().max()? + 10,
        ))
    }

    /// The highest ground at `x` that is at or below `y`.
    pub fn ground_below(&self, x: i16, y: i16) -> Option<i16> {
        self.lines
            .iter()
            .filter_map(|fh| fh.ground_at(x))
            .filter(|&ground| ground >= i32::from(y))
            .min()
            .and_then(|ground| i16::try_from(ground).ok())
    }

    /// Whether a floor at about height `y` covers `x`.
    fn floor_at(&self, x: i16, y: i16) -> bool {
        self.lines
            .iter()
            .filter_map(|fh| fh.ground_at(x))
            .any(|ground| (ground - i32::from(y)).abs() <= FLOOR_TOLERANCE)
    }

    /// Whether a wall stands between `from` and the column at `x`, at the
    /// heights the client's physics checks (up to 50 px above `from`). A
    /// mob stopped by a wall stands exactly on its x, so a wall there only
    /// counts if `from`'s floor doesn't go on past it toward `x`.
    pub fn wall_between(&self, from: (i16, i16), x: i16) -> bool {
        let (fx, fy) = from;
        if x == fx {
            return false;
        }
        let top = i32::from(fy) - i32::from(DROP_SEARCH_HEIGHT);
        let bottom = i32::from(fy) - 1;
        let step = if x < fx { -1 } else { 1 };
        let open_at_start = self.floor_at(fx.saturating_add(step), fy);
        self.lines
            .iter()
            .filter(|fh| fh.wall_overlaps(top, bottom))
            .any(|wall| {
                let wx = wall.x1;
                let between = if x < fx {
                    x <= wx && wx < fx
                } else {
                    fx < wx && wx <= x
                };
                between || (wx == fx && !open_at_start)
            })
    }

    /// Where a drop thrown from `from` toward `x` comes to rest: on the
    /// ground below `x`, or below `from` itself when no ground is there or a
    /// wall is in the way. Without footholds the drop keeps `from`'s height.
    pub fn drop_position(&self, from: (i16, i16), x: i16) -> (i16, i16) {
        if self.is_empty() {
            return (x, from.1);
        }
        let search = from.1.saturating_sub(DROP_SEARCH_HEIGHT);
        let fallback = (from.0, self.ground_below(from.0, search).unwrap_or(from.1));
        if self.wall_between(from, x) {
            return fallback;
        }
        match self.ground_below(x, search) {
            Some(y) => (x, y),
            None => fallback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x1: i16, y1: i16, x2: i16, y2: i16) -> Foothold {
        Foothold { x1, y1, x2, y2 }
    }

    /// A cliff: upper ground at y 100 left of x 0, a wall down to the lower
    /// ground at y 200, which runs right to x 300 and then ends in the air.
    fn cliff() -> Footholds {
        Footholds::new(vec![
            line(-300, 100, 0, 100),
            line(0, 100, 0, 200),
            line(0, 200, 150, 200),
            line(150, 200, 300, 180),
        ])
    }

    #[test]
    fn bounds_take_the_client_margins_around_every_line() {
        // The cliff spans x -300..300 and y 100..200, its wall included.
        assert_eq!(cliff().bounds(), Some((-270, -200, 270, 210)));
        assert_eq!(Footholds::default().bounds(), None);
    }

    #[test]
    fn drops_land_on_the_ground_below_their_column() {
        let footholds = cliff();
        assert_eq!(footholds.drop_position((100, 200), 125), (125, 200));
        assert_eq!(footholds.drop_position((100, 200), 75), (75, 200));
        // On the slope the ground is interpolated.
        assert_eq!(footholds.drop_position((150, 200), 225), (225, 190));
    }

    #[test]
    fn a_mob_against_a_wall_keeps_its_drops_out_of_it() {
        let footholds = cliff();
        // The mob stands at the foot of the wall; the drop aimed into the
        // cliff stays with the mob.
        assert_eq!(footholds.drop_position((0, 200), -25), (0, 200));
        assert_eq!(footholds.drop_position((12, 200), -12), (12, 200));
        // The other side is open.
        assert_eq!(footholds.drop_position((0, 200), 25), (25, 200));
    }

    #[test]
    fn drops_past_a_ledge_fall_to_the_ground_below() {
        let footholds = cliff();
        // From the upper ground's edge, past the wall's top, down the cliff.
        assert_eq!(footholds.drop_position((-10, 100), 15), (15, 200));
        // A wall standing up from the mob's own floor still blocks.
        let footholds = Footholds::new(vec![line(-100, 100, 0, 100), line(0, 100, 0, 40)]);
        assert_eq!(footholds.drop_position((-10, 100), 15), (-10, 100));
    }

    #[test]
    fn drops_past_the_end_of_the_ground_stay_with_the_mob() {
        let footholds = cliff();
        assert_eq!(footholds.drop_position((290, 182), 315), (290, 182));
    }

    #[test]
    fn a_mob_killed_in_the_air_drops_onto_the_ground() {
        let footholds = cliff();
        assert_eq!(footholds.drop_position((100, 150), 112), (112, 200));
        assert_eq!(footholds.drop_position((100, 150), 400), (100, 200));
    }

    #[test]
    fn maps_without_footholds_keep_the_old_placement() {
        let footholds = Footholds::default();
        assert_eq!(footholds.drop_position((100, 50), 125), (125, 50));
    }
}
