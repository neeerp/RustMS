//! Packet handler contracts and the opcode registry.
//!
//! Login and world handlers are separate traits with separate action types,
//! so neither server has to ignore the other's verbs. Each server has one
//! routing table that maps an opcode to its handler and to the session stage
//! the client must be in; the runtime dispatches through `login_route` /
//! `world_route` and never matches on opcodes itself.

use crate::dialogue::NpcInteraction;
use crate::error::NetworkError;
use crate::packet::handle::{login, world};
use crate::packet::op::RecvOpcode;
use db::session::{Session, SessionState};
use game::Player;
use packet::Packet;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Unique identifier for a connected world client (its character id).
pub type ClientId = i32;

/// Actions requested by a handler, in order.
#[derive(Debug)]
pub struct HandlerResult<A> {
    pub actions: Vec<A>,
}

/// Every action type can carry a plain reply to the requesting client.
pub trait ReplyAction {
    fn reply(packet: Packet) -> Self;
}

impl<A: ReplyAction> HandlerResult<A> {
    pub fn empty() -> Self {
        Self { actions: vec![] }
    }

    pub fn reply(packet: Packet) -> Self {
        Self::empty().with_reply(packet)
    }

    pub fn with_reply(self, packet: Packet) -> Self {
        self.with(A::reply(packet))
    }

    pub fn with(mut self, action: A) -> Self {
        self.actions.push(action);
        self
    }
}

// ---------------------------------------------------------------------------
// Login server
// ---------------------------------------------------------------------------

/// What a login handler can see. Login handlers may touch the database; the
/// runtime runs them on the blocking pool.
pub struct LoginContext<'a> {
    pub session: &'a Option<Session>,
}

impl LoginContext<'_> {
    pub fn session(&self) -> Result<&Session, NetworkError> {
        self.session.as_ref().ok_or(NetworkError::NotLoggedIn)
    }
}

#[derive(Debug)]
pub enum LoginAction {
    Reply(Packet),
    /// Open a session for an authenticated account.
    CreateSession {
        account_id: i32,
        hwid: String,
        state: SessionState,
    },
    /// Move the current session to a new state (e.g. terms accepted).
    SetSessionState(SessionState),
    /// Persist the world and channel the client picked.
    SelectChannel { world_id: u8, channel_id: u8 },
    /// Issue a world-entry ticket for an owned character.
    EnterWorld { character_id: i32 },
    Disconnect,
}

impl ReplyAction for LoginAction {
    fn reply(packet: Packet) -> Self {
        LoginAction::Reply(packet)
    }
}

pub type LoginResult = Result<HandlerResult<LoginAction>, NetworkError>;

pub trait LoginHandler: Send + Sync {
    fn handle(&self, packet: &mut Packet, ctx: &LoginContext) -> LoginResult;
}

/// Session requirement for a login opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginStage {
    /// Allowed before authentication.
    Any,
    /// Requires a session, including one still accepting terms.
    Session,
    /// Requires a fully logged-in session.
    LoggedIn,
}

impl LoginStage {
    pub fn admits(self, session: &Option<Session>) -> bool {
        match self {
            LoginStage::Any => true,
            LoginStage::Session => session.is_some(),
            LoginStage::LoggedIn => session
                .as_ref()
                .is_some_and(|s| s.state == SessionState::AfterLogin),
        }
    }
}

pub struct LoginRoute {
    pub stage: LoginStage,
    pub handler: &'static dyn LoginHandler,
}

fn login_routes() -> &'static HashMap<i16, LoginRoute> {
    static ROUTES: OnceLock<HashMap<i16, LoginRoute>> = OnceLock::new();
    ROUTES.get_or_init(|| {
        use LoginStage::*;
        use RecvOpcode as Op;
        let table: [(Op, LoginStage, &'static dyn LoginHandler); 13] = [
            (Op::LoginStarted, Any, &login::LoginStartHandler),
            (Op::LoginCredentials, Any, &login::LoginCredentialsHandler),
            (Op::GuestLogin, Any, &login::GuestLoginHandler),
            (Op::AcceptTOS, Session, &login::AcceptTOSHandler),
            (Op::SetGender, Session, &login::SetGenderHandler),
            (Op::ServerListRequest, LoggedIn, &login::WorldListHandler),
            (Op::ServerListReRequest, LoggedIn, &login::WorldListHandler),
            (Op::ServerStatusRequest, LoggedIn, &login::ServerStatusHandler),
            (Op::CharListRequest, LoggedIn, &login::CharListHandler),
            (Op::CheckCharName, LoggedIn, &login::CheckCharNameHandler),
            (Op::CreateChar, LoggedIn, &login::CreateCharacterHandler),
            (Op::DeleteChar, LoggedIn, &login::DeleteCharHandler),
            (Op::CharSelect, LoggedIn, &login::CharacterSelectHandler),
        ];
        table
            .into_iter()
            .map(|(op, stage, handler)| (op as i16, LoginRoute { stage, handler }))
            .collect()
    })
}

