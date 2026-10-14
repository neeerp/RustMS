//! Messages exchanged between actors, and the handles used to send them.
//!
//! Topology (arrows are channel sends):
//!
//! ```text
//!   ClientActor ──► FieldActor        (its current map: chat, movement, NPCs)
//!   ClientActor ──► WorldDirectory    (name lookup: whisper, broadcast, messenger)
//!   FieldActor / WorldDirectory ──► ClientHandle (try_send, never awaited)
//! ```
//!
//! Clients await their sends into fields and the directory (bounded
//! backpressure), but fields and the directory never await a client: a client
//! whose queue is full is kicked instead. That makes wait cycles impossible.

use game::{AvatarLook, FieldAvatar};
use net::{BroadcastScope, ClientId, FieldRequest, MessengerRequest, WhisperDelivery};
use packet::Packet;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Capacity of each client's inbound queue. A client this far behind is
/// considered dead weight and disconnected.
pub const CLIENT_QUEUE_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FieldKey {
    pub channel_id: u8,
    pub map_id: i32,
    pub instance_id: u32,
}

impl FieldKey {
    pub fn new(channel_id: u8, map_id: i32) -> Self {
        Self {
            channel_id,
            map_id,
            instance_id: 0,
        }
    }
}

/// Messages delivered TO a client actor.
#[derive(Debug)]
pub enum ServerMessage {
    /// NPC identity validated by the player's field. Discard if stale.
    NpcConversation {
        npc_id: i32,
        field: FieldKey,
        request_id: u64,
    },
    /// A mob this player helped kill; `exp` is their share.
    MobKilled {
        mob_id: i32,
        object_id: i32,
        exp: i32,
        x: i16,
        y: i16,
    },
    /// This player broke a reactor; its loot drops for them.
    ReactorBroken {
        reactor_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    },
    /// The field gave this player a drop to put in their inventory.
    PickedUp {
        drop: crate::actor::field_drops::Drop,
    },
    /// Whether a quest NPC is on the player's map and within reach.
    QuestNpcChecked {
        request_id: u64,
        nearby: bool,
    },
    /// Someone on this channel looked for this player; answer with the map.
    Locate {
        reply_to: ClientHandle,
    },
    /// `requester` (maybe this player) double-clicked this player; send them
    /// CHAR_INFO.
    CharInfoRequested {
        requester: ClientHandle,
    },
    SendPacket(Packet),
}

/// A cheap, clonable way to reach one client connection.
#[derive(Clone, Debug)]
pub struct ClientHandle {
    pub client_id: ClientId,
    /// Distinguishes reconnects of the same character.
    pub connection_id: u64,
    tx: mpsc::Sender<ServerMessage>,
    kill: CancellationToken,
}

impl ClientHandle {
    pub fn new(
        client_id: ClientId,
        connection_id: u64,
        tx: mpsc::Sender<ServerMessage>,
        kill: CancellationToken,
    ) -> Self {
        Self {
            client_id,
            connection_id,
            tx,
            kill,
        }
    }

    /// Queue a message without waiting. A full queue means the client is not
    /// keeping up, so it is disconnected rather than allowed to stall the
    /// sender. Returns whether the message was queued.
    pub fn send(&self, message: ServerMessage) -> bool {
        match self.tx.try_send(message) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                warn!(
                    client_id = self.client_id,
                    "Client queue full; disconnecting slow client"
                );
                self.kill.cancel();
                false
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }

    pub fn send_packet(&self, packet: Packet) -> bool {
        self.send(ServerMessage::SendPacket(packet))
    }

    /// Ask the client actor to disconnect.
    pub fn kick(&self) {
        self.kill.cancel();
    }
}

/// Messages to a field actor.
#[derive(Debug)]
pub enum FieldMessage {
    Join {
        handle: ClientHandle,
        avatar: FieldAvatar,
    },
    Leave {
        client_id: ClientId,
    },
    /// The occupant's appearance or level changed.
    UpdateAvatar {
        client_id: ClientId,
        avatar: FieldAvatar,
        /// Sent to everyone else on the map (e.g. UPDATE_CHAR_LOOK).
        announce: Option<Packet>,
    },
    Chat {
        from: ClientId,
        packet: Packet,
    },
    Move {
        from: ClientId,
        packet: Packet,
        movement_bytes: Vec<u8>,
    },
    NpcInteraction {
        from: ClientId,
        object_id: i32,
        request_id: u64,
    },
    /// A player's request of the field (`net::handler::FieldRequest`).
    FromClient {
        from: ClientId,
        request: FieldRequest,
    },
    /// Put a player's loot from a kill on the ground.
    SpawnLoot {
        owner: ClientId,
        loot: Vec<net::loot::Loot>,
        x: i16,
        y: i16,
        dropper: i32,
    },
    /// The client couldn't take the drop (full inventory); put it back.
    ReturnDrop {
        drop: crate::actor::field_drops::Drop,
    },
    /// Check that an NPC with this template id is near the player before a
    /// quest action; answered with `ServerMessage::QuestNpcChecked`.
    QuestNpc {
        from: ClientId,
        npc_id: i32,
        request_id: u64,
    },
}

/// A way to reach one field actor.
#[derive(Clone, Debug)]
pub struct FieldHandle {
    pub key: FieldKey,
    pub tx: mpsc::Sender<FieldMessage>,
}

/// Messages to the world directory.
#[derive(Debug)]
pub enum DirectoryMessage {
    Register {
        handle: ClientHandle,
        name: String,
        channel_id: u8,
    },
    Unregister {
        client_id: ClientId,
        connection_id: u64,
        /// The client is moving to another channel and will register again.
        changing_channel: bool,
    },
    Whisper {
        from: ClientId,
        delivery: WhisperDelivery,
    },
    Find {
        from: ClientId,
        target_name: String,
    },
    Broadcast {
        from: ClientId,
        scope: BroadcastScope,
        packet: Packet,
    },
    Messenger {
        from: ClientId,
        request: MessengerRequest,
    },
    /// The client's worn equipment changed.
    LookChanged {
        client_id: ClientId,
        look: AvatarLook,
    },
}
