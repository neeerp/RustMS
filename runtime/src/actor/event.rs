//! One party's run through an event in its own copy of the event's maps
//! (`net::events`). The directory picks the team and reserves the event on
//! the channel; this actor gives each member a ticket into the copy, keeps
//! the time, and when the event ends sends those still in it out, closes
//! the copy's fields and frees the event for the next party.
//!
//! Members' client actors do the moving: a ticket makes `change_field` use
//! the copy's fields for the event's maps, and leaving those maps, the
//! party or the game hands the ticket back (`EventMessage::Left`). A ticket
//! dropped any other way, say with a client that went away before reading
//! it, hands itself back.
//!
//! The stages cleared are shared with the tickets, so a member's client
//! actor can tell at once whether a stage's way on is open. Clearing the
//! last stage wins the event: its clock stops, and it goes on until the
//! last member leaves its maps.

use crate::actor::registry::FieldRegistry;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use net::events::{
    draw_combo, draw_question, Cloto, EventCommand, EventKind, Puzzles, Stage1, Team,
};
use net::ClientId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, sleep_until, Instant};
use tracing::{debug, info};

/// How long members have to get out after the event ends before its fields
/// close.
const GRACE: Duration = Duration::from_secs(2);
const EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug)]
pub enum EventMessage {
    /// `client_id` left the event's maps, the party, or the game.
    Left { client_id: ClientId },
    /// `client_id` talks to Cloto on `stage`; she has nothing to say on a
    /// stage without a part for her.
    Cloto {
        client_id: ClientId,
        stage: u8,
        reply: oneshot::Sender<Option<Cloto>>,
    },
    /// `client_id`'s script did something the event keeps track of.
    Command {
        client_id: ClientId,
        command: EventCommand,
    },
}

/// The stages an event's party has cleared, one bit each. Only the event
/// writes it.
#[derive(Debug, Default)]
pub struct Cleared(AtomicU8);

impl Cleared {
    pub fn has(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.0.load(Ordering::Acquire) & bit != 0)
    }

    /// Returns whether `stage` was not cleared before.
    fn set(&self, stage: u8) -> bool {
        bit(stage).is_some_and(|bit| self.0.fetch_or(bit, Ordering::Release) & bit == 0)
    }
}

fn bit(stage: u8) -> Option<u8> {
    stage
        .checked_sub(1)
        .and_then(|shift| 1u8.checked_shl(shift.into()))
}

/// A way to reach one running event.
#[derive(Clone, Debug)]
pub struct EventHandle {
    tx: mpsc::Sender<EventMessage>,
}

/// What a member holds while in the event: the event's own fields for its
/// maps, and when it ends. Dropping it leaves the event.
#[derive(Debug)]
pub struct EventTicket {
    pub id: u32,
    pub kind: EventKind,
    pub ends: Instant,
    member: ClientId,
    event: EventHandle,
    cleared: Arc<Cleared>,
}

impl Drop for EventTicket {
    fn drop(&mut self) {
        // Each member has one ticket, so the queue can't fill.
        let _ = self.event.tx.try_send(EventMessage::Left {
            client_id: self.member,
        });
    }
}

impl EventTicket {
    /// Whether `map_id` is one of the event's maps, played in its copy.
    pub fn holds(&self, map_id: i32) -> bool {
        self.kind.event().holds(map_id)
    }

    /// Whether the party has cleared `stage`.
    pub fn cleared(&self, stage: u8) -> bool {
        self.cleared.has(stage)
    }

    /// Whether the party has cleared the last stage, so has no time limit.
    pub fn won(&self) -> bool {
        self.cleared(self.kind.event().last_stage())
    }

    /// Where the member stands with Cloto on `stage`, or `None` if she has
    /// nothing to say there or the event is over.
    pub async fn cloto(&self, stage: u8) -> Option<Cloto> {
        let (reply, view) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Cloto {
                client_id: self.member,
                stage,
                reply,
            })
            .await
            .ok()?;
        view.await.ok().flatten()
    }

    /// Tell the event what the member's script did.
    pub async fn command(&self, command: EventCommand) {
        let _ = self
            .event
            .tx
            .send(EventMessage::Command {
                client_id: self.member,
                command,
            })
            .await;
    }
}

/// An event the directory reserved for a party: `id` on `channel_id`, with
/// the members who go.
#[derive(Debug)]
pub struct EventStart {
    pub id: u32,
    pub kind: EventKind,
    pub channel_id: u8,
    pub leader: ClientId,
    pub team: Vec<ClientHandle>,
}

