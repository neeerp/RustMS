use crate::actor::field::FieldMapEntityNpc;
use crate::actor::FieldActor;
use crate::message::{ChannelMessage, FieldKey, FieldMessage, RuntimeLocation};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

const MAP_NPC_OBJECT_ID_BASE: i32 = 1_000_000_000;

struct FieldHandle {
    sender: mpsc::Sender<FieldMessage>,
}

pub struct ChannelActor {
    channel_id: u8,
    rx: mpsc::Receiver<ChannelMessage>,
    fields: HashMap<FieldKey, FieldHandle>,
}

impl ChannelActor {
    pub fn new(channel_id: u8, rx: mpsc::Receiver<ChannelMessage>) -> Self {
        Self {
            channel_id,
            rx,
            fields: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        info!(channel_id = self.channel_id, "ChannelActor started");

        while let Some(message) = self.rx.recv().await {
            self.handle_message(message).await;
        }

        info!(channel_id = self.channel_id, "ChannelActor shutting down");
    }

    async fn handle_message(&mut self, message: ChannelMessage) {
        match message {
            ChannelMessage::JoinClient {
                client_id,
                sender,
                character,
                location,
            } => {
                let field = self.get_or_create_field(location.field_key());
                if field
                    .send(FieldMessage::Join {
                        client_id,
                        sender,
                        character,
                    })
                    .await
                    .is_err()
                {
                    warn!(client_id, location = ?location, "Failed to join client to field");
                }
            }
            ChannelMessage::LeaveClient {
                client_id,
                location,
            } => {
                self.send_to_field(location, FieldMessage::Leave { client_id }, client_id)
                    .await;
            }
            ChannelMessage::Chat {
                client_id,
                location,
                packet,
            } => {
                self.send_to_field(location, FieldMessage::Chat { from: client_id, packet }, client_id)
                    .await;
            }
            ChannelMessage::Move {
                client_id,
                location,
                packet,
                movement_bytes,
            } => {
                self.send_to_field(
                    location,
                    FieldMessage::Move {
                        from: client_id,
                        packet,
                        movement_bytes,
                    },
                    client_id,
                )
                .await;
            }
            ChannelMessage::TransferWithinChannel {
                client_id,
                sender,
                character,
                old,
                new,
            } => {
                self.send_to_field(old, FieldMessage::Leave { client_id }, client_id)
                    .await;

                let field = self.get_or_create_field(new.field_key());
                if field
                    .send(FieldMessage::Join {
                        client_id,
                        sender,
                        character,
                    })
                    .await
                    .is_err()
                {
                    warn!(client_id, location = ?new, "Failed to join transferred client to field");
                }
            }
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

    async fn send_to_field(&self, location: RuntimeLocation, message: FieldMessage, client_id: i32) {
        let field_key = location.field_key();
        let Some(field) = self.fields.get(&field_key) else {
            warn!(client_id, field = ?field_key, "No field actor for client location");
            return;
        };

        if field.sender.send(message).await.is_err() {
            warn!(client_id, field = ?field_key, "Failed to forward channel event to field");
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
    use crate::message::{FieldCharacter, ServerMessage};
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

    #[tokio::test]
    async fn different_channel_actors_isolate_same_map_presence() {
        let (first_tx, first_rx) = mpsc::channel(16);
        let (second_tx, second_rx) = mpsc::channel(16);
        tokio::spawn(ChannelActor::new(0, first_rx).run());
        tokio::spawn(ChannelActor::new(1, second_rx).run());

        let (first_client_tx, mut first_client_rx) = mpsc::channel::<ServerMessage>(16);
        let (second_client_tx, mut second_client_rx) = mpsc::channel::<ServerMessage>(16);

        let mut first_character = test_character(1, "first", 100000000, 240, 190);
        first_character.channel_id = 0;
        let mut second_character = test_character(2, "second", 100000000, 240, 190);
        second_character.channel_id = 1;

        first_tx
            .send(ChannelMessage::JoinClient {
                client_id: 1,
                sender: first_client_tx,
                character: first_character,
                location: RuntimeLocation {
                    channel_id: 0,
                    map_id: 100000000,
                    instance_id: 0,
                },
            })
            .await
            .unwrap();
        second_tx
            .send(ChannelMessage::JoinClient {
                client_id: 2,
                sender: second_client_tx,
                character: second_character,
                location: RuntimeLocation {
                    channel_id: 1,
                    map_id: 100000000,
                    instance_id: 0,
                },
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(100), first_client_rx.recv())
            .await
            .is_err());
        assert!(timeout(Duration::from_millis(100), second_client_rx.recv())
            .await
            .is_err());
    }
}
