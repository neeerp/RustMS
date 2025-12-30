use crate::error::NetworkError;
use crate::helpers::to_hex_string;
use db::session::SessionWrapper;
use packet::Packet;

/// Unique identifier for a connected client.
/// Uses character_id for world server clients.
pub type ClientId = i32;

/// Trait for async packet handlers.
/// Handlers take a packet and context, returning actions to perform.
pub trait AsyncPacketHandler: Send + Sync {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError>;
}

/// Default handler that logs unknown packets.
pub struct DefaultAsyncHandler;

impl AsyncPacketHandler for DefaultAsyncHandler {
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

/// Get the async packet handler for the given opcode and server type.
pub fn get_async_handler(op: i16, server_type: &ServerType) -> Box<dyn AsyncPacketHandler> {
    match server_type {
        ServerType::Login => get_async_login_handler(op),
        ServerType::World => get_async_world_handler(op),
    }
}

fn get_async_login_handler(op: i16) -> Box<dyn AsyncPacketHandler> {
    use crate::packet::handle::login;

    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::LoginCredentials) => Box::new(login::AsyncLoginCredentialsHandler::new()),
        Some(RecvOpcode::GuestLogin) => Box::new(login::AsyncGuestLoginHandler::new()),
        Some(RecvOpcode::ServerListReRequest) => Box::new(login::AsyncWorldListHandler::new()),
        Some(RecvOpcode::CharListRequest) => Box::new(login::AsyncCharListHandler::new()),
        Some(RecvOpcode::ServerStatusRequest) => Box::new(login::AsyncServerStatusHandler::new()),
        Some(RecvOpcode::AcceptTOS) => Box::new(login::AsyncAcceptTOSHandler::new()),
        Some(RecvOpcode::SetGender) => Box::new(login::AsyncSetGenderHandler::new()),
        Some(RecvOpcode::AfterLogin) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::RegisterPin) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::ServerListRequest) => Box::new(login::AsyncWorldListHandler::new()),
        Some(RecvOpcode::ViewAllChar) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::PickAllChar) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::CharSelect) => Box::new(login::AsyncCharacterSelectHandler::new()),
        Some(RecvOpcode::CheckCharName) => Box::new(login::AsyncCheckCharNameHandler::new()),
        Some(RecvOpcode::CreateChar) => Box::new(login::AsyncCreateCharacterHandler::new()),
        Some(RecvOpcode::DeleteChar) => Box::new(login::AsyncDeleteCharHandler::new()),
        Some(RecvOpcode::RegisterPic) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::CharSelectWithPic) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::ViewAllPicRegister) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::ViewAllWithPic) => Box::new(DefaultAsyncHandler),
        Some(RecvOpcode::LoginStarted) => Box::new(login::AsyncLoginStartHandler::new()),
        None | Some(_) => Box::new(DefaultAsyncHandler),
    }
}

fn get_async_world_handler(op: i16) -> Box<dyn AsyncPacketHandler> {
    use crate::packet::handle::world;

    match num::FromPrimitive::from_i16(op) {
        Some(RecvOpcode::PlayerMove) => Box::new(world::AsyncPlayerMoveHandler::new()),
        Some(RecvOpcode::PlayerLoggedIn) => Box::new(world::AsyncPlayerLoggedInHandler::new()),
        Some(RecvOpcode::PlayerMapTransfer) => Box::new(world::AsyncPlayerMapTransferHandler::new()),
        Some(RecvOpcode::ChangeMap) => Box::new(world::AsyncChangeMapHandler::new()),
        Some(RecvOpcode::PartySearch) => Box::new(world::AsyncPartySearchHandler::new()),
        Some(RecvOpcode::ChangeKeybinds) => Box::new(world::AsyncChangeKeybindsHandler::new()),
        Some(RecvOpcode::AllChat) => Box::new(world::AsyncAllChatHandler::new()),
        None | Some(_) => Box::new(DefaultAsyncHandler),
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
        self.actions.push(HandlerAction::Broadcast { scope, packet });
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
        self.actions.push(HandlerAction::AttachCharacter { character_id });
        self
    }

    /// Add a reattach session action (for world server).
    pub fn with_reattach_session(mut self, character_id: i32) -> Self {
        self.actions.push(HandlerAction::ReattachSession { character_id });
        self
    }
}