/// Start the event's actor, which hands out the tickets.
pub fn spawn(
    start: EventStart,
    fields: Arc<FieldRegistry>,
    directory: mpsc::Sender<DirectoryMessage>,
) {
    let (id, kind, channel_id) = (start.id, start.kind, start.channel_id);
    fields.open_instance(channel_id, id);
    let (tx, rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
    let everyone = start.team.clone();
    let actor = EventActor {
        id,
        kind,
        ends: Some(Instant::now() + kind.event().duration()),
        event: Some(EventHandle { tx }),
        rx,
        team: Team::new(
            kind.event(),
            start.leader,
            start.team.iter().map(|m| m.client_id).collect(),
        ),
        members: start
            .team
            .into_iter()
            .map(|member| (member.client_id, member))
            .collect(),
        stage1: Stage1::default(),
        puzzles: Puzzles::default(),
        cleared: Arc::default(),
    };
    // However it ends, even by a panic, its members are sent out, the copy
    // closes and the next party may go.
    spawn_supervised(format!("event {kind:?} {id}"), actor.run(), move || {
        for member in &everyone {
            member.send(ServerMessage::EventEnded { id });
        }
        fields.close_instance(channel_id, id);
        tokio::spawn(async move {
            let _ = directory
                .send(DirectoryMessage::EventClosed {
                    channel_id,
                    kind,
                    id,
                })
                .await;
        });
    });
}

struct EventActor {
    id: u32,
    kind: EventKind,
    /// None once the event is won.
    ends: Option<Instant>,
    /// Ours until the tickets are out; then only they keep the queue open.
    event: Option<EventHandle>,
    rx: mpsc::Receiver<EventMessage>,
    team: Team,
    members: HashMap<ClientId, ClientHandle>,
    stage1: Stage1,
    puzzles: Puzzles,
    cleared: Arc<Cleared>,
}

impl EventActor {
    async fn run(mut self) {
        let (id, kind) = (self.id, self.kind);
        info!(?kind, id, members = ?self.team.members(), "Event started");
        if let (Some(event), Some(ends)) = (self.event.take(), self.ends) {
            for member in self.members.values() {
                // A ticket that doesn't arrive hands itself back.
                member.send(ServerMessage::EventStarted(EventTicket {
                    id,
                    kind,
                    ends,
                    member: member.client_id,
                    event: event.clone(),
                    cleared: self.cleared.clone(),
                }));
            }
        }
        let mut over = false;
        while !over {
            let ends = self.ends;
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(EventMessage::Left { client_id }) => over = self.team.leave(client_id),
                    Some(EventMessage::Cloto { client_id, stage, reply }) => {
                        let _ = reply.send(self.cloto(client_id, stage));
                    }
                    Some(EventMessage::Command { client_id, command }) => {
                        self.command(client_id, command);
                    }
                    None => over = true,
                },
                _ = until(ends) => over = true,
            }
        }
        info!(?kind, id, "Event ended");
        // Nothing is answered from here on: a member asking Cloto hears
        // nothing at once rather than after the grace.
        drop(self.rx);
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventEnded { id });
            }
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn cloto(&mut self, client_id: ClientId, stage: u8) -> Option<Cloto> {
        if !self.team.has(client_id) {
            return None;
        }
        let event = self.kind.event();
        if stage == 1 {
            return Some(self.stage1.talk(client_id, &self.team, draw_question));
        }
        if stage == event.last_stage() {
            let cleared = self.cleared.has(stage);
            return Some(event.last_stage_talk(client_id, &self.team, cleared));
        }
        if !event.puzzles.contains(&stage) {
            return None;
        }
        let cleared = self.cleared.has(stage);
        let draw = || {
            let map_id = event.stage_map(stage);
            let field = net::get_game_data().ok()?.field(map_id)?;
            draw_combo(field.areas.len())
        };
        Some(
            self.puzzles
                .talk(stage, client_id, &self.team, cleared, draw),
        )
    }

    fn command(&mut self, client_id: ClientId, command: EventCommand) {
        if !self.team.has(client_id) {
            return;
        }
        let leader = client_id == self.team.leader();
        let event = self.kind.event();
        let puzzles = &event.puzzles;
        match command {
            EventCommand::PassGiven => self.stage1.pass_given(client_id),
            EventCommand::ClearStage(1) if leader => {
                if self.stage1.clear() {
                    self.clear(1);
                }
            }
            EventCommand::ClearStage(stage)
                if leader && puzzles.contains(&stage) && self.puzzles.set(stage) =>
            {
                self.clear(stage);
            }
            EventCommand::ClearStage(stage) if leader && stage == event.last_stage() => {
                self.win();
            }
            EventCommand::ClearStage(stage) => debug!(stage, client_id, "Clear not taken"),
        }
    }

    /// The leader handed in the last stage's passes: the event is won, and
    /// its time no longer runs (Cosmic `clearPQ`).
    fn win(&mut self) {
        if !self.clear(self.kind.event().last_stage()) {
            return;
        }
        self.team.win();
        self.ends = None;
        info!(kind = ?self.kind, id = self.id, "Event won");
    }

    /// The way on from `stage` opens before anyone hears of the clear, so a
    /// member who sees it can go through. A stage clears once; returns
    /// whether this cleared it.
    fn clear(&self, stage: u8) -> bool {
        if !self.cleared.set(stage) {
            return false;
        }
        info!(kind = ?self.kind, id = self.id, stage, "Stage cleared");
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::StageCleared { id: self.id, stage });
            }
        }
        true
    }
}

/// Wait until `ends`, or forever without it.
async fn until(ends: Option<Instant>) {
    match ends {
        Some(ends) => sleep_until(ends).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleared_keeps_one_bit_per_stage() {
        let cleared = Cleared::default();
        assert!(cleared.set(1) && cleared.set(3) && !cleared.set(1));
        assert!(cleared.has(1) && !cleared.has(2) && cleared.has(3));
        // Stages are counted from 1, and eight fit.
        assert!(!cleared.set(0) && !cleared.set(9));
        assert!(!cleared.has(0) && !cleared.has(9) && !cleared.has(8));
        assert!(cleared.set(8));
        assert!(cleared.has(8));
    }
}
