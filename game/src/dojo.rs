//! A character's standing at the Mu Lung Dojo: the training points earned
//! on its floors and the score recorded with So Gong (Cosmic's
//! `dojoPoints`, `lastDojoStage`, `finishedDojoTutorial`,
//! `vanquisherStage` and `vanquisherKills`).

/// No floor earns more once the points reach this, the last belt's price.
pub const MOST_POINTS: i32 = 17_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DojoRecord {
    /// Training points, spent on belts.
    pub points: i32,
    /// The rest floor whose score So Gong recorded, to start after next
    /// time; 0 for none.
    pub stage: u8,
    /// Whether the challenger has beaten So Gong's first-time trial.
    pub tutorial_done: bool,
    /// The medal tried for, from 1, and the Dojo monsters beaten toward it.
    pub vanquisher_stage: i32,
    pub vanquisher_kills: i32,
}

impl DojoRecord {
    /// The points leaving solo floor `floor` earns: one, one more for each
    /// rest floor up to it, and one for going alone (Cosmic
    /// `addDojoPointsByMap`). None once the points reach `MOST_POINTS`.
    pub fn floor_points(&self, floor: u8) -> i32 {
        if self.points >= MOST_POINTS {
            return 0;
        }
        2 + i32::from(floor / 6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_section_of_floors_earns_a_point_more_until_the_last_belts_price() {
        let record = DojoRecord::default();
        let points: Vec<_> = [1, 5, 6, 7, 12, 36, 37]
            .map(|floor| record.floor_points(floor))
            .into();
        assert_eq!(points, [2, 2, 3, 3, 4, 8, 8]);
        let full = DojoRecord {
            points: MOST_POINTS,
            ..record
        };
        assert_eq!(full.floor_points(1), 0);
    }
}
