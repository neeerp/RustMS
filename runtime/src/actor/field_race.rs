//! A timed GM event on one of its maps, such as the Physical Fitness Test
//! (report b172): from the GM's `/start` its clock runs, its notices go
//! out, and when the time is up those still inside go out. Each of the
//! event's maps keeps its own copy, all from the same start. Pure state;
//! `FieldActor` sends the packets and the players off.

use net::gm_event::TimedEvent;
use std::collections::VecDeque;
use tokio::time::{Duration, Instant};

/// What the time did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// A notice goes out.
    Notice(&'static str),
    /// The time is up.
    Over,
}

#[derive(Debug)]
pub struct Race {
    notices: VecDeque<(Instant, &'static str)>,
    clock: Instant,
    end: Instant,
}

impl Race {
    /// `event` from `started`, its times `scale`d.
    pub fn new(event: &TimedEvent, started: Instant, scale: impl Fn(Duration) -> Duration) -> Self {
        Self {
            notices: event
                .notices
                .iter()
                .map(|&(at, key)| (started + scale(at), key))
                .collect(),
            clock: started + scale(event.length),
            end: started + scale(event.end()),
        }
    }

    /// The time left on the clock.
    pub fn left(&self, now: Instant) -> Duration {
        self.clock.saturating_duration_since(now)
    }

    /// The next thing due by `now`, if any: each notice in turn, then the
    /// end.
    pub fn tick(&mut self, now: Instant) -> Option<Turn> {
        if self.notices.front().is_some_and(|&(at, _)| at <= now) {
            return self.notices.pop_front().map(|(_, key)| Turn::Notice(key));
        }
        (self.end <= now).then_some(Turn::Over)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn the_notices_go_out_in_turn_and_then_the_time_is_up() {
        const EVENT: TimedEvent = TimedEvent {
            maps: &[1],
            length: Duration::from_secs(10),
            notices: &[(SECOND, "a"), (Duration::from_secs(5), "b")],
        };
        let now = Instant::now();
        let mut race = Race::new(&EVENT, now, |time| time);
        assert_eq!(race.left(now + 4 * SECOND), 6 * SECOND);
        assert_eq!(race.tick(now), None);
        assert_eq!(race.tick(now + SECOND), Some(Turn::Notice("a")));
        assert_eq!(race.tick(now + SECOND), None);
        // Both at once, if a tick came late.
        let late = now + 11 * SECOND;
        assert_eq!(race.tick(late), Some(Turn::Notice("b")));
        assert_eq!(race.left(late), Duration::ZERO);
        assert_eq!(race.tick(late), Some(Turn::Over));
    }

    #[test]
    fn a_scaled_race_is_shorter() {
        let now = Instant::now();
        let mut race = Race::new(&net::gm_event::FITNESS, now, |time| time / 100);
        assert_eq!(race.left(now), 9 * SECOND);
        let mut notices = 0;
        while let Some(Turn::Notice(_)) = race.tick(now + 9 * SECOND) {
            notices += 1;
        }
        assert_eq!(notices, net::gm_event::FITNESS.notices.len());
        assert_eq!(race.tick(now + 9 * SECOND), None);
        assert_eq!(
            race.tick(now + 9010 * Duration::from_millis(1)),
            Some(Turn::Over)
        );
    }
}
