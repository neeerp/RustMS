use net::{BroadcastScope, ClientId};
use packet::Packet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FieldKey {
    pub channel_id: u8,
    pub map_id: i32,
    pub instance_id: u32,
}

#[derive(Clone, Debug)]
pub struct FieldCharacter {
    pub id: i32,
    pub name: String,
    pub level: i16,
    pub job: i16,
    pub face: i32,
    pub hair: i32,
    pub skin: i32,
    pub gender: i16,
    pub map_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
}

impl FieldCharacter {
    pub fn field_key(&self) -> FieldKey {
        FieldKey {
            channel_id: 0,
            map_id: self.map_id,
            instance_id: 0,
        }
    }
}

/// Messages sent TO a client from the server or other clients.
#[derive(Debug)]
pub enum ServerMessage {
    /// Send a packet to this client
    SendPacket(Packet),
    /// Forcibly disconnect with reason
    Kick(String),
    /// Server is shutting down
    Shutdown,
}

/// Events sent FROM a client TO the world server.
#[derive(Debug)]
pub enum ClientEvent {
    /// Client has connected and is ready to receive messages
    Connected {
        client_id: ClientId,
        sender: tokio::sync::mpsc::Sender<ServerMessage>,
        character: FieldCharacter,
    },
    /// Client has disconnected
    Disconnected { client_id: ClientId },
    /// Client changed maps
    MapChanged {
        client_id: ClientId,
        old_map_id: i32,
        new_map_id: i32,
        spawn_portal_id: Option<u8>,
        spawn_x: Option<i16>,
        spawn_y: Option<i16>,
        spawn_stance: Option<u8>,
    },
    /// Request to broadcast a packet
    Broadcast {
        from: ClientId,
        scope: BroadcastScope,
        packet: Packet,
    },
    /// Request to broadcast local field chat.
    FieldChat { from: ClientId, packet: Packet },
    /// Request to broadcast local field movement.
    FieldMove {
        from: ClientId,
        packet: Packet,
        movement_bytes: Vec<u8>,
    },
    /// Request to deliver a whisper to a named online player.
    Whisper {
        from: ClientId,
        target_name: String,
        recipient_packet: Packet,
        sender_success_packet: Packet,
        sender_failure_packet: Packet,
    },
}

#[derive(Debug)]
pub enum FieldMessage {
    Join {
        client_id: ClientId,
        sender: tokio::sync::mpsc::Sender<ServerMessage>,
        character: FieldCharacter,
    },
    Leave {
        client_id: ClientId,
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
}
