use crate::error::NetworkError;
use crate::helpers::to_hex_string;
use db::session::SessionWrapper;
use packet::Packet;

/// Unique identifier for a connected client.
/// Uses character_id for world server clients.
pub type ClientId = i32;

/// Trait for packet handlers.
/// Handlers take a packet and context, returning actions to perform.
pub trait PacketHandler: Send + Sync {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError>;
}

/// Default handler that logs unknown packets.
pub struct DefaultHandler;

impl PacketHandler for DefaultHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        _ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let op = packet.opcode();
        println!("Opcode: {}", op);
        println!("Received packet: {}", to_hex_string(&packet.bytes));
        Err(NetworkError::UnsupportedOpcodeError(op))
    }
}

use crate::listener::ServerType;
use crate::packet::op::RecvOpcode;

/// Get the packet handler for the given opcode and server type.
pub fn get_handler(op: i16, server_type: &ServerType) -> Box<dyn PacketHandler> {
    match server_type {
        ServerType::Login => get_login_handler(op),
        ServerType::World => get_world_handler(op),
    }
}

fn get_login_handler(op: i16) -> Box<dyn PacketHandler> {
    use crate::packet::handle::login;

    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::LoginCredentials) => Box::new(login::LoginCredentialsHandler::new()),
        Some(RecvOpcode::GuestLogin) => Box::new(login::GuestLoginHandler::new()),
        Some(RecvOpcode::ServerListReRequest) => Box::new(login::WorldListHandler::new()),
        Some(RecvOpcode::CharListRequest) => Box::new(login::CharListHandler::new()),
        Some(RecvOpcode::ServerStatusRequest) => Box::new(login::ServerStatusHandler::new()),
        Some(RecvOpcode::AcceptTOS) => Box::new(login::AcceptTOSHandler::new()),
        Some(RecvOpcode::SetGender) => Box::new(login::SetGenderHandler::new()),
        Some(RecvOpcode::AfterLogin) => Box::new(DefaultHandler),
        Some(RecvOpcode::RegisterPin) => Box::new(DefaultHandler),
        Some(RecvOpcode::ServerListRequest) => Box::new(login::WorldListHandler::new()),
        Some(RecvOpcode::ViewAllChar) => Box::new(DefaultHandler),
        Some(RecvOpcode::PickAllChar) => Box::new(DefaultHandler),
        Some(RecvOpcode::CharSelect) => Box::new(login::CharacterSelectHandler::new()),
        Some(RecvOpcode::CheckCharName) => Box::new(login::CheckCharNameHandler::new()),
        Some(RecvOpcode::CreateChar) => Box::new(login::CreateCharacterHandler::new()),
        Some(RecvOpcode::DeleteChar) => Box::new(login::DeleteCharHandler::new()),
        Some(RecvOpcode::RegisterPic) => Box::new(DefaultHandler),
        Some(RecvOpcode::CharSelectWithPic) => Box::new(DefaultHandler),
        Some(RecvOpcode::ViewAllPicRegister) => Box::new(DefaultHandler),
        Some(RecvOpcode::ViewAllWithPic) => Box::new(DefaultHandler),
        Some(RecvOpcode::LoginStarted) => Box::new(login::LoginStartHandler::new()),
        None | Some(_) => Box::new(DefaultHandler),
    }
}

fn get_world_handler(op: i16) -> Box<dyn PacketHandler> {
    use crate::packet::handle::world;

    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::PlayerMove) => Box::new(world::PlayerMoveHandler::new()),
        Some(RecvOpcode::PlayerLoggedIn) => Box::new(world::PlayerLoggedInHandler::new()),
        Some(RecvOpcode::PlayerMapTransfer) => Box::new(world::PlayerMapTransferHandler::new()),
        Some(RecvOpcode::ChangeMap) => Box::new(world::ChangeMapHandler::new()),
        Some(RecvOpcode::PartySearch) => Box::new(world::PartySearchHandler::new()),
        Some(RecvOpcode::ChangeKeybinds) => Box::new(world::ChangeKeybindsHandler::new()),
        Some(RecvOpcode::AllChat) => Box::new(world::AllChatHandler::new()),
        Some(RecvOpcode::Whisper) => Box::new(world::WhisperHandler::new()),
        None | Some(_) => Box::new(DefaultHandler),
    }
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

/// Context available to packet handlers.
/// Provides access to session data without exposing the network stream.
pub struct HandlerContext<'a> {
    /// The client's identifier (character_id for world server)
    pub client_id: ClientId,
    /// Session and character data
    pub session: &'a mut SessionWrapper,
}

use db::session::SessionState;

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
    /// Create a new session (login server only)
    /// The actor will fill in network-specific details (IP address)
    CreateSession {
        account_id: i32,
        hwid: String,
        state: SessionState,
    },
    /// Attach a character to the current session
    AttachCharacter { character_id: i32 },
    /// Reattach session from login server (world server)
    ReattachSession { character_id: i32 },
    /// Deliver a directed whisper to another player.
    Whisper {
        target_name: String,
        recipient_packet: Packet,
        sender_success_packet: Packet,
        sender_failure_packet: Packet,
    },
    /// Broadcast local chat to the client's current field.
    FieldChat { packet: Packet },
    /// Broadcast player movement to the client's current field.
    FieldMove {
        packet: Packet,
        movement_bytes: Vec<u8>,
    },
    /// Notify runtime that this client changed maps.
    MapChanged { old_map_id: i32, new_map_id: i32 },
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
        self.actions
            .push(HandlerAction::Broadcast { scope, packet });
        self
    }

    /// Add a disconnect action to this result.
    pub fn with_disconnect(mut self) -> Self {
        self.actions.push(HandlerAction::Disconnect);
        self
    }

    /// Add a create session action (for login).
    pub fn with_create_session(
        mut self,
        account_id: i32,
        hwid: String,
        state: SessionState,
    ) -> Self {
        self.actions.push(HandlerAction::CreateSession {
            account_id,
            hwid,
            state,
        });
        self
    }

    /// Add an attach character action.
    pub fn with_attach_character(mut self, character_id: i32) -> Self {
        self.actions
            .push(HandlerAction::AttachCharacter { character_id });
        self
    }

    /// Add a reattach session action (for world server).
    pub fn with_reattach_session(mut self, character_id: i32) -> Self {
        self.actions
            .push(HandlerAction::ReattachSession { character_id });
        self
    }

    /// Add a whisper delivery action.
    pub fn with_whisper(
        mut self,
        target_name: String,
        recipient_packet: Packet,
        sender_success_packet: Packet,
        sender_failure_packet: Packet,
    ) -> Self {
        self.actions.push(HandlerAction::Whisper {
            target_name,
            recipient_packet,
            sender_success_packet,
            sender_failure_packet,
        });
        self
    }

    /// Add a local field-chat action.
    pub fn with_field_chat(mut self, packet: Packet) -> Self {
        self.actions.push(HandlerAction::FieldChat { packet });
        self
    }

    /// Add a local field-move action.
    pub fn with_field_move(mut self, packet: Packet, movement_bytes: Vec<u8>) -> Self {
        self.actions.push(HandlerAction::FieldMove {
            packet,
            movement_bytes,
        });
        self
    }

    /// Notify runtime that this client changed maps.
    pub fn with_map_changed(mut self, old_map_id: i32, new_map_id: i32) -> Self {
        self.actions.push(HandlerAction::MapChanged {
            old_map_id,
            new_map_id,
        });
        self
    }
}
