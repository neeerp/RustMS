use crate::message::{FieldCharacter, FieldKey, FieldMessage, ServerMessage};
use net::packet::build::world::field::{
    build_player_enter_field, build_player_leave_field, parse_movement_state, ForeignCharacter,
};
use net::packet::build::world::npc::{build_spawn_npc, ForeignNpc};
use packet::Packet;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

struct Occupant {
    sender: mpsc::Sender<ServerMessage>,
    character: FieldCharacter,
}

#[derive(Clone, Debug)]
pub struct FieldMapEntityNpc {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

pub struct FieldActor {
    key: FieldKey,
    event_rx: mpsc::Receiver<FieldMessage>,
    occupants: HashMap<i32, Occupant>,
    map_npcs: Vec<FieldMapEntityNpc>,
}

impl FieldActor {
    pub fn new(
        key: FieldKey,
        event_rx: mpsc::Receiver<FieldMessage>,
        map_npcs: Vec<FieldMapEntityNpc>,
    ) -> Self {
        Self {
            key,
            event_rx,
            occupants: HashMap::new(),
            map_npcs,
        }
    }

    pub async fn run(mut self) {
        info!(field = ?self.key, "FieldActor started");

        while let Some(message) = self.event_rx.recv().await {
            self.handle_message(message).await;
        }

        info!(field = ?self.key, "FieldActor shutting down");
    }

    async fn handle_message(&mut self, message: FieldMessage) {
        match message {
            FieldMessage::Join {
                client_id,
                sender,
                character,
            } => {
                self.handle_join(client_id, sender, character).await;
            }
            FieldMessage::Leave { client_id } => {
                self.handle_leave(client_id).await;
            }
            FieldMessage::Chat { packet, .. } => {
                self.broadcast_to_all(packet).await;
            }
            FieldMessage::Move {
                from,
                packet,
                movement_bytes,
            } => {
                if let Some(occupant) = self.occupants.get_mut(&from) {
                    if let Some((x, y, stance)) = parse_movement_state(
                        &movement_bytes,
                        (
                            occupant.character.x,
                            occupant.character.y,
                            occupant.character.stance,
                        ),
                    ) {
                        occupant.character.x = x;
                        occupant.character.y = y;
                        occupant.character.stance = stance;
                    }
                }
                self.broadcast_to_others(from, packet).await;
            }
        }
    }

    async fn handle_join(
        &mut self,
        client_id: i32,
        sender: mpsc::Sender<ServerMessage>,
        character: FieldCharacter,
    ) {
        for map_npc in &self.map_npcs {
            match build_spawn_npc(&to_foreign_npc(map_npc)) {
                Ok(packet) => self.send_packet(&sender, packet, client_id).await,
                Err(error) => {
                    warn!(
                        client_id,
                        npc_id = map_npc.npc_id,
                        error = %error,
                        "Failed to build existing map NPC replay packet"
                    );
                }
            }
        }

        for occupant in self.occupants.values() {
            match build_player_enter_field(&to_foreign_character(&occupant.character)) {
                Ok(packet) => {
                    self.send_packet(&sender, packet, client_id).await;
                }
                Err(error) => {
                    warn!(
                        client_id,
                        error = %error,
                        "Failed to build existing occupant replay packet"
                    );
                }
            }
        }

        match build_player_enter_field(&to_foreign_character(&character)) {
            Ok(join_packet) => {
                for (&occupant_id, occupant) in &self.occupants {
                    self.send_packet(&occupant.sender, join_packet.clone(), occupant_id)
                        .await;
                }
            }
            Err(error) => {
                warn!(
                    client_id,
                    error = %error,
                    "Failed to build join packet for occupant broadcast"
                );
            }
        }

        self.occupants
            .insert(client_id, Occupant { sender, character });
    }

    async fn handle_leave(&mut self, client_id: i32) {
        let Some(occupant) = self.occupants.remove(&client_id) else {
            return;
        };

        match build_player_leave_field(occupant.character.id) {
            Ok(packet) => self.broadcast_to_all(packet).await,
            Err(error) => warn!(client_id, error = %error, "Failed to build leave packet"),
        }
    }

