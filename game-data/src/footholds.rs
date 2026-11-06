//! A map's footholds: the lines that players, mobs, and drops stand on.

/// How far above a drop's start the ground below it is searched from, as
/// v83 servers place drops.
const DROP_SEARCH_HEIGHT: i16 = 50;
/// How far a floor may be from a point's height and still count as the
/// floor that point stands on.
const FLOOR_TOLERANCE: i32 = 5;

/// How far a spawn may sit off the foothold it names and still stand on it.
const STOOD_ON_TOLERANCE: i32 = 10;

/// One foothold line, `Map.nx` `foothold/<layer>/<group>/<id>`. Lines with
/// the same x at both ends are walls; the rest are floors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Foothold {
    /// Its `<id>`, by which packets name it.
    pub id: i16,
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

    /// The map's bounds (`bounds`) inside its view range, for a map with
    /// `info/VRLimit` (`FieldTemplate::view_limit`), as the v83 client
    /// narrows them (r25 0xa44c3a): the left edge to 20 px inside the view's,
    /// the right to 20 px inside, the top to 65 px below and the bottom to
    /// the view's bottom, each only inward and only when the view's edge is
    /// not 0.
    pub fn bounds_within(
        &self,
        view: Option<(i32, i32, i32, i32)>,
    ) -> Option<(i32, i32, i32, i32)> {
        let (mut left, mut top, mut right, mut bottom) = self.bounds()?;
        if let Some((view_left, view_top, view_right, view_bottom)) = view {
            if view_left != 0 {
                left = left.max(view_left + 20);
            }
            if view_top != 0 {
                top = top.max(view_top + 65);
            }
            if view_right != 0 {
                right = right.min(view_right - 20);
            }
            if view_bottom != 0 {
                bottom = bottom.min(view_bottom);
            }
        }
        Some((left, top, right, bottom))
    }

    /// The highest ground at `x` that is at or below `y`.
    pub fn ground_below(&self, x: i16, y: i16) -> Option<i16> {
        self.below(x, y).map(|(ground, _)| ground)
    }

    /// The highest ground at `x` that is at or below `y`, and the id of
    /// the foothold it is on.
    pub fn below(&self, x: i16, y: i16) -> Option<(i16, i16)> {
        self.lines
            .iter()
            .filter_map(|fh| Some((fh.ground_at(x)?, fh.id)))
            .filter(|&(ground, _)| ground >= i32::from(y))
            .min()
            .and_then(|(ground, id)| Some((i16::try_from(ground).ok()?, id)))
    }

    /// The foothold a spawn at (`x`, `y`) stands on: `named`, unless that
    /// is not the floor there, then the ground below it, or 0 with none.
    /// Some Carnival fields name footholds on far platforms, and the client
    /// would stand a mob on one wherever it is.
    pub fn stood_on(&self, named: i16, x: i16, y: i16) -> i16 {
        let on_it = self.lines.iter().filter(|fh| fh.id == named).any(|fh| {
            let near = x.clamp(fh.x1.min(fh.x2), fh.x1.max(fh.x2));
            (i32::from(near) - i32::from(x)).abs() <= STOOD_ON_TOLERANCE
                && fh
                    .ground_at(near)
                    .is_some_and(|ground| (ground - i32::from(y)).abs() <= STOOD_ON_TOLERANCE)
        });
        if on_it {
            return named;
        }
        self.below(x, y).map_or(0, |(_, id)| id)
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

    /// The point on a floor nearest `from`.
    fn nearest_floor(&self, from: (i16, i16)) -> Option<(i16, i16)> {
        let (fx, fy) = from;
        self.lines
            .iter()
            .filter_map(|fh| {
                let x = fx.clamp(fh.x1.min(fh.x2), fh.x1.max(fh.x2));
                let y = fh.ground_at(x)?;
                let dx = i64::from(x) - i64::from(fx);
                let dy = i64::from(y) - i64::from(fy);
                Some((dx * dx + dy * dy, x, i16::try_from(y).ok()?))
            })
            .min()
            .map(|(_, x, y)| (x, y))
    }

    /// Where a drop thrown from `from` toward `x` comes to rest: on the
    /// ground below `x`, or below `from` itself when no ground is there or a
    /// wall is in the way. With no ground below `from` either, on the nearest
    /// floor: a flier can die in the rock along a wall or past the ends of the
    /// floors, where nobody could reach what it leaves (the Amorian Challenge
    /// needs the Geist Fang from its last boss). Without footholds the drop
    /// keeps `from`'s height.
    pub fn drop_position(&self, from: (i16, i16), x: i16) -> (i16, i16) {
        if self.is_empty() {
            return (x, from.1);
        }
        let search = from.1.saturating_sub(DROP_SEARCH_HEIGHT);
        let fallback = self
            .ground_below(from.0, search)
            .map(|y| (from.0, y))
            .or_else(|| self.nearest_floor(from))
            .unwrap_or(from);
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
        Foothold {
            x1,
            y1,
            x2,
            y2,
            ..Foothold::default()
        }
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
    fn a_spawn_stands_on_the_foothold_it_names_only_if_that_is_there() {
        let mut lines = vec![
            line(-300, 100, 0, 100),
            line(0, 100, 0, 200),
            line(0, 200, 300, 200),
        ];
        for (id, fh) in lines.iter_mut().enumerate() {
            fh.id = id as i16 + 1;
        }
        let map = Footholds::new(lines);
        // Its own floor, and a few px off its end or height.
        assert_eq!(map.stood_on(1, -100, 100), 1);
        assert_eq!(map.stood_on(1, 8, 96), 1);
        // A foothold far away, a wall, and one not on the map at all.
        assert_eq!(map.stood_on(1, 200, 200), 3);
        assert_eq!(map.stood_on(2, 200, 200), 3);
        assert_eq!(map.stood_on(9, -100, 100), 1);
        // No ground under it: the client drops it.
        assert_eq!(map.stood_on(1, 400, 200), 0);
    }

    #[test]
    fn bounds_take_the_client_margins_around_every_line() {
        // The cliff spans x -300..300 and y 100..200, its wall included.
        assert_eq!(cliff().bounds(), Some((-270, -200, 270, 210)));
        assert_eq!(Footholds::default().bounds(), None);
    }

    #[test]
    fn a_view_range_only_pulls_the_bounds_in() {
        let cliff = cliff();
        assert_eq!(cliff.bounds_within(None), cliff.bounds());
        // Left and top move in; the right and bottom view edges lie outside.
        assert_eq!(
            cliff.bounds_within(Some((-200, -300, 400, 500))),
            Some((-180, -200, 270, 210))
        );
        // Right and bottom move in; an edge of 0 is left alone.
        assert_eq!(
            cliff.bounds_within(Some((0, 0, 200, 150))),
            Some((-270, -200, 180, 150))
        );
        assert_eq!(Footholds::default().bounds_within(Some((1, 1, 1, 1))), None);
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
    fn a_flier_killed_over_nothing_drops_onto_the_nearest_floor() {
        let footholds = cliff();
        // Past the end of the ground.
        assert_eq!(footholds.drop_position((400, 190), 400), (300, 180));
        // Under the upper ground, in the rock behind the cliff.
        assert_eq!(footholds.drop_position((-100, 300), -100), (0, 200));
        assert_eq!(footholds.drop_position((-100, 130), -100), (-100, 100));
        // Spread past it, with the ground no nearer.
        assert_eq!(footholds.drop_position((400, 190), 425), (300, 180));
    }

    #[test]
    fn maps_without_footholds_keep_the_old_placement() {
        let footholds = Footholds::default();
        assert_eq!(footholds.drop_position((100, 50), 125), (125, 50));
    }
}
