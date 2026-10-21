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

use crate::actor::registry::FieldRegistry;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use net::events::{draw_question, Cloto, EventCommand, EventKind, Stage1, Team};
use net::ClientId;
use std::collections::HashMap;
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
    /// `client_id` talks to Cloto on the first stage.
    Cloto {
        client_id: ClientId,
        reply: oneshot::Sender<Cloto>,
    },
    /// `client_id`'s script did something the event keeps track of.
    Command {
        client_id: ClientId,
        command: EventCommand,
    },
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

    /// Where the member stands with Cloto, or `None` once the event is
    /// over.
    pub async fn cloto(&self) -> Option<Cloto> {
        let (reply, view) = oneshot::channel();
        self.event
            .tx
            .send(EventMessage::Cloto {
                client_id: self.member,
                reply,
            })
            .await
            .ok()?;
        view.await.ok()
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
        ends: Instant::now() + kind.event().duration(),
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
    ends: Instant,
    /// Ours until the tickets are out; then only they keep the queue open.
    event: Option<EventHandle>,
    rx: mpsc::Receiver<EventMessage>,
    team: Team,
    members: HashMap<ClientId, ClientHandle>,
    stage1: Stage1,
}

impl EventActor {
    async fn run(mut self) {
        let (id, kind, ends) = (self.id, self.kind, self.ends);
        info!(?kind, id, members = ?self.team.members(), "Event started");
        if let Some(event) = self.event.take() {
            for member in self.members.values() {
                // A ticket that doesn't arrive hands itself back.
                member.send(ServerMessage::EventStarted(EventTicket {
                    id,
                    kind,
                    ends,
                    member: member.client_id,
                    event: event.clone(),
                }));
            }
        }
        let mut over = false;
        while !over {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(EventMessage::Left { client_id }) => over = self.team.leave(client_id),
                    Some(EventMessage::Cloto { client_id, reply }) => {
                        let _ = reply.send(self.stage1.talk(client_id, &self.team, draw_question));
                    }
                    Some(EventMessage::Command { client_id, command }) => {
                        self.command(client_id, command);
                    }
                    None => over = true,
                },
                _ = sleep_until(ends) => over = true,
            }
        }
        info!(?kind, id, "Event ended");
        for member in self.team.members() {
            if let Some(handle) = self.members.get(member) {
                handle.send(ServerMessage::EventEnded { id });
            }
        }
        // Give them time to get out before the copy closes.
        sleep(GRACE).await;
    }

    fn command(&mut self, client_id: ClientId, command: EventCommand) {
        if !self.team.has(client_id) {
            return;
        }
        match command {
            EventCommand::PassGiven => self.stage1.pass_given(client_id),
            EventCommand::ClearStage(1) if client_id == self.team.leader() => {
                if self.stage1.clear() {
                    info!(kind = ?self.kind, id = self.id, "Stage 1 cleared");
                }
            }
            EventCommand::ClearStage(stage) => debug!(stage, client_id, "Clear not taken"),
        }
    }
}
