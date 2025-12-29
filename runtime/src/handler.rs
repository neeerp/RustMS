use crate::message::{BroadcastScope, ClientId};
use db::session::SessionWrapper;
use packet::Packet;

/// Context available to packet handlers.
/// Provides access to session data without exposing the network stream.
pub struct HandlerContext<'a> {
    /// The client's identifier (character_id for world server)
    pub client_id: ClientId,
    /// Session and character data
    pub session: &'a mut SessionWrapper,
}

/// Actions a handler can request the actor to perform.
#[derive(Debug)]
pub enum HandlerAction {
    /// Send a packet to the requesting client
    Reply(Packet),
    /// Broadcast a packet to multiple clients
    Broadcast {
        scope: BroadcastScope,
        packet: Packet,
    },
    /// Disconnect this client
    Disconnect,
}

/// Result of handling a packet - contains all requested actions.
#[derive(Debug, Default)]
pub struct HandlerResult {
    pub actions: Vec<HandlerAction>,
}

impl HandlerResult {
    /// Create an empty result (no actions).
    pub fn empty() -> Self {
        Self { actions: vec![] }
    }

    /// Create a result with a single reply packet.
    pub fn reply(packet: Packet) -> Self {
        Self {
            actions: vec![HandlerAction::Reply(packet)],
        }
    }

    /// Create a result with multiple reply packets.
    pub fn replies(packets: Vec<Packet>) -> Self {
        Self {
            actions: packets.into_iter().map(HandlerAction::Reply).collect(),
        }
    }

    /// Add a reply packet to this result.
    pub fn with_reply(mut self, packet: Packet) -> Self {
        self.actions.push(HandlerAction::Reply(packet));
        self
    }

    /// Add a broadcast action to this result.
    pub fn with_broadcast(mut self, scope: BroadcastScope, packet: Packet) -> Self {
        self.actions.push(HandlerAction::Broadcast { scope, packet });
        self
    }

    /// Add a disconnect action to this result.
    pub fn with_disconnect(mut self) -> Self {
        self.actions.push(HandlerAction::Disconnect);
        self
    }
}
