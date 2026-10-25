//! A character's standing at the Mu Lung Dojo: the training points earned
//! on its floors and the score recorded with So Gong (Cosmic's
//! `dojoPoints`, `lastDojoStage`, `finishedDojoTutorial`,
//! `vanquisherStage` and `vanquisherKills`).

/// No floor earns more once the points reach this, the last belt's price.
pub const MOST_POINTS: i32 = 17_000;

/// So Gong's belts, white to black, each for a challenger over its level
/// with its price in training points, and the belt before it to hand in
/// (Cosmic `2091005`).
pub const BELTS: [i32; 5] = [1132000, 1132001, 1132002, 1132003, 1132004];
pub const BELT_LEVELS: [u8; 5] = [25, 35, 45, 60, 75];
pub const BELT_PRICES: [i32; 5] = [200, 1800, 4000, 9200, 17000];

/// The Dojo's rounds, one boss each; medal `n` (from 1) is for beating
/// round `n`'s boss this many times, and the last, for Mu Gong's, is the
/// Mu Lung Dojo Vanquisher Medal.
pub const ROUNDS: i32 = 32;
pub const MEDAL_KILLS: i32 = 100;
const FIRST_MEDAL: i32 = 1142033;

/// The medal for round `stage`'s boss (Mano's for 1).
pub fn medal(stage: i32) -> i32 {
    FIRST_MEDAL + stage - 1
}

/// The energy a challenger builds on the Dojo's maps (Cosmic's
/// `dojoEnergy`). Once it is full, it pays for one of the Beginner's secret
/// skills, which empties it.
pub const FULL_ENERGY: i32 = 10_000;
/// What each attack that reaches a monster adds, and what each hit taken
/// adds (Cosmic's `DOJO_ENERGY_ATK` and `DOJO_ENERGY_DMG`).
pub const ATTACK_ENERGY: i32 = 100;
pub const HIT_ENERGY: i32 = 20;

/// Whether `map_id` is one of the Dojo's maps, from its entrance to the
/// last party floor (Cosmic `MapId.isDojo`).
pub fn in_dojo(map_id: i32) -> bool {
    (925_020_000..=925_033_804).contains(&map_id)
}

/// The energy after `more` is added to `energy`, capped at `FULL_ENERGY`.
pub fn charged(energy: i32, more: i32) -> i32 {
    (energy + more).min(FULL_ENERGY)
}

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
    /// The points leaving floor `floor` earns: one, one more for each rest
    /// floor up to it, and one if `alone` (Cosmic `addDojoPointsByMap`).
    /// None once the points reach `MOST_POINTS`.
    pub fn floor_points(&self, floor: u8, alone: bool) -> i32 {
        if self.points >= MOST_POINTS {
            return 0;
        }
        1 + i32::from(floor / 6) + i32::from(alone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_section_of_floors_earns_a_point_more_until_the_last_belts_price() {
        let record = DojoRecord::default();
        let points: Vec<_> = [1, 5, 6, 7, 12, 36, 37]
            .map(|floor| record.floor_points(floor, true))
            .into();
        assert_eq!(points, [2, 2, 3, 3, 4, 8, 8]);
        assert_eq!(record.floor_points(7, false), 2);
        let full = DojoRecord {
            points: MOST_POINTS,
            ..record
        };
        assert_eq!(full.floor_points(1, true), 0);
    }

    #[test]
    fn energy_fills_on_the_dojos_maps_up_to_full() {
        assert!(in_dojo(925020000) && in_dojo(925020010) && in_dojo(925033804));
        assert!(!in_dojo(925010000) && !in_dojo(925033805));
        assert_eq!(charged(0, ATTACK_ENERGY), 100);
        assert_eq!(charged(9_990, HIT_ENERGY), FULL_ENERGY);
        assert_eq!(charged(FULL_ENERGY, ATTACK_ENERGY), FULL_ENERGY);
    }

    #[test]
    fn a_medal_for_each_rounds_boss_ending_with_the_dojos_own() {
        assert_eq!(medal(1), 1142033);
        assert_eq!(medal(ROUNDS), 1142064);
    }
}
