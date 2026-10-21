//! Avatar megaphone banners (report c003). The v83 client shows one banner
//! at a time and replaces it the moment another arrives, so the world
//! directory, which reaches every channel, shows them in turn: each for
//! the 10 s the client keeps it up, the rest waiting in line.

use packet::Packet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long the client shows a banner (CWvsContext::Update hides it after
/// 10000 ms).
pub const SHOWN_FOR: Duration = Duration::from_secs(10);
/// Past this wait a banner is refused: "The waiting line is longer than 15
/// seconds." (StringPool 0xf84).
const LONGEST_WAIT: Duration = Duration::from_secs(15);

/// What became of a banner put in line.
#[derive(Debug)]
pub enum Queued {
    /// Nothing is up: show it to the world now.
    Now(Packet),
    /// It waits for those before it.
    Later,
    /// The wait would be too long.
    Refused,
}

#[derive(Default)]
pub struct AvatarBanners {
    /// When the banner up now has had its time, if one is up.
    until: Option<Instant>,
    waiting: VecDeque<Packet>,
}

impl AvatarBanners {
    pub fn queue(&mut self, banner: Packet, now: Instant) -> Queued {
        let Some(until) = self.until else {
            self.until = Some(now + SHOWN_FOR);
            return Queued::Now(banner);
        };
        let wait = until.saturating_duration_since(now) + SHOWN_FOR * self.waiting.len() as u32;
        if wait > LONGEST_WAIT {
            return Queued::Refused;
        }
        self.waiting.push_back(banner);
        Queued::Later
    }

    /// When the banner up now has had its time.
    pub fn until(&self) -> Option<Instant> {
        self.until
    }

    /// The banner up has had its time: the next one to show, if any.
    pub fn next(&mut self, now: Instant) -> Option<Packet> {
        let next = self.waiting.pop_front();
        self.until = next.is_some().then(|| now + SHOWN_FOR);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn banner(n: u8) -> Packet {
        Packet::new(&[n])
    }

    fn shown(queued: Queued) -> Vec<u8> {
        let Queued::Now(packet) = queued else {
            panic!("expected it shown at once: {queued:?}");
        };
        packet.bytes
    }

    #[test]
    fn the_first_banner_shows_at_once_and_the_next_waits_its_turn() {
        let mut banners = AvatarBanners::default();
        let start = Instant::now();
        assert_eq!(shown(banners.queue(banner(1), start)), [1]);
        assert_eq!(banners.until(), Some(start + SHOWN_FOR));
        assert!(matches!(banners.queue(banner(2), start), Queued::Later));
        let later = start + SHOWN_FOR;
        assert_eq!(banners.next(later).unwrap().bytes, [2]);
        assert_eq!(banners.until(), Some(later + SHOWN_FOR));
        assert!(banners.next(later + SHOWN_FOR).is_none());
        assert_eq!(banners.until(), None);
        assert_eq!(shown(banners.queue(banner(3), later)), [3]);
    }

    #[test]
    fn a_wait_past_15_seconds_is_refused() {
        let mut banners = AvatarBanners::default();
        let start = Instant::now();
        banners.queue(banner(1), start);
        // 10 s for the one up and 10 s for the one waiting.
        assert!(matches!(banners.queue(banner(2), start), Queued::Later));
        assert!(matches!(banners.queue(banner(3), start), Queued::Refused));
        // With 5 s of the first left the wait is just 15 s.
        let later = start + Duration::from_secs(5);
        assert!(matches!(banners.queue(banner(3), later), Queued::Later));
    }
}
