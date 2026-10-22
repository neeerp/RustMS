//! Maple TV messages (cash items 5075000-5075005, report 179 sections 1.6
//! and 6). The world shows one at a time, each for its item's time, the
//! rest waiting in line.

use crate::handler::BroadcastScope;
use game::AvatarLook;
use std::time::Duration;

/// A message waiting for, or on, the TV.
#[derive(Clone, Debug)]
pub struct MapleTv {
    pub item_id: i32,
    pub sender: String,
    pub look: AvatarLook,
    /// The one the message is for, whose look shows beside the sender's.
    pub receiver: Option<String>,
    pub lines: [String; 5],
    /// The sender's channel, or the world for the Mega kinds with the ear
    /// ticked.
    pub scope: BroadcastScope,
}

impl MapleTv {
    /// The screen it shows on (recv 341's type): 0 plain, 1 star, 2 heart,
    /// for the kinds in order and again for the Mega kinds.
    pub fn screen(&self) -> u8 {
        (self.item_id % 10 % 3) as u8
    }

    /// How long it stays up: the client has no timer of its own (report
    /// c056 section 5), so these are Cosmic's: 30 s for the Star
    /// Megassenger, 60 s for the Heart Megassenger, 15 s for the rest.
    pub fn shown_for(&self) -> Duration {
        Duration::from_secs(match self.item_id % 10 {
            4 => 30,
            5 => 60,
            _ => 15,
        })
    }
}
