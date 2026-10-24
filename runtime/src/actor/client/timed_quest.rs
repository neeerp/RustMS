//! Quests with a time limit (`QuestInfo` `timeLimit`): the client shows a
//! clock from the start, and when it runs out the quest is forfeited. The
//! time counts only while the player stays in the game: leaving it, or going
//! to the Cash Shop, gives the quest up (report b149).

use super::quest::forfeit;
use super::ClientActor;
use crate::error::RuntimeError;
use game::Player;
use net::packet::build::world::quest;
use net::quests;
use packet::Packet;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::time::Instant;
use tracing::warn;

/// The clocks of players changing channel, by character, for the channel
/// they arrive on. They are kept with the session, so only the login that
/// carries it on picks them up.
#[derive(Default)]
pub struct QuestClocks(Mutex<HashMap<i32, (i32, Vec<(i16, SystemTime)>)>>);

impl QuestClocks {
    fn hand_over(&self, character_id: i32, session_id: i32, clocks: Vec<(i16, SystemTime)>) {
        let mut waiting = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if clocks.is_empty() {
            waiting.remove(&character_id);
        } else {
            waiting.insert(character_id, (session_id, clocks));
        }
    }

    fn take(&self, character_id: i32, session_id: Option<i32>) -> Vec<(i16, SystemTime)> {
        let mut waiting = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match waiting.remove(&character_id) {
            Some((session, clocks)) if Some(session) == session_id => clocks,
            _ => Vec::new(),
        }
    }
}

/// A started quest's clock: when its time is up.
pub(super) struct QuestClock {
    quest_id: i16,
    ends: Instant,
}

/// The seconds `quest_id` gives, if it is timed.
fn time_limit(quest_id: i16) -> Option<u32> {
    quests::catalog().ok()?.quest(quest_id)?.time_limit
}

impl ClientActor {
    /// The quest just started: its clock starts too, shown first.
    pub(super) fn start_quest_clock(
        &mut self,
        quest_id: i16,
        packets: &mut Vec<Packet>,
    ) -> Result<(), RuntimeError> {
        let Some(seconds) = time_limit(quest_id) else {
            return Ok(());
        };
        self.stop_quest_clock(quest_id);
        // Shortened with the events' times for testing.
        let limit = net::events::scaled(Duration::from_secs(u64::from(seconds)));
        let ms = i32::try_from(limit.as_millis()).unwrap_or(i32::MAX);
        packets.insert(0, quest::build_quest_timer(quest_id, ms)?);
        self.quest_clocks.push(QuestClock {
            quest_id,
            ends: Instant::now() + limit,
        });
        Ok(())
    }

    /// The quest is done or given up: its clock goes from the screen first.
    pub(super) fn end_quest_clock(
        &mut self,
        quest_id: i16,
        packets: &mut Vec<Packet>,
    ) -> Result<(), RuntimeError> {
        if self.stop_quest_clock(quest_id) {
            packets.insert(0, quest::build_quest_timer_end(quest_id)?);
        }
        Ok(())
    }

    fn stop_quest_clock(&mut self, quest_id: i16) -> bool {
        let before = self.quest_clocks.len();
        self.quest_clocks.retain(|clock| clock.quest_id != quest_id);
        self.quest_clocks.len() != before
    }

    /// When the next clock runs out.
    pub(super) fn quest_clocks_due(&self) -> Option<Instant> {
        self.quest_clocks.iter().map(|clock| clock.ends).min()
    }

    /// Forfeit the quests whose time is up: the clock goes, the quest log
    /// drops them, and the chat log says why.
    pub(super) async fn expire_quests(&mut self) -> Result<(), RuntimeError> {
        let now = Instant::now();
        let expired: Vec<i16> = self
            .quest_clocks
            .iter()
            .filter(|clock| clock.ends <= now)
            .map(|clock| clock.quest_id)
            .collect();
        for quest_id in expired {
            let mut packets = match self.drop_quest(quest_id).await {
                Ok(packets) => packets,
                Err(error) => {
                    // Not again at every wake-up; the quest stays started.
                    warn!(self.client_id, quest_id, %error, "Failed to expire a timed quest");
                    self.stop_quest_clock(quest_id);
                    continue;
                }
            };
            if !packets.is_empty() {
                packets.push(quest::build_quest_expired(quest_id)?);
            }
            self.end_quest_clock(quest_id, &mut packets)?;
            self.send_all(&mut packets).await?;
        }
        Ok(())
    }

    /// Off to the Cash Shop: the timed quests are given up, as v83's
    /// servers do, and their clocks go.
    pub(super) async fn give_up_timed_quests(&mut self) -> Result<(), RuntimeError> {
        let running: Vec<i16> = self.quest_clocks.iter().map(|c| c.quest_id).collect();
        for quest_id in running {
            let mut packets = match self.drop_quest(quest_id).await {
                Ok(packets) => packets,
                Err(error) => {
                    warn!(self.client_id, quest_id, %error, "Failed to give up a timed quest");
                    Vec::new()
                }
            };
            self.end_quest_clock(quest_id, &mut packets)?;
            self.send_all(&mut packets).await?;
        }
        Ok(())
    }

    /// Changing channel: the clocks go with the session, handed over
    /// before the client is sent on, so the new channel finds them.
    pub(super) fn hand_over_quest_clocks(&mut self, session_id: i32) {
        let (now, wall) = (Instant::now(), SystemTime::now());
        let handed = self
            .quest_clocks
            .drain(..)
            .map(|clock| {
                (
                    clock.quest_id,
                    wall + clock.ends.saturating_duration_since(now),
                )
            })
            .collect();
        self.services
            .quest_clocks
            .hand_over(self.client_id, session_id, handed);
    }

    /// Leaving the game: the timed quests are given up, unless the player
    /// is only changing channel and their clocks went with them.
    pub(super) async fn leave_timed_quests(&mut self) {
        if self.changing_channel {
            return;
        }
        for clock in std::mem::take(&mut self.quest_clocks) {
            let Ok(game) = self.in_game_mut() else {
                return;
            };
            if let Err(error) = forfeit(&mut game.player, clock.quest_id).await {
                warn!(self.client_id, %error, "Failed to give up a timed quest");
            }
        }
    }

    /// Entering the game as `player`, before the client hears of them:
    /// timed quests started on another channel keep their clocks, which
    /// that client still shows; any other is given up. Returns what to tell
    /// the client once in the game: the clocks that ran out on the way.
    pub(super) async fn resume_timed_quests(
        &mut self,
        player: &mut Player,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let handed = self
            .services
            .quest_clocks
            .take(player.id(), self.session_id);
        let started: Vec<i16> = player
            .quests
            .started()
            .map(|record| record.quest_id)
            .filter(|&id| time_limit(id).is_some())
            .collect();
        let (now, wall) = (Instant::now(), SystemTime::now());
        let mut packets = Vec::new();
        for quest_id in started {
            let left = handed
                .iter()
                .find(|(id, _)| *id == quest_id)
                .map(|(_, ends)| ends.duration_since(wall).unwrap_or_default());
            match left {
                Some(left) if !left.is_zero() => self.quest_clocks.push(QuestClock {
                    quest_id,
                    ends: now + left,
                }),
                ran_out => {
                    forfeit(player, quest_id).await?;
                    if ran_out.is_some() {
                        packets.push(quest::build_quest_timer_end(quest_id)?);
                        packets.push(quest::build_quest_expired(quest_id)?);
                    }
                }
            }
        }
        Ok(packets)
    }
}