    async fn broadcast_to_all(&self, packet: Packet) {
        for (&client_id, occupant) in &self.occupants {
            self.send_packet(&occupant.sender, packet.clone(), client_id)
                .await;
        }
    }

    async fn broadcast_to_others(&self, from: i32, packet: Packet) {
        for (&client_id, occupant) in &self.occupants {
            if client_id == from {
                continue;
            }
            self.send_packet(&occupant.sender, packet.clone(), client_id)
                .await;
        }
    }

    async fn send_packet(
        &self,
        sender: &mpsc::Sender<ServerMessage>,
        packet: Packet,
        client_id: i32,
    ) {
        if sender
            .send(ServerMessage::SendPacket(packet))
            .await
            .is_err()
        {
            warn!(client_id, "Failed to send field packet to client");
        }
    }
}

fn to_foreign_character(character: &FieldCharacter) -> ForeignCharacter {
    ForeignCharacter {
        id: character.id,
        name: character.name.clone(),
        level: character.level,
        job: character.job,
        face: character.face,
        hair: character.hair,
        skin: character.skin,
        gender: character.gender,
        map_id: character.map_id,
        x: character.x,
        y: character.y,
        stance: character.stance,
    }
}

fn to_foreign_npc(npc: &FieldMapEntityNpc) -> ForeignNpc {
    ForeignNpc {
        object_id: npc.object_id,
        npc_id: npc.npc_id,
        x: npc.x,
        y: npc.y,
        flip: npc.flip,
        foothold: npc.foothold,
        rx0: npc.rx0,
        rx1: npc.rx1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ServerMessage;
    use net::packet::build::world::field::{build_local_chat, build_player_move};
    use net::packet::op::SendOpcode;
    use packet::io::read::PktRead;
    use std::io::Cursor;
    use tokio::sync::mpsc;

    fn test_character(id: i32, name: &str) -> FieldCharacter {
        FieldCharacter {
            id,
            name: name.to_string(),
            level: 1,
            job: 0,
            face: 20000,
            hair: 30000,
            skin: 0,
            gender: 0,
            map_id: 1_000_000,
            x: 240,
            y: 190,
            stance: 2,
        }
    }

    fn assert_opcode(msg: ServerMessage, opcode: SendOpcode) {
        match msg {
            ServerMessage::SendPacket(packet) => assert_eq!(packet.opcode(), opcode as i16),
            other => panic!("expected packet, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn joining_second_player_replays_existing_occupant() {
        let (field_tx, field_rx) = mpsc::channel(8);
        let field = FieldActor::new(
            FieldKey {
                channel_id: 0,
                map_id: 1_000_000,
                instance_id: 0,
            },
            field_rx,
            Vec::new(),
        );
        tokio::spawn(field.run());

        let (first_tx, mut first_rx) = mpsc::channel(8);
        let (second_tx, mut second_rx) = mpsc::channel(8);

        field_tx
            .send(FieldMessage::Join {
                client_id: 1,
                sender: first_tx,
                character: test_character(1, "first"),
            })
            .await
            .unwrap();
        field_tx
            .send(FieldMessage::Join {
                client_id: 2,
                sender: second_tx,
                character: test_character(2, "second"),
            })
            .await
            .unwrap();

        assert_opcode(first_rx.recv().await.unwrap(), SendOpcode::SpawnPlayer);
        assert_opcode(second_rx.recv().await.unwrap(), SendOpcode::SpawnPlayer);
    }

    #[tokio::test]
    async fn leave_broadcasts_remove_packet() {
        let (field_tx, field_rx) = mpsc::channel(8);
        let field = FieldActor::new(
            FieldKey {
                channel_id: 0,
                map_id: 1_000_000,
                instance_id: 0,
            },
            field_rx,
            Vec::new(),
        );
        tokio::spawn(field.run());

        let (first_tx, mut first_rx) = mpsc::channel(8);
        let (second_tx, _second_rx) = mpsc::channel(8);

        field_tx
            .send(FieldMessage::Join {
                client_id: 1,
                sender: first_tx,
                character: test_character(1, "first"),
            })
            .await
            .unwrap();
        field_tx
            .send(FieldMessage::Join {
                client_id: 2,
                sender: second_tx,
                character: test_character(2, "second"),
            })
            .await
            .unwrap();
        let _ = first_rx.recv().await;

        field_tx
            .send(FieldMessage::Leave { client_id: 2 })
            .await
            .unwrap();

        assert_opcode(
            first_rx.recv().await.unwrap(),
            SendOpcode::RemovePlayerFromMap,
        );
    }

    #[tokio::test]
    async fn movement_excludes_sender_but_chat_includes_sender() {
        let (field_tx, field_rx) = mpsc::channel(8);
        let field = FieldActor::new(
            FieldKey {
                channel_id: 0,
                map_id: 1_000_000,
                instance_id: 0,
            },
            field_rx,
            Vec::new(),
        );
        tokio::spawn(field.run());

        let (first_tx, mut first_rx) = mpsc::channel(8);
        let (second_tx, mut second_rx) = mpsc::channel(8);

        field_tx
            .send(FieldMessage::Join {
                client_id: 1,
                sender: first_tx,
                character: test_character(1, "first"),
            })
            .await
            .unwrap();
        field_tx
            .send(FieldMessage::Join {
                client_id: 2,
                sender: second_tx,
                character: test_character(2, "second"),
            })
            .await
            .unwrap();
        let _ = first_rx.recv().await;
        let _ = second_rx.recv().await;

        let movement_packet = build_player_move(1, &[1, 2, 3]).unwrap();
        field_tx
            .send(FieldMessage::Move {
                from: 1,
                packet: movement_packet,
                movement_bytes: vec![1, 2, 3],
            })
            .await
            .unwrap();

        assert_opcode(second_rx.recv().await.unwrap(), SendOpcode::MovePlayer);

        let chat_packet = build_local_chat(1, "hello", false, 0).unwrap();
        field_tx
            .send(FieldMessage::Chat {
                from: 1,
                packet: chat_packet,
            })
            .await
            .unwrap();

        assert_opcode(first_rx.recv().await.unwrap(), SendOpcode::ChatText);
        assert_opcode(second_rx.recv().await.unwrap(), SendOpcode::ChatText);
    }

    #[tokio::test]
    async fn joining_player_receives_map_npcs_before_existing_occupants() {
        let (field_tx, field_rx) = mpsc::channel(8);
        let field = FieldActor::new(
            FieldKey {
                channel_id: 0,
                map_id: 10_000,
                instance_id: 0,
            },
            field_rx,
            vec![FieldMapEntityNpc {
                object_id: 1_000_000_000,
                npc_id: 2101,
                x: 130,
                y: 293,
                flip: true,
                foothold: 51,
                rx0: 80,
                rx1: 180,
            }],
        );
        tokio::spawn(field.run());

        let (first_tx, _first_rx) = mpsc::channel(8);
        let (second_tx, mut second_rx) = mpsc::channel(8);

        field_tx
            .send(FieldMessage::Join {
                client_id: 1,
                sender: first_tx,
                character: test_character(1, "first"),
            })
            .await
            .unwrap();
        field_tx
            .send(FieldMessage::Join {
                client_id: 2,
                sender: second_tx,
                character: test_character(2, "second"),
            })
            .await
            .unwrap();

        let first_msg = second_rx.recv().await.expect("npc spawn message");
        let second_msg = second_rx.recv().await.expect("occupant replay message");

        match first_msg {
            ServerMessage::SendPacket(packet) => {
                let mut cursor = Cursor::new(&packet.bytes[..]);
                assert_eq!(cursor.read_short().expect("opcode"), SendOpcode::SpawnNpc as i16);
                assert_eq!(cursor.read_int().expect("object id"), 1_000_000_000);
                assert_eq!(cursor.read_int().expect("npc id"), 2101);
            }
            other => panic!("expected packet, got {other:?}"),
        }

        assert_opcode(second_msg, SendOpcode::SpawnPlayer);
    }
}
