use crate::actor::FieldActor;
use crate::handler::{BroadcastScope, ClientId};
use crate::message::{ClientEvent, FieldKey, FieldMessage, ServerMessage};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Registry entry for a connected client.
struct ClientEntry {
    sender: mpsc::Sender<ServerMessage>,
    field_key: FieldKey,
    name: String,
    character: crate::message::FieldCharacter,
}

struct FieldHandle {
    sender: mpsc::Sender<FieldMessage>,
}

/// Central actor managing all world server clients.
pub struct WorldServerActor {
    /// Channel to receive events from clients
    event_rx: mpsc::Receiver<ClientEvent>,
    /// Registry of connected clients
    clients: HashMap<ClientId, ClientEntry>,
    /// Live field actors keyed by field identity
    fields: HashMap<FieldKey, FieldHandle>,
    /// Character name to client ID for directed routing
    names: HashMap<String, ClientId>,
}

impl WorldServerActor {
    pub fn new(event_rx: mpsc::Receiver<ClientEvent>) -> Self {
        Self {
            event_rx,
            clients: HashMap::new(),
            fields: HashMap::new(),
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
                character,
            } => {
                self.register_client(client_id, sender, character).await;
            }
            ClientEvent::Disconnected { client_id } => {
                self.unregister_client(client_id).await;
            }
            ClientEvent::MapChanged {
                client_id,
                old_map_id,
                new_map_id,
            } => {
                self.handle_map_change(client_id, old_map_id, new_map_id)
                    .await;
            }
            ClientEvent::Broadcast {
                from,
                scope,
                packet,
            } => {
                self.handle_broadcast(from, scope, packet).await;
            }
            ClientEvent::FieldChat { from, packet } => {
                self.forward_to_field(from, FieldMessage::Chat { from, packet })
                    .await;
            }
            ClientEvent::FieldMove {
                from,
                packet,
                movement_bytes,
            } => {
                self.forward_to_field(
                    from,
                    FieldMessage::Move {
                        from,
                        packet,
                        movement_bytes,
                    },
                )
                .await;
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

    async fn register_client(
        &mut self,
        client_id: ClientId,
        sender: mpsc::Sender<ServerMessage>,
        character: crate::message::FieldCharacter,
    ) {
        let field_key = character.field_key();
        let character_name = character.name.clone();
        info!(client_id, field = ?field_key, character_name, "Client connected");

        self.names.insert(character_name.clone(), client_id);
        self.clients.insert(
            client_id,
            ClientEntry {
                sender: sender.clone(),
                field_key,
                name: character_name,
                character: character.clone(),
            },
        );

        let field_sender = self.get_or_create_field(field_key);
        if field_sender
            .send(FieldMessage::Join {
                client_id,
                sender,
                character,
            })
            .await
            .is_err()
        {
            warn!(client_id, field = ?field_key, "Failed to join client to field");
        }
    }

    async fn unregister_client(&mut self, client_id: ClientId) {
        if let Some(entry) = self.clients.remove(&client_id) {
            info!(client_id, "Client disconnected");
            self.names.remove(&entry.name);
            if let Some(field) = self.fields.get(&entry.field_key) {
                if field
                    .sender
                    .send(FieldMessage::Leave { client_id })
                    .await
                    .is_err()
                {
                    warn!(client_id, field = ?entry.field_key, "Failed to leave field");
                }
            }
        }
    }

    async fn handle_map_change(&mut self, client_id: ClientId, old_map_id: i32, new_map_id: i32) {
        let Some(entry) = self.clients.get_mut(&client_id) else {
            warn!(
                client_id,
                old_map_id, new_map_id, "Map change from unknown client"
            );
            return;
        };

        let old_field_key = entry.field_key;
        let new_field_key = FieldKey {
            channel_id: old_field_key.channel_id,
            map_id: new_map_id,
            instance_id: old_field_key.instance_id,
        };

        entry.field_key = new_field_key;
        entry.character.map_id = new_map_id;

        let sender = entry.sender.clone();
        let character = entry.character.clone();

        if let Some(old_field) = self.fields.get(&old_field_key) {
            if old_field
                .sender
                .send(FieldMessage::Leave { client_id })
                .await
                .is_err()
            {
                warn!(client_id, field = ?old_field_key, "Failed to leave previous field");
            }
        }

        let new_field = self.get_or_create_field(new_field_key);
        if new_field
            .send(FieldMessage::Join {
                client_id,
                sender,
                character,
            })
            .await
            .is_err()
        {
            warn!(client_id, field = ?new_field_key, "Failed to join destination field");
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
            BroadcastScope::Map(map_id) => self
                .clients
                .iter()
                .filter_map(|(&client_id, entry)| {
                    (entry.field_key.map_id == *map_id).then_some(client_id)
                })
                .collect(),
            BroadcastScope::MapExcludeSelf(map_id) => self
                .clients
                .iter()
                .filter_map(|(&client_id, entry)| {
                    (entry.field_key.map_id == *map_id && client_id != from).then_some(client_id)
                })
                .collect(),
            BroadcastScope::World => self.clients.keys().copied().collect(),
            BroadcastScope::WorldExcludeSelf => self
                .clients
                .keys()
                .filter(|&&id| id != from)
                .copied()
                .collect(),
        }
    }

    fn get_or_create_field(&mut self, field_key: FieldKey) -> mpsc::Sender<FieldMessage> {
        if let Some(handle) = self.fields.get(&field_key) {
            return handle.sender.clone();
        }

        let (field_tx, field_rx) = mpsc::channel(64);
        let actor = FieldActor::new(field_key, field_rx);
        tokio::spawn(async move {
            actor.run().await;
        });

        self.fields.insert(
            field_key,
            FieldHandle {
                sender: field_tx.clone(),
            },
        );

        field_tx
    }

    async fn forward_to_field(&self, client_id: ClientId, message: FieldMessage) {
        let Some(entry) = self.clients.get(&client_id) else {
            warn!(client_id, "Ignoring field event from unregistered client");
            return;
        };

        let Some(field) = self.fields.get(&entry.field_key) else {
            warn!(client_id, field = ?entry.field_key, "No field actor for client");
            return;
        };

        if field.sender.send(message).await.is_err() {
            warn!(client_id, field = ?entry.field_key, "Failed to forward field event");
        }
    }
}
