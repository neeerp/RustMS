//! Packet handler contracts and the opcode registry.
//!
//! Login and world handlers are separate traits with separate action types,
//! so neither server has to ignore the other's verbs. Each server has one
//! routing table that maps an opcode to its handler and to the session stage
//! the client must be in; the runtime dispatches through `login_route` /
//! `world_route` and never matches on opcodes itself.

use crate::combat::AttackKind;
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
    pub peer_ip: std::net::IpAddr,
}

impl LoginContext<'_> {
    pub fn session(&self) -> Result<&Session, NetworkError> {
        self.session.as_ref().ok_or(NetworkError::NotLoggedIn)
    }
}

#[derive(Debug)]
pub enum LoginAction {
    Reply(Packet),
    /// Replace the connection's session (handlers open and close sessions
    /// themselves so that acquiring one is atomic).
    SetSession(Option<Session>),
    /// Move the current session to a new state (e.g. terms accepted).
    SetSessionState(SessionState),
    /// Persist the world and channel the client picked.
    SelectChannel {
        world_id: u8,
        channel_id: u8,
    },
    /// Issue a world-entry ticket for an owned character.
    EnterWorld {
        character_id: i32,
    },
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
            (
                Op::ServerStatusRequest,
                LoggedIn,
                &login::ServerStatusHandler,
            ),
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

/// A party operation; the world directory owns parties and carries it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartyRequest {
    Create,
    /// Leave the party; the leader leaving disbands it.
    Leave,
    /// Accept an invitation to `party_id`.
    Accept {
        party_id: i32,
    },
    /// Invite the named character, creating a party first if needed.
    Invite {
        name: String,
    },
    Decline {
        inviter: String,
    },
    Expel {
        member_id: i32,
    },
    ChangeLeader {
        member_id: i32,
    },
    Chat {
        text: String,
    },
}

/// A buddy list operation, carried out by the client's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuddyRequest {
    /// Add the named character, asking them to add the player back.
    Add { name: String, group: String },
    /// Accept a request from `id`.
    Accept { id: i32 },
    /// Remove `id` from the list, or decline their request.
    Remove { id: i32 },
    /// Say something to the listed buddies.
    Chat { recipients: Vec<i32>, text: String },
}

/// Something a player asks of their Maple Messenger, which the world
/// directory owns.
#[derive(Debug)]
pub enum MessengerRequest {
    /// Open a new messenger (`messenger_id` 0) or accept an invitation.
    Open {
        messenger_id: i32,
        look: game::AvatarLook,
    },
    Leave,
    Invite {
        target: String,
    },
    Decline {
        inviter: String,
    },
    /// A line already formatted as `name : text`.
    Chat {
        line: String,
    },
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
    EnterGame {
        character_id: i32,
    },
    /// The player already moved to `map_id` (the handler updated it and sent
    /// the warp); move the client's field membership to match.
    ChangeField {
        map_id: i32,
        spawn: Option<SpawnPosition>,
    },
    FieldChat(Packet),
    FieldMove {
        packet: Packet,
        /// Where the path starts.
        start: (i16, i16),
        movement_bytes: Vec<u8>,
    },
    NpcInteraction(NpcInteraction),
    /// Start, complete, or forfeit a quest (v83 `QUEST_ACTION`).
    Quest(crate::quests::QuestRequest),
    /// Buy, sell, recharge at, or leave the shop the player has open.
    Shop(crate::shop::ShopRequest),
    /// The player touched the named script portal on their map.
    ScriptPortal {
        portal: String,
    },
    /// The player's worn equipment changed; refresh their field avatar and
    /// show the others this UPDATE_CHAR_LOOK.
    LookChanged(Packet),
    /// The buffs others see changed; refresh the player's field avatar and
    /// show the others this GIVE_FOREIGN_BUFF or CANCEL_FOREIGN_BUFF.
    BuffsChanged(Packet),
    /// Natural HP/MP recovery the client reports; the runtime checks timing.
    NaturalHeal {
        hp: i16,
        mp: i16,
    },
    /// Something for the player's field to do on their behalf.
    Field(FieldRequest),
    /// The player's HP reached 0.
    Died,
    Whisper(WhisperDelivery),
    Party(PartyRequest),
    /// A skill the player used reached party members; see `PartySkill`.
    PartySkill(PartySkill),
    Buddy(BuddyRequest),
    /// Raise (or drop) the fame of `target`, who must be on the player's map.
    GiveFame {
        target: i32,
        up: bool,
    },
    /// Tell the player where a character is (`/find`).
    Find {
        target_name: String,
    },
    Messenger(MessengerRequest),
    ChangeChannel {
        channel_id: u8,
    },
    /// Leave the field for the Cash Shop.
    EnterCashShop,
    /// Return from the Cash Shop to the map the player left.
    LeaveCashShop,
    /// Buy something or save the wish list, in the Cash Shop.
    CashShop(crate::cash_shop::CashShopRequest),
    Disconnect,
}

