//! The chandelier in Ergoth's room (990000900, field type 8, report b380):
//! a hit on the pulley drops it from `yMin` to `yMax`, where it lands on
//! the mobs below, then it is wound back up and the pulley frees. Pure
//! state; `FieldActor` sends the packets and deals the damage.

use game_data::GuildBoss;
use net::ClientId;
use std::ops::RangeInclusive;
use tokio::time::{Duration, Instant};

/// How often the chandelier moves `fall` or `rise` pixels (QUESTIONS b379:
/// the map gives speeds without a unit).
pub const STEP: Duration = Duration::from_millis(100);
/// How far the chandelier's picture reaches left and right of its `x`
/// (`Obj/guild.img/syarenian/boss/0`: 249 wide, its origin at 124).
const LEFT: i32 = 124;
const RIGHT: i32 = 125;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Up,
    Falling,
    Rising,
}

/// What a tick did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swing {
    /// The chandelier is at `y` now.
    Moved(i16),
    /// It landed at `y`, on the mobs below, for `by`, who pulled it.
    Landed { y: i16, by: ClientId },
    /// It is back up at `y`, and the pulley frees.
    Freed(i16),
}

#[derive(Clone, Debug)]
pub struct Chandelier {
    data: GuildBoss,
    y: i32,
    phase: Phase,
    next: Instant,
    by: ClientId,
}

impl Chandelier {
    pub fn new(data: GuildBoss) -> Self {
        Self {
            y: data.y_min,
            data,
            phase: Phase::Up,
            next: Instant::now(),
            by: 0,
        }
    }

    /// Where it hangs.
    pub fn y(&self) -> i16 {
        self.y as i16
    }

    /// Whether it hangs anywhere but at the top, where a newcomer's client
    /// puts it.
    pub fn lowered(&self) -> bool {
        self.y != self.data.y_min
    }

    /// Whether the pulley is locked, the chandelier being away.
    pub fn locked(&self) -> bool {
        self.phase != Phase::Up
    }

    /// `from` pulled the pulley: the chandelier starts falling, unless it is
    /// already away.
    pub fn pull(&mut self, from: ClientId, now: Instant) -> bool {
        if self.locked() {
            return false;
        }
        self.phase = Phase::Falling;
        self.next = now + STEP;
        self.by = from;
        true
    }

    /// The x the chandelier covers where it lands.
    pub fn span(&self) -> RangeInclusive<i32> {
        self.data.healer_x - LEFT..=self.data.healer_x + RIGHT
    }

    /// The damage it deals to each mob it lands on.
    pub fn damage(&self) -> RangeInclusive<i32> {
        self.data.heal_min..=self.data.heal_max.max(self.data.heal_min)
    }

    /// Moves it as far as the time says, at most once to a landing or
    /// freeing, which `tick` reports over a plain move.
    pub fn tick(&mut self, now: Instant) -> Option<Swing> {
        let mut moved = None;
        while self.phase != Phase::Up && self.next <= now {
            self.next += STEP;
            match self.phase {
                Phase::Falling => {
                    self.y = (self.y + self.data.fall.max(1)).min(self.data.y_max);
                    if self.y >= self.data.y_max {
                        self.phase = Phase::Rising;
                        return Some(Swing::Landed {
                            y: self.y(),
                            by: self.by,
                        });
                    }
                }
                Phase::Rising => {
                    self.y = (self.y - self.data.rise.max(1)).max(self.data.y_min);
                    if self.y <= self.data.y_min {
                        self.phase = Phase::Up;
                        return Some(Swing::Freed(self.y()));
                    }
                }
                Phase::Up => {}
            }
            moved = Some(Swing::Moved(self.y()));
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> GuildBoss {
        GuildBoss {
            healer_x: 295,
            y_min: -564,
            y_max: -350,
            rise: 1,
            fall: 10,
            heal_min: 1000,
            heal_max: 2000,
            pulley_x: -59,
            pulley_y: -394,
        }
    }

    #[test]
    fn a_pull_drops_it_onto_the_mobs_then_winds_it_back_up() {
        let now = Instant::now();
        let mut chandelier = Chandelier::new(room());
        assert_eq!((chandelier.y(), chandelier.locked()), (-564, false));
        assert_eq!(chandelier.tick(now + STEP), None);

        assert!(chandelier.pull(7, now));
        assert!(!chandelier.pull(8, now), "the pulley locks");
        assert_eq!(chandelier.tick(now), None);
        assert_eq!(chandelier.tick(now + STEP), Some(Swing::Moved(-554)));
        assert!(chandelier.lowered());
        // A late tick catches up.
        assert_eq!(chandelier.tick(now + STEP * 3), Some(Swing::Moved(-534)));

        // 214 pixels fall in 22 steps, the last one short.
        let landed = chandelier.tick(now + STEP * 22);
        assert_eq!(landed, Some(Swing::Landed { y: -350, by: 7 }));
        assert!(chandelier.locked());
        assert_eq!(chandelier.tick(now + STEP * 23), Some(Swing::Moved(-351)));

        // And 214 rise a pixel at a time.
        let freed = chandelier.tick(now + STEP * (22 + 214));
        assert_eq!(freed, Some(Swing::Freed(-564)));
        assert!(!chandelier.locked() && !chandelier.lowered());
        assert_eq!(chandelier.tick(now + STEP * 300), None);
        assert!(chandelier.pull(8, now + STEP * 300));
    }

    #[test]
    fn it_covers_its_picture_and_hits_for_its_heal() {
        let chandelier = Chandelier::new(room());
        assert_eq!(chandelier.span(), 171..=420);
        assert!(chandelier.span().contains(&344), "Ergoth, below the altar");
        assert!(!chandelier.span().contains(&130) && !chandelier.span().contains(&540));
        assert_eq!(chandelier.damage(), 1000..=2000);
    }
}
