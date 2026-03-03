use crate::handler::{BroadcastScope, ClientId};
use crate::message::{ClientEvent, ServerMessage};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Registry entry for a connected client.
struct ClientEntry {
    sender: mpsc::Sender<ServerMessage>,
    map_id: i32,
    name: String,
}

/// Central actor managing all world server clients.
pub struct WorldServerActor {
    /// Channel to receive events from clients
    event_rx: mpsc::Receiver<ClientEvent>,
    /// Registry of connected clients
    clients: HashMap<ClientId, ClientEntry>,
    /// Map ID to client IDs for efficient map broadcasts
    maps: HashMap<i32, Vec<ClientId>>,
    /// Character name to client ID for directed routing
    names: HashMap<String, ClientId>,
}

impl WorldServerActor {
    pub fn new(event_rx: mpsc::Receiver<ClientEvent>) -> Self {
        Self {
            event_rx,
            clients: HashMap::new(),
            maps: HashMap::new(),
            names: HashMap::new(),
        }
    }

    /// Run the world server event loop.
    pub async fn run(mut self) {
        info!("WorldServerActor started");

        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await;
        }

        info!("WorldServerActor shutting down");
    }

    async fn handle_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Connected {
                client_id,
                sender,
                map_id,
                character_name,
            } => {
                self.register_client(client_id, sender, map_id, character_name);
            }
            ClientEvent::Disconnected { client_id } => {
                self.unregister_client(client_id);
            }
            ClientEvent::MapChanged {
                client_id,
                old_map_id,
                new_map_id,
            } => {
                self.handle_map_change(client_id, old_map_id, new_map_id);
            }
            ClientEvent::Broadcast {
                from,
                scope,
                packet,
            } => {
                self.handle_broadcast(from, scope, packet).await;
            }
            ClientEvent::Whisper {
                from,
                target_name,
                recipient_packet,
                sender_success_packet,
                sender_failure_packet,
            } => {
                self.handle_whisper(
                    from,
                    target_name,
                    recipient_packet,
                    sender_success_packet,
                    sender_failure_packet,
                )
                .await;
            }
        }
    }

    fn register_client(
        &mut self,
        client_id: ClientId,
        sender: mpsc::Sender<ServerMessage>,
        map_id: i32,
        character_name: String,
    ) {
        info!(client_id, map_id, character_name, "Client connected");

        self.names.insert(character_name.clone(), client_id);
        self.clients.insert(
            client_id,
            ClientEntry {
                sender,
                map_id,
                name: character_name,
            },
        );
        self.maps.entry(map_id).or_default().push(client_id);
    }

    fn unregister_client(&mut self, client_id: ClientId) {
        if let Some(entry) = self.clients.remove(&client_id) {
            info!(client_id, "Client disconnected");
            self.names.remove(&entry.name);

            // Remove from map index
            if let Some(clients) = self.maps.get_mut(&entry.map_id) {
                clients.retain(|&id| id != client_id);
                if clients.is_empty() {
                    self.maps.remove(&entry.map_id);
                }
            }
        }
    }

    fn handle_map_change(&mut self, client_id: ClientId, old_map_id: i32, new_map_id: i32) {
        // Remove from old map
        if let Some(clients) = self.maps.get_mut(&old_map_id) {
            clients.retain(|&id| id != client_id);
            if clients.is_empty() {
                self.maps.remove(&old_map_id);
            }
        }

        // Add to new map
        self.maps.entry(new_map_id).or_default().push(client_id);

        // Update client entry
        if let Some(entry) = self.clients.get_mut(&client_id) {
            entry.map_id = new_map_id;
        }

        info!(client_id, old_map_id, new_map_id, "Client changed map");
    }

    async fn handle_broadcast(
        &mut self,
        from: ClientId,
        scope: BroadcastScope,
        packet: packet::Packet,
    ) {
        let targets = self.get_broadcast_targets(from, &scope);

        for client_id in targets {
            if let Some(entry) = self.clients.get(&client_id) {
                let msg = ServerMessage::SendPacket(packet.clone());
                if entry.sender.send(msg).await.is_err() {
                    warn!(
                        client_id,
                        "Failed to send broadcast, client may have disconnected"
                    );
                }
            }
        }
    }

    async fn handle_whisper(
        &mut self,
        from: ClientId,
        target_name: String,
        recipient_packet: packet::Packet,
        sender_success_packet: packet::Packet,
        sender_failure_packet: packet::Packet,
    ) {
        let Some(&target_id) = self.names.get(&target_name) else {
            self.send_packet_to_client(from, sender_failure_packet)
                .await;
            return;
        };

        let delivered = if let Some(entry) = self.clients.get(&target_id) {
            entry
                .sender
                .send(ServerMessage::SendPacket(recipient_packet))
                .await
                .is_ok()
        } else {
            false
        };

        if delivered {
            self.send_packet_to_client(from, sender_success_packet)
                .await;
        } else {
            warn!(from, target_name, "Failed to deliver whisper to target");
            self.send_packet_to_client(from, sender_failure_packet)
                .await;
        }
    }

    async fn send_packet_to_client(&self, client_id: ClientId, packet: packet::Packet) {
        if let Some(entry) = self.clients.get(&client_id) {
            if entry
                .sender
                .send(ServerMessage::SendPacket(packet))
                .await
                .is_err()
            {
                warn!(client_id, "Failed to send directed packet to client");
            }
        }
    }

    fn get_broadcast_targets(&self, from: ClientId, scope: &BroadcastScope) -> Vec<ClientId> {
        match scope {
            BroadcastScope::Map(map_id) => self.maps.get(map_id).cloned().unwrap_or_default(),
            BroadcastScope::MapExcludeSelf(map_id) => self
                .maps
                .get(map_id)
                .map(|clients| clients.iter().filter(|&&id| id != from).copied().collect())
                .unwrap_or_default(),
            BroadcastScope::World => self.clients.keys().copied().collect(),
            BroadcastScope::WorldExcludeSelf => self
                .clients
                .keys()
                .filter(|&&id| id != from)
                .copied()
                .collect(),
        }
    }
}