/// A skill a player used that reached their party: the members in the
/// slots set in `slots` (a bit each, the first slot highest, as USE_SKILL
/// has them) get its buffs, or `heal` HP from Heal, if they are alive and
/// in its area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartySkill {
    pub skill_id: i32,
    pub level: i32,
    pub slots: u8,
    pub heal: i32,
}

/// A request a player makes of their field, which owns its mobs.
#[derive(Debug)]
pub enum FieldRequest {
    /// The controller moved a mob.
    MobMove {
        object_id: i32,
        move_id: i16,
        start: (i16, i16),
        movement: Vec<u8>,
    },
    /// How far the player is from a firstAttack mob.
    MobDistance { object_id: i32, distance: i32 },
    /// The NPCs' controller chose an idle action for one: what followed the
    /// object id (action, line, and a path for an NPC that walks).
    NpcAction { object_id: i32, action: Vec<u8> },
    /// A validated attack: `broadcast` for the others, total damage per mob,
    /// and what it may put on each mob it hits and leaves alive.
    Attack {
        broadcast: Packet,
        hits: Vec<(i32, i64)>,
        inflicts: Vec<crate::skills::Inflict>,
    },
    /// The player's spell left a mist where they stand, facing left or
    /// not, that puts `inflict` on the mobs in it.
    Mist {
        skill_id: i32,
        level: i32,
        left: bool,
        inflict: crate::skills::Inflict,
    },
    /// Show everyone else on the map this packet.
    Broadcast(Packet),
    /// The player used Monster Magnet: show the others `effect`, and each
    /// mob it reached, caught or not; the caught ones chase the player.
    Magnet {
        effect: Packet,
        caught: Vec<(i32, bool)>,
    },
    /// The player used a skill that puts statuses on the mobs it reaches
    /// (`inflict`), or takes their buffs away, from `origin` (where the
    /// player stands if None): these mobs, as their client saw them.
    MobDebuff {
        skill_id: i32,
        level: i32,
        origin: Option<(i16, i16)>,
        mobs: Vec<i32>,
        inflict: crate::skills::Inflict,
    },
    /// Sit on the portable chair `item_id`, or stand up with 0.
    Chair(i32),
    /// Pick up the drop `object_id`, answered with `ServerMessage::PickedUp`;
    /// the player stands at (x, y).
    Pickup { object_id: i32, x: i16, y: i16 },
    /// Throw `loot` down where the player stands; anyone may pick it up at
    /// once. With `vanish` it falls and fades instead (quest items).
    Throw {
        loot: crate::loot::Loot,
        vanish: bool,
    },
    /// Hit the reactor `object_id` with `skill_id` (0 for a plain attack).
    HitReactor {
        object_id: i32,
        skill_id: i32,
        stance: i16,
    },
    /// Show the player the info window for `character_id`, who must be on
    /// their map (and may be themselves).
    CharInfo { character_id: i32 },
    /// Have `target`, if they are here, take `giver_name`'s fame.
    Fame {
        target: i32,
        giver_name: String,
        up: bool,
    },
    /// The player called up their skill's summon at `at`, facing left or
    /// not, for `lasts`, replacing the one their buff of that kind gave.
    /// With `appear` it plays its `summoned` animation, but not when it
    /// follows them to a new map.
    Summon {
        skill_id: i32,
        level: i32,
        at: (i16, i16),
        left: bool,
        lasts: std::time::Duration,
        appear: bool,
    },
    /// Send away the player's summon of this skill, or all of them.
    Unsummon {
        skill_id: Option<i32>,
        why: crate::packet::build::world::summon::Gone,
    },
    /// The player's client moved their summon.
    SummonMove {
        object_id: i32,
        start: (i16, i16),
        movement: Vec<u8>,
    },
    /// The player's summon attacked; it must keep to its skill's limits.
    SummonAttack {
        attack: crate::summons::SummonAttack,
        limits: Vec<crate::summons::Limit>,
    },
    /// A mob hurt the player's Puppet.
    SummonDamage(crate::summons::SummonDamage),
    /// The player's Beholder heals or buffs them, if it is theirs and not
    /// too soon; the others see it play `action`.
    Beholder {
        object_id: i32,
        action: u8,
        support: crate::summons::Support,
    },
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
        let table = [
            (Op::PlayerLoggedIn, PreGame(&world::PlayerLoggedInHandler)),
            (Op::PlayerMove, InGame(&world::PlayerMoveHandler)),
            (Op::ChangeChannel, InGame(&world::ChangeChannelHandler)),
            (Op::ChangeMap, InGame(&world::ChangeMapHandler)),
            (Op::EnterCashShop, InGame(&world::EnterCashShopHandler)),
            (
                Op::CashShopOperation,
                InGame(&world::CashShopOperationHandler),
            ),
            (
                Op::PlayerMapTransfer,
                InGame(&world::PlayerMapTransferHandler),
            ),
            (
                Op::PartySearchUpdate,
                InGame(&world::PartySearchUpdateHandler),
            ),
            (Op::ChangeKeybinds, InGame(&world::ChangeKeybindsHandler)),
            (
                Op::MonsterBookCover,
                InGame(&world::MonsterBookCoverHandler),
            ),
            (Op::AllChat, InGame(&world::AllChatHandler)),
            (Op::FaceExpression, InGame(&world::FaceExpressionHandler)),
            (Op::UseChair, InGame(&world::UseChairHandler)),
            (Op::CancelChair, InGame(&world::CancelChairHandler)),
            (Op::Whisper, InGame(&world::WhisperHandler)),
            (Op::PartyOperation, InGame(&world::PartyOperationHandler)),
            (
                Op::DenyPartyRequest,
                InGame(&world::DenyPartyRequestHandler),
            ),
            (Op::MultiChat, InGame(&world::MultiChatHandler)),
            (Op::BuddyListModify, InGame(&world::BuddyListModifyHandler)),
            (Op::Messenger, InGame(&world::MessengerHandler)),
            (Op::TalkToNpc, InGame(&world::NpcTalkHandler)),
            (Op::NpcTalkMore, InGame(&world::NpcTalkMoreHandler)),
            (Op::QuestAction, InGame(&world::QuestActionHandler)),
            (Op::NpcShopAction, InGame(&world::NpcShopActionHandler)),
            (
                Op::ChangeMapSpecial,
                InGame(&world::ChangeMapSpecialHandler),
            ),
            (Op::GatherItems, InGame(&world::GatherItemsHandler)),
            (Op::SortItems, InGame(&world::SortItemsHandler)),
            (Op::ItemMove, InGame(&world::ItemMoveHandler)),
            (Op::UseItem, InGame(&world::UseItemHandler)),
            (Op::UseSkillBook, InGame(&world::UseSkillBookHandler)),
            (Op::MoveLife, InGame(&world::MoveLifeHandler)),
            (Op::MobDistance, InGame(&world::MobDistanceHandler)),
            (Op::NpcAction, InGame(&world::NpcActionHandler)),
            (
                Op::CloseRangeAttack,
                InGame(&world::AttackHandler(AttackKind::Close)),
            ),
            (
                Op::RangedAttack,
                InGame(&world::AttackHandler(AttackKind::Ranged)),
            ),
            (
                Op::MagicAttack,
                InGame(&world::AttackHandler(AttackKind::Magic)),
            ),
            (Op::TakeDamage, InGame(&world::TakeDamageHandler)),
            (Op::ItemPickup, InGame(&world::ItemPickupHandler)),
            (Op::DamageReactor, InGame(&world::DamageReactorHandler)),
            (Op::HealOverTime, InGame(&world::HealOverTimeHandler)),
            (Op::SpendAp, InGame(&world::SpendApHandler)),
            (Op::SpendSp, InGame(&world::SpendSpHandler)),
            (Op::UseSkill, InGame(&world::UseSkillHandler)),
            (Op::CancelBuff, InGame(&world::CancelBuffHandler)),
            (Op::SkillEffect, InGame(&world::SkillEffectHandler)),
            (Op::ThrowGrenade, InGame(&world::ThrowGrenadeHandler)),
            (Op::MesoDrop, InGame(&world::MesoDropHandler)),
            (Op::CharInfoRequest, InGame(&world::CharInfoRequestHandler)),
            (Op::GiveFame, InGame(&world::GiveFameHandler)),
            (Op::MoveSummon, InGame(&world::MoveSummonHandler)),
            (Op::SummonAttack, InGame(&world::SummonAttackHandler)),
            (Op::DamageSummon, InGame(&world::DamageSummonHandler)),
            (Op::Beholder, InGame(&world::BeholderHandler)),
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
        assert_eq!(world_routes().len(), 53);
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
