use crate::actor::field::FieldMapEntityNpc;
use crate::actor::FieldActor;
use crate::handler::{BroadcastScope, ClientId};
use crate::message::{ClientEvent, FieldKey, FieldMessage, ServerMessage};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

const MAP_NPC_OBJECT_ID_BASE: i32 = 1_000_000_000;

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
                spawn_portal_id,
                spawn_x,
                spawn_y,
                spawn_stance,
            } => {
                self.handle_map_change(
                    client_id,
                    old_map_id,
                    new_map_id,
                    spawn_portal_id,
                    spawn_x,
                    spawn_y,
                    spawn_stance,
                )
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

    async fn handle_map_change(
        &mut self,
        client_id: ClientId,
        old_map_id: i32,
        new_map_id: i32,
        _spawn_portal_id: Option<u8>,
        spawn_x: Option<i16>,
        spawn_y: Option<i16>,
        spawn_stance: Option<u8>,
    ) {
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
        if let Some(x) = spawn_x {
            entry.character.x = x;
        }
        if let Some(y) = spawn_y {
            entry.character.y = y;
        }
        if let Some(stance) = spawn_stance {
            entry.character.stance = stance;
        }

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
        let sender_channel_id = self
            .clients
            .get(&from)
            .map(|entry| entry.field_key.channel_id);

        match scope {
            BroadcastScope::Map(map_id) => self
                .clients
                .iter()
                .filter_map(|(&client_id, entry)| {
                    (entry.field_key.map_id == *map_id
                        && sender_channel_id
                            .map(|channel_id| entry.field_key.channel_id == channel_id)
                            .unwrap_or(true))
                    .then_some(client_id)
                })
                .collect(),
            BroadcastScope::MapExcludeSelf(map_id) => self
                .clients
                .iter()
                .filter_map(|(&client_id, entry)| {
                    (entry.field_key.map_id == *map_id
                        && client_id != from
                        && sender_channel_id
                            .map(|channel_id| entry.field_key.channel_id == channel_id)
                            .unwrap_or(true))
                    .then_some(client_id)
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
        let map_npcs = load_field_map_npcs(field_key);
        let actor = FieldActor::new(field_key, field_rx, map_npcs);
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

fn load_field_map_npcs(field_key: FieldKey) -> Vec<FieldMapEntityNpc> {
    let Ok(game_data) = net::get_game_data() else {
        warn!(field = ?field_key, "Failed to load game data for field NPCs");
        return Vec::new();
    };

    let Some(field) = game_data.field(field_key.map_id) else {
        return Vec::new();
    };

    field
        .map_npcs
        .iter()
        .enumerate()
        .filter_map(|(index, npc)| {
            i32::try_from(index).ok().map(|offset| FieldMapEntityNpc {
                object_id: MAP_NPC_OBJECT_ID_BASE + offset,
                npc_id: npc.npc_id,
                x: npc.x,
                y: npc.y,
                flip: npc.flip,
                foothold: npc.foothold,
                rx0: npc.rx0,
                rx1: npc.rx1,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::FieldCharacter;
    use net::packet::op::SendOpcode;
    use packet::io::read::PktRead;
    use std::io::Cursor;
    use tokio::time::{timeout, Duration};

    fn test_character(id: i32, name: &str, map_id: i32, x: i16, y: i16) -> FieldCharacter {
        FieldCharacter {
            id,
            name: name.to_string(),
            level: 1,
            job: 0,
            face: 20000,
            hair: 30000,
            skin: 0,
            gender: 0,
            channel_id: 0,
            map_id,
            x,
            y,
            stance: 2,
        }
    }

    fn decode_spawn_position(packet: &packet::Packet) -> (i16, i16, u8) {
        let mut cursor = Cursor::new(&packet.bytes[..]);
        let opcode = cursor.read_short().expect("spawn opcode");
        assert_eq!(opcode, SendOpcode::SpawnPlayer as i16);
        cursor.read_int().expect("spawn character id");
        cursor.read_byte().expect("spawn level");
        cursor.read_str_with_length().expect("spawn name");
        cursor
            .read_bytes(2 + 2 + 1 + 2 + 1 + 8 + 4 + 4 + 4 + 43 + 4 + 61 + 2)
            .expect("spawn pre-position payload");
        cursor
            .read_bytes(1 + 1 + 4 + 1 + 4 + 1 + 4 + 1 + 4 + 1 + 1 + 4 + 4 + 4 + 4 + 4 + 4 + 4)
            .expect("spawn look payload");
        let x = cursor.read_short().expect("spawn x");
        let y = cursor.read_short().expect("spawn y");
        let stance = cursor.read_byte().expect("spawn stance");
        (x, y, stance)
    }

    #[tokio::test]
    async fn map_change_join_uses_spawn_coordinates_when_present() {
        let (world_tx, world_rx) = mpsc::channel(16);
        let world = WorldServerActor::new(world_rx);
        tokio::spawn(world.run());

        let (mover_tx, _mover_rx) = mpsc::channel(16);
        let (observer_tx, mut observer_rx) = mpsc::channel(16);

        world_tx
            .send(ClientEvent::Connected {
                client_id: 1,
                sender: mover_tx,
                character: test_character(1, "mover", 100000000, 240, 190),
            })
            .await
            .unwrap();
        world_tx
            .send(ClientEvent::Connected {
                client_id: 2,
                sender: observer_tx,
                character: test_character(2, "observer", 100000001, 10, 20),
            })
            .await
            .unwrap();
        world_tx
            .send(ClientEvent::MapChanged {
                client_id: 1,
                old_map_id: 100000000,
                new_map_id: 100000001,
                spawn_portal_id: Some(2),
                spawn_x: Some(202),
                spawn_y: Some(124),
                spawn_stance: Some(2),
            })
            .await
            .unwrap();

        let packet = match observer_rx.recv().await.expect("observer spawn packet") {
            ServerMessage::SendPacket(packet) => packet,
            other => panic!("expected spawn packet, got {other:?}"),
        };
        let (x, y, stance) = decode_spawn_position(&packet);
        assert_eq!((x, y, stance), (202, 124, 2));
    }

    #[tokio::test]
    async fn same_map_different_channels_do_not_share_presence() {
        let (world_tx, world_rx) = mpsc::channel(16);
        let world = WorldServerActor::new(world_rx);
        tokio::spawn(world.run());

        let (first_tx, mut first_rx) = mpsc::channel(16);
        let (second_tx, mut second_rx) = mpsc::channel(16);

        let mut first_character = test_character(1, "first", 100000000, 240, 190);
        first_character.channel_id = 0;
        let mut second_character = test_character(2, "second", 100000000, 240, 190);
        second_character.channel_id = 1;

        world_tx
            .send(ClientEvent::Connected {
                client_id: 1,
                sender: first_tx,
                character: first_character,
            })
            .await
            .unwrap();
        world_tx
            .send(ClientEvent::Connected {
                client_id: 2,
                sender: second_tx,
                character: second_character,
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(100), first_rx.recv())
            .await
            .is_err());
        assert!(timeout(Duration::from_millis(100), second_rx.recv())
            .await
            .is_err());
    }
}
