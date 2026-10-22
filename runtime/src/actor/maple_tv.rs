//! The Maple TV's line (report 179 section 6). The world directory shows
//! one message at a time, each for its item's time, then clears the screen
//! (recv 342) and shows the next. A message that would wait over an hour
//! is refused.

use game::AvatarLook;
use net::handler::BroadcastScope;
use net::maple_tv::MapleTv;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Past this wait a message is refused: "The waiting line is longer than
/// an hour." (StringPool 0xf76).
const LONGEST_WAIT: Duration = Duration::from_secs(60 * 60);

/// A message with the receiver's look, if it names one.
#[derive(Clone, Debug)]
pub struct OnAir {
    pub tv: MapleTv,
    pub partner: Option<AvatarLook>,
}

/// The message up now.
struct Up {
    shown: OnAir,
    until: Instant,
}

#[derive(Default)]
pub struct MapleTvLine {
    up: Option<Up>,
    waiting: VecDeque<OnAir>,
}

impl MapleTvLine {
    /// Put `message` in line: `Ok(true)` when it goes up now, `Ok(false)`
    /// when it waits, `Err(())` when the wait would be over an hour.
    pub fn queue(&mut self, message: OnAir, now: Instant) -> Result<bool, ()> {
        if self.up.is_none() {
            self.up = Some(Up {
                until: now + message.tv.shown_for(),
                shown: message,
            });
            return Ok(true);
        }
        if self.wait(now) > LONGEST_WAIT {
            return Err(());
        }
        self.waiting.push_back(message);
        Ok(false)
    }

    /// How long until the line is clear.
    pub fn wait(&self, now: Instant) -> Duration {
        let up = self
            .up
            .as_ref()
            .map_or(Duration::ZERO, |up| up.until.saturating_duration_since(now));
        up + self
            .waiting
            .iter()
            .map(|m| m.tv.shown_for())
            .sum::<Duration>()
    }

    /// The message up now, if any.
    pub fn up(&self) -> Option<&OnAir> {
        self.up.as_ref().map(|up| &up.shown)
    }

    /// When the message up now has had its time.
    pub fn until(&self) -> Option<Instant> {
        self.up.as_ref().map(|up| up.until)
    }

    /// The message up has had its time: where to clear it, and the next
    /// one, which is up now.
    pub fn next(&mut self, now: Instant) -> (Option<BroadcastScope>, Option<&OnAir>) {
        let ended = self.up.take().map(|up| up.shown.tv.scope);
        self.up = self.waiting.pop_front().map(|shown| Up {
            until: now + shown.tv.shown_for(),
            shown,
        });
        (ended, self.up())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(item_id: i32) -> OnAir {
        OnAir {
            tv: MapleTv {
                item_id,
                sender: "Tester".into(),
                look: AvatarLook {
                    gender: 0,
                    skin: 0,
                    face: 20000,
                    hair: 30000,
                    visible: Default::default(),
                    masked: Default::default(),
                    cash_weapon: None,
                    pets: [0; 3],
                },
                receiver: None,
                lines: Default::default(),
                scope: BroadcastScope::Channel(0),
            },
            partner: None,
        }
    }

    #[test]
    fn the_first_message_goes_up_and_the_next_waits_its_turn() {
        let mut line = MapleTvLine::default();
        let start = Instant::now();
        assert_eq!(line.queue(message(5075000), start), Ok(true));
        assert_eq!(line.until(), Some(start + Duration::from_secs(15)));
        assert_eq!(line.queue(message(5075005), start), Ok(false));
        assert_eq!(line.wait(start), Duration::from_secs(75));
        let later = start + Duration::from_secs(15);
        let (ended, up) = line.next(later);
        assert_eq!(ended, Some(BroadcastScope::Channel(0)));
        assert_eq!(up.unwrap().tv.item_id, 5075005);
        assert_eq!(line.until(), Some(later + Duration::from_secs(60)));
        let (ended, up) = line.next(later + Duration::from_secs(60));
        assert!(ended.is_some() && up.is_none());
        assert_eq!(line.until(), None);
        assert_eq!(line.wait(later), Duration::ZERO);
    }

    #[test]
    fn a_wait_over_an_hour_is_refused() {
        let mut line = MapleTvLine::default();
        let start = Instant::now();
        // 60 Heart Megassengers are an hour.
        for _ in 0..60 {
            assert!(line.queue(message(5075005), start).is_ok());
        }
        assert_eq!(line.wait(start), Duration::from_secs(60 * 60));
        assert_eq!(line.queue(message(5075001), start), Ok(false));
        assert_eq!(line.queue(message(5075001), start), Err(()));
    }
}