/// The route for a login opcode, if one is registered.
pub fn login_route(opcode: i16) -> Option<&'static LoginRoute> {
    login_routes().get(&opcode)
}

// ---------------------------------------------------------------------------
// World server
// ---------------------------------------------------------------------------

/// What an in-game world handler can see: the player it acts on. World
/// handlers are pure (no database or network IO) and run inline on the
/// client actor; anything with side effects is returned as an action.
pub struct WorldContext<'a> {
    pub client_id: ClientId,
    pub channel_id: u8,
    pub player: &'a mut Player,
}

/// A whisper ready for delivery; the world directory picks which packet the
/// sender gets back.
#[derive(Debug)]
pub struct WhisperDelivery {
    pub target_name: String,
    pub recipient_packet: Packet,
    pub sender_success_packet: Packet,
    pub sender_failure_packet: Packet,
}

/// Where a character appears after changing maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnPosition {
    pub x: i16,
    pub y: i16,
    pub stance: u8,
}

#[derive(Debug)]
pub enum WorldAction {
    Reply(Packet),
    /// Enter the game with the character named in PLAYER_LOGGEDIN.
    EnterGame { character_id: i32 },
    /// The player already moved to `map_id` (the handler updated it and sent
    /// the warp); move the client's field membership to match.
    ChangeField {
        map_id: i32,
        spawn: Option<SpawnPosition>,
    },
    FieldChat(Packet),
    FieldMove {
        packet: Packet,
        movement_bytes: Vec<u8>,
    },
    NpcInteraction(NpcInteraction),
    Whisper(WhisperDelivery),
    ChangeChannel { channel_id: u8 },
    Disconnect,
}

impl ReplyAction for WorldAction {
    fn reply(packet: Packet) -> Self {
        WorldAction::Reply(packet)
    }
}

pub type WorldResult = Result<HandlerResult<WorldAction>, NetworkError>;

/// Handles packets from a client that has entered the game.
pub trait WorldHandler: Send + Sync {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult;
}

/// Handles packets from a client that has connected but not entered yet.
pub trait PreGameHandler: Send + Sync {
    fn handle(&self, packet: &mut Packet) -> WorldResult;
}

pub enum WorldRoute {
    PreGame(&'static dyn PreGameHandler),
    InGame(&'static dyn WorldHandler),
}

fn world_routes() -> &'static HashMap<i16, WorldRoute> {
    static ROUTES: OnceLock<HashMap<i16, WorldRoute>> = OnceLock::new();
    ROUTES.get_or_init(|| {
        use RecvOpcode as Op;
        use WorldRoute::*;
        let table: [(Op, WorldRoute); 11] = [
            (Op::PlayerLoggedIn, PreGame(&world::PlayerLoggedInHandler)),
            (Op::PlayerMove, InGame(&world::PlayerMoveHandler)),
            (Op::ChangeChannel, InGame(&world::ChangeChannelHandler)),
            (Op::ChangeMap, InGame(&world::ChangeMapHandler)),
            (Op::PlayerMapTransfer, InGame(&world::PlayerMapTransferHandler)),
            (Op::PlayerUpdate, InGame(&world::PlayerUpdateHandler)),
            (Op::ChangeKeybinds, InGame(&world::ChangeKeybindsHandler)),
            (Op::AllChat, InGame(&world::AllChatHandler)),
            (Op::Whisper, InGame(&world::WhisperHandler)),
            (Op::TalkToNpc, InGame(&world::NpcTalkHandler)),
            (Op::NpcTalkMore, InGame(&world::NpcTalkMoreHandler)),
        ];
        table
            .into_iter()
            .map(|(op, route)| (op as i16, route))
            .collect()
    })
}

/// The route for a world opcode, if one is registered.
pub fn world_route(opcode: i16) -> Option<&'static WorldRoute> {
    world_routes().get(&opcode)
}

/// Defines who should receive a world-wide broadcast.
#[derive(Debug, Clone)]
pub enum BroadcastScope {
    /// All players in the world.
    World,
    /// All players in the world except the sender.
    WorldExcludeSelf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_opcode_is_unique_per_server() {
        // The maps would silently drop duplicates; check the table sizes match.
        assert_eq!(login_routes().len(), 13);
        assert_eq!(world_routes().len(), 11);
    }

    #[test]
    fn character_flow_requires_a_logged_in_session() {
        let route = login_route(RecvOpcode::CharSelect as i16).unwrap();
        assert_eq!(route.stage, LoginStage::LoggedIn);
        assert!(!route.stage.admits(&None));
        assert!(login_route(RecvOpcode::LoginCredentials as i16)
            .unwrap()
            .stage
            .admits(&None));
    }

    #[test]
    fn only_player_login_is_allowed_before_entering_the_game() {
        let pre_game: Vec<_> = world_routes()
            .iter()
            .filter(|(_, route)| matches!(route, WorldRoute::PreGame(_)))
            .map(|(&op, _)| op)
            .collect();
        assert_eq!(pre_game, vec![RecvOpcode::PlayerLoggedIn as i16]);
    }
}
