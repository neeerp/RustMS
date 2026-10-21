//! One party's run through an event in its own copy of the event's maps
//! (`net::events`). The directory picks the team and reserves the event on
//! the channel; this actor gives each member a ticket into the copy, keeps
//! the time, and when the event ends sends those still in it out, closes
//! the copy's fields and frees the event for the next party.
//!
//! Members' client actors do the moving: a ticket makes `change_field` use
//! the copy's fields for the event's maps, and leaving those maps, the
//! party or the game hands the ticket back (`EventMessage::Left`).

use crate::actor::registry::FieldRegistry;
use crate::message::{ClientHandle, DirectoryMessage, ServerMessage};
use crate::supervisor::spawn_supervised;
use net::events::{EventKind, Team};
use net::ClientId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, sleep_until, Instant};
use tracing::info;

/// How long members have to get out after the event ends before its fields
/// close.
const GRACE: Duration = Duration::from_secs(2);
const EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug)]
pub enum EventMessage {
    /// `client_id` left the event's maps, the party, or the game.
    Left { client_id: ClientId },
}

/// A way to reach one running event.
#[derive(Clone, Debug)]
pub struct EventHandle {
    tx: mpsc::Sender<EventMessage>,
}

impl EventHandle {
    pub fn left(&self, client_id: ClientId) {
        // Only members send, once each, so the queue can't fill.
        let _ = self.tx.try_send(EventMessage::Left { client_id });
    }
}

/// What a member holds while in the event: the event's own fields for its
/// maps, and when it ends.
#[derive(Clone, Debug)]
pub struct EventTicket {
    pub id: u32,
    pub kind: EventKind,
    pub ends: Instant,
    pub event: EventHandle,
}

impl EventTicket {
    /// Whether `map_id` is one of the event's maps, played in its copy.
    pub fn holds(&self, map_id: i32) -> bool {
        self.kind.event().holds(map_id)
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
    let actor = EventActor {
        ticket: EventTicket {
            id,
            kind,
            ends: Instant::now() + kind.event().duration(),
            event: EventHandle { tx },
        },
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
    };
    // However it ends, the copy closes and the next party may go.
    spawn_supervised(format!("event {kind:?} {id}"), actor.run(), move || {
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
    ticket: EventTicket,
    rx: mpsc::Receiver<EventMessage>,
    team: Team,
    members: HashMap<ClientId, ClientHandle>,
}

impl EventActor {
    async fn run(mut self) {
        let EventTicket { id, kind, ends, .. } = self.ticket;
        info!(?kind, id, members = ?self.team.members(), "Event started");
        let gone: Vec<_> = self
            .members
            .values()
            .filter(|member| !member.send(ServerMessage::EventStarted(self.ticket.clone())))
            .map(|member| member.client_id)
            .collect();
        let mut over = false;
        for client_id in gone {
            over |= self.team.leave(client_id);
        }
        // The members' tickets hold the only senders besides ours.
        drop(self.ticket);
        while !over {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(EventMessage::Left { client_id }) => over = self.team.leave(client_id),
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
        sleep(GRACE).await;
    }
}
