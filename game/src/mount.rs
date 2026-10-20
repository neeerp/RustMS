//! A character's tamed monster for Monster Rider: its level, EXP and
//! tiredness. Riding it is a buff (`BuffStat::Riding`) on the mount worn
//! at `equip_slot::MOUNT`.

use std::time::Duration;

/// The EXP a mount needs to leave each level, by level; level 0 is never
/// used (Cosmic's `ExpTable.mount`). The EXP counts up across levels and is
/// never reset. The real v83 table isn't known.
pub const EXP_TABLE: [i32; 29] = [
    1, 24, 50, 105, 134, 196, 254, 263, 315, 367, 430, 543, 587, 679, 725, 897, 1146, 1394, 1701,
    2247, 2543, 2898, 3156, 3313, 3584, 3923, 4150, 4305, 4550,
];

/// The highest level, where Cosmic's table runs out.
pub const MAX_LEVEL: i32 = EXP_TABLE.len() as i32;

/// How tired a mount gets: past this, the ride ends.
pub const MAX_TIREDNESS: i32 = 99;

/// How often a ridden mount tires by one (Cosmic's MountTirednessTask).
pub const TIRE_INTERVAL: Duration = Duration::from_secs(60);

/// The skills that ride a 190xxxx mount: Monster Rider for explorers,
/// Cygnus Knights, Aran and Evan (v83 FUN_00966f7a).
pub const RIDER_SKILLS: [i32; 4] = [1004, 10001004, 20001004, 20011004];

/// How long a ride lasts: until the player gets off, as the client never
/// ends it (Cosmic gives it `Integer.MAX_VALUE` ms).
pub const RIDE_DURATION: Duration = Duration::from_millis(i32::MAX as u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mount {
    pub level: i32,
    pub exp: i32,
    pub tiredness: i32,
}

impl Default for Mount {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            tiredness: 0,
        }
    }
}

impl Mount {
    /// One `TIRE_INTERVAL` of riding. True when the mount has had enough:
    /// it stays at `MAX_TIREDNESS` and the ride ends.
    pub fn tire(&mut self) -> bool {
        self.tiredness += 1;
        if self.tiredness > MAX_TIREDNESS {
            self.tiredness = MAX_TIREDNESS;
            return true;
        }
        false
    }

    /// Feed it something that takes up to `relief` tiredness off, as a
    /// Revitalizer does. Each 30 taken earns `2 * level + 6` EXP, rounded
    /// up (Cosmic's UseMountFoodHandler). Returns the tiredness taken and
    /// whether it reached a new level.
    pub fn feed(&mut self, relief: i32) -> (i32, bool) {
        let taken = self.tiredness.min(relief).max(0);
        if taken == 0 {
            return (0, false);
        }
        self.tiredness -= taken;
        self.exp += (taken as f32 / 30.0 * (2 * self.level + 6) as f32).ceil() as i32;
        let level_up = self.level < MAX_LEVEL && self.exp >= EXP_TABLE[self.level as usize];
        if level_up {
            self.level += 1;
        }
        (taken, level_up)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tires_by_one_until_it_has_had_enough() {
        let mut mount = Mount {
            tiredness: 98,
            ..Mount::default()
        };
        assert!(!mount.tire());
        assert_eq!(mount.tiredness, 99);
        assert!(mount.tire());
        assert_eq!(mount.tiredness, MAX_TIREDNESS);
    }

    #[test]
    fn feeding_takes_tiredness_off_for_exp() {
        let mut mount = Mount {
            exp: 20,
            tiredness: 45,
            ..Mount::default()
        };
        // 30 off a level 1 mount: 8 EXP, past the 24 it needs.
        assert_eq!(mount.feed(30), (30, true));
        assert_eq!((mount.level, mount.exp, mount.tiredness), (2, 28, 15));
        // 15 off a level 2 mount: half of 10, short of 50.
        assert_eq!(mount.feed(30), (15, false));
        assert_eq!((mount.level, mount.exp, mount.tiredness), (2, 33, 0));
        assert_eq!(mount.feed(30), (0, false), "nothing to take off");
    }

    #[test]
    fn stops_at_the_last_level() {
        let mut mount = Mount {
            level: MAX_LEVEL,
            exp: 10_000,
            tiredness: 30,
        };
        assert_eq!(mount.feed(30), (30, false));
        assert_eq!(mount.level, MAX_LEVEL);
    }
}
