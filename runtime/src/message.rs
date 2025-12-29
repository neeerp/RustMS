use packet::Packet;

/// Unique identifier for a connected client.
/// Uses character_id for world server clients.
pub type ClientId = i32;

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
}

/// Defines who should receive a broadcast message.
#[derive(Debug, Clone)]
pub enum BroadcastScope {
    /// All players on a specific map
    Map(i32),
    /// All players on a map except the sender
    MapExcludeSelf(i32),
    /// All players in the world/channel
    World,
    /// All players in the world except the sender
    WorldExcludeSelf,
    // Future: Party(i32), Guild(i32), Nearby(i32, i16, i16), etc.
}
