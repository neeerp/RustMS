use crate::message::{ClientHandle, DirectoryMessage};
use net::{BroadcastScope, ClientId, WhisperDelivery};
use packet::Packet;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

struct Entry {
    handle: ClientHandle,
    name: String,
}

/// World-wide directory of online characters: who is online, under which
/// name, on which channel. It owns no game state; it exists for features
/// that cross fields (whispers, world broadcasts, and later buddies/parties).
pub struct WorldDirectory {
    rx: mpsc::Receiver<DirectoryMessage>,
    clients: HashMap<ClientId, Entry>,
    names: HashMap<String, ClientId>,
}

impl WorldDirectory {
    pub fn new(rx: mpsc::Receiver<DirectoryMessage>) -> Self {
        Self {
            rx,
            clients: HashMap::new(),
            names: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        while let Some(message) = self.rx.recv().await {
            self.handle(message);
        }
    }

    fn handle(&mut self, message: DirectoryMessage) {
        match message {
            DirectoryMessage::Register {
                handle,
                name,
                channel_id,
            } => self.register(handle, name, channel_id),
            DirectoryMessage::Unregister {
                client_id,
                connection_id,
            } => self.unregister(client_id, connection_id),
            DirectoryMessage::Whisper { from, delivery } => self.whisper(from, delivery),
            DirectoryMessage::Broadcast {
                from,
                scope,
                packet,
            } => self.broadcast(from, scope, packet),
        }
    }

    fn register(&mut self, handle: ClientHandle, name: String, channel_id: u8) {
        let client_id = handle.client_id;
        if let Some(previous) = self.clients.remove(&client_id) {
            // The same character connected again: only one session may live.
            warn!(client_id, "Duplicate login; disconnecting previous connection");
            previous.handle.kick();
            self.names.remove(&previous.name);
        }
        info!(client_id, name, channel_id, "Character online");
        self.names.insert(name.clone(), client_id);
        self.clients.insert(client_id, Entry { handle, name });
    }

    fn unregister(&mut self, client_id: ClientId, connection_id: u64) {
        // Ignore a stale unregister from a connection that was replaced.
        let is_current = self
            .clients
            .get(&client_id)
            .is_some_and(|entry| entry.handle.connection_id == connection_id);
        if is_current {
            if let Some(entry) = self.clients.remove(&client_id) {
                self.names.remove(&entry.name);
                info!(client_id, "Character offline");
            }
        }
    }

    fn whisper(&self, from: ClientId, delivery: WhisperDelivery) {
        let Some(sender) = self.clients.get(&from) else {
            return;
        };
        let delivered = self
            .names
            .get(&delivery.target_name)
            .and_then(|id| self.clients.get(id))
            .is_some_and(|target| target.handle.send_packet(delivery.recipient_packet));
        sender.handle.send_packet(if delivered {
            delivery.sender_success_packet
        } else {
            delivery.sender_failure_packet
        });
    }

    fn broadcast(&self, from: ClientId, scope: BroadcastScope, packet: Packet) {
        for (&client_id, entry) in &self.clients {
            if matches!(scope, BroadcastScope::WorldExcludeSelf) && client_id == from {
                continue;
            }
            entry.handle.send_packet(packet.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ServerMessage;
    use tokio_util::sync::CancellationToken;

    fn handle(client_id: ClientId, connection_id: u64) -> (ClientHandle, mpsc::Receiver<ServerMessage>, CancellationToken) {
        let (tx, rx) = mpsc::channel(4);
        let kill = CancellationToken::new();
        (ClientHandle::new(client_id, connection_id, tx, kill.clone()), rx, kill)
    }

    #[test]
    fn duplicate_login_kicks_the_previous_connection_only() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let (first, _first_rx, first_kill) = handle(1, 10);
        let (second, _second_rx, second_kill) = handle(1, 11);

        directory.register(first, "dup".into(), 0);
        directory.register(second, "dup".into(), 0);
        assert!(first_kill.is_cancelled());
        assert!(!second_kill.is_cancelled());

        // The old connection's late unregister must not remove the new one.
        directory.unregister(1, 10);
        assert!(directory.names.contains_key("dup"));
        directory.unregister(1, 11);
        assert!(!directory.names.contains_key("dup"));
    }

    #[test]
    fn full_client_queue_kicks_instead_of_blocking() {
        let (client, _rx, kill) = handle(1, 1);
        for _ in 0..4 {
            assert!(client.send_packet(Packet::new_empty()));
        }
        assert!(!client.send_packet(Packet::new_empty()));
        assert!(kill.is_cancelled());
    }
}
