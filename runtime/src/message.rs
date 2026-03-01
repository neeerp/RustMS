use net::{BroadcastScope, ClientId};
use packet::Packet;

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
        map_id: i32,
        character_name: String,
    },
    /// Client has disconnected
    Disconnected { client_id: ClientId },
    /// Client changed maps
    MapChanged {
        client_id: ClientId,
        old_map_id: i32,
        new_map_id: i32,
    },
    /// Request to broadcast a packet
    Broadcast {
        from: ClientId,
        scope: BroadcastScope,
        packet: Packet,
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
