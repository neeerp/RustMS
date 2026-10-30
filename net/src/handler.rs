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
use crate::packet::build::world::party::PartyStatus;
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

/// A GM's hand on another character's account (row 205 S6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discipline {
    /// Block the account for `days` days, or for good, and disconnect it,
    /// giving `reason` (Etc/BlockReason.img's number, 0 for none). `slash`
    /// is v83's `/ban` or `/block`, whose GM hears ADMIN_RESULT 4 or 6
    /// rather than a line.
    Block {
        target: String,
        reason: u8,
        days: Option<u32>,
        note: String,
        slash: bool,
    },
    /// Lift the account's block.
    Unblock { target: String },
    /// Disconnect the character if they are online.
    Disconnect { target: String },
    /// Hold the character in the jail map for `minutes` more.
    Jail { target: String, minutes: u32 },
    /// Let the character out of jail.
    Release { target: String },
    /// Block the account from chatting for `minutes`, giving `reason`
    /// (Etc/ChatBlockReason.img's number, 1..=6).
    Mute {
        target: String,
        minutes: u32,
        reason: u8,
    },
    /// Lift the account's chat block.
    Unmute { target: String },
}

/// A whisper ready for delivery; the world directory picks which packet the
/// sender gets back, if any.
#[derive(Debug)]
pub struct WhisperDelivery {
    pub target_name: String,
    pub recipient_packet: Packet,
    pub sender_success_packet: Option<Packet>,
    pub sender_failure_packet: Option<Packet>,
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
    /// Turn down `inviter`'s invitation, telling them `status`. Busy with
    /// another invitation leaves this one waiting for that one's answer.
    Decline {
        inviter: String,
        status: PartyStatus,
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
    /// The leader starts a party search (`PARTY_SEARCH_START`): members of
    /// these levels and jobs (the window's checkboxes, one bit each), until
    /// the party has `members`.
    SearchStart {
        min_level: i32,
        max_level: i32,
        members: i32,
        jobs: u32,
    },
    /// Stop searching, sent also to pause and after any map change in the
    /// channel.
    SearchStop,
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

/// A note (memo) operation, carried out by the client's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteRequest {
    /// Leave the character named `to` a note, using up the note item
    /// `item_id` in Cash `slot`.
    Send {
        slot: i16,
        item_id: i32,
        to: String,
        text: String,
    },
    /// Send the notes waiting for the player.
    List,
    /// The player has read these notes: delete them.
    Delete { ids: Vec<i32> },
    /// Thank `to` with a note for the gift with cash serial `gift`.
    ThankForGift { to: String, text: String, gift: i64 },
}

/// An alliance operation (`ALLIANCE_OPERATION`), carried out by the
/// client's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllianceRequest {
    /// Send the alliance, with its guilds (op 1).
    Info,
    /// Take the player's guild out of the alliance (op 2).
    Leave,
    /// Invite the guild with this name (op 3).
    Invite { guild_name: String },
    /// Bring the player's guild into the alliance that invited it (op 4).
    Accept { alliance_id: i32 },
    /// Put a guild out of the alliance (op 6).
    Expel { guild_id: i32 },
    /// Hand the alliance to another guild's master (op 7).
    ChangeMaster { member_id: i32 },
    /// Give the ranks these titles (op 8).
    SetTitles { titles: [String; 5] },
    /// Move a member one rank up or down (op 9).
    SetRank { member_id: i32, up: bool },
    /// Post the alliance notice (op 10).
    SetNotice { notice: String },
    /// Say something to the alliance's members online.
    Chat { text: String },
    /// Turn down the invitation `inviter` sent (`DENY_ALLIANCE_REQUEST`).
    Deny { inviter: String },
}

/// A family request (report b039), carried out by the client's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FamilyRequest {
    /// The Family window opened (`OPEN_FAMILY`).
    Open,
    /// Show the Family Tree of the character named so.
    Chart { name: String },
    /// Ask the character named so to be the player's junior.
    Add { name: String },
    /// Answer `inviter`'s request to be their junior.
    Answer { inviter: ClientId, accept: bool },
    /// The leader's new precepts.
    Precepts { precepts: String },
    /// Part from the junior with this id, or from the player's senior.
    Separate { junior: Option<ClientId> },
    /// Buy entitlement `index`, naming `target` for those that need one.
    Use {
        index: usize,
        target: Option<String>,
    },
    /// Answer the summons the player was sent by `summoner`.
    SummonAnswer { summoner: String, accept: bool },
}

/// An engagement, carried out by the client's actor (report b052).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RingRequest {
    /// Propose to the character named so with the ring box `item_id`.
    Propose { name: String, item_id: i32 },
    /// Withdraw the proposal the player is waiting on.
    Cancel,
    /// Answer the proposal of the character with id `proposer`.
    Answer { proposer: ClientId, accept: bool },
    /// Break off by dropping the token `item_id`.
    Break { item_id: i32 },
    /// Invite the character named `guest` to the couple `marriage_id`'s
    /// wedding with the invitation in Etc `slot`.
    Invite {
        guest: String,
        marriage_id: i32,
        slot: i16,
    },
    /// Open the guest's invitation `item_id` in Etc `slot`.
    OpenInvitation { slot: i16, item_id: i32 },
    /// The groom's or bride's wishlist, from the window the greeter opened
    /// (report b085).
    Wishlist { wishes: Vec<String> },
}

/// What the player did in the Wedding Gift window (`WEDDING_ACTION`,
/// report b085).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GiftRequest {
    /// Give `quantity` of the item `item_id` in `slot` of its inventory.
    Give {
        slot: i16,
        item_id: i32,
        quantity: i16,
    },
    /// Take the gift at `index` in the list of inventory `kind` (1 equip to
    /// 5 cash) last shown.
    Take { kind: u8, index: u8 },
    /// Close the window.
    Close,
}

/// A guild operation, carried out by the client's actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildRequest {
    /// Send the player's guild again (`GUILD_OPERATION` mode 0).
    Reload,
    /// Found a guild with this name, answering the name prompt.
    Create {
        name: String,
    },
    /// Say something to the guild members online.
    Chat {
        text: String,
    },
    /// Agree, or not, to found the guild the party's leader named.
    Agree {
        agree: bool,
    },
    /// Ask the character with this name to join the guild.
    Invite {
        name: String,
    },
    /// Join the guild that invited the player.
    Accept {
        guild_id: i32,
    },
    /// Turn down the invitation `inviter` sent.
    Deny {
        inviter: String,
        refusal: crate::packet::build::world::guild::InviteRefusal,
    },
    Leave,
    /// Put a member out of the guild.
    Expel {
        member_id: i32,
    },
    /// Give the ranks these titles.
    SetTitles {
        titles: [String; 5],
    },
    /// Move a member one rank up or down.
    SetRank {
        member_id: i32,
        rank: u8,
    },
    /// Post the guild notice.
    SetNotice {
        notice: String,
    },
    /// Give the guild the emblem made in Lea's emblem maker.
    SetEmblem {
        emblem: game::guild::Emblem,
    },
    /// Add to the guild's points (`!gp`, the Guild Quest).
    GainPoints {
        amount: i32,
    },
    Bbs(BbsRequest),
}

/// Something asked of the guild bulletin board. Threads are named by the
/// guild's number for them, and the notice is thread 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BbsRequest {
    /// Post a thread or the notice, or rewrite thread `edit`.
    Write {
        edit: Option<i32>,
        notice: bool,
        title: String,
        body: String,
        icon: i32,
    },
    Delete {
        thread: i32,
    },
    /// List a page of threads, counting from 0.
    List {
        page: i32,
    },
    Show {
        thread: i32,
    },
    Reply {
        thread: i32,
        text: String,
    },
    /// Delete a reply, named by the board's number for it.
    DeleteReply {
        reply: i32,
    },
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
    /// Turn down `inviter`'s invitation, `blocking` when the Game Option
    /// turned it away unseen.
    Decline {
        inviter: String,
        blocking: bool,
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
    /// Send `packet` `after` this long, as when the client animates a
    /// result before it may see it.
    ReplyLater {
        after: std::time::Duration,
        packet: Packet,
    },
    /// A GM opens or closes their channel's event (report b170).
    GmEntry(GmEntry),
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
    /// Open Mu Mu's shop for the Miu Miu in Cash `slot`, unless the player
    /// is trading, keeps a shop or has an NPC window open, and take the Miu
    /// Miu.
    MiuMiu {
        slot: i16,
    },
    /// Run a scripted item's `script` where the player stands
    /// (`scripts::item`).
    ScriptedItem {
        script: String,
    },
    /// Take out of, store in, sort, or close the storage the player has open.
    Storage(crate::storage::StorageRequest),
    /// Open Duey's window, send, claim, or delete a parcel, or close it.
    Duey(crate::duey::DueyRequest),
    /// A button of the Rock, Paper, Scissors window.
    Rps(crate::rps::RpsRequest),
    /// The Monster Carnival window spends CP on `index` of tab `tab`.
    CarnivalSpend {
        tab: u8,
        index: u8,
    },
    /// Open, join, or act in a trading room.
    Trade(crate::trade::TradeRequest),
    /// Search the Free Market's shops with the Owl of Minerva, or go to
    /// one it found.
    Owl(crate::owl::OwlRequest),
    /// Open, stock, visit, or buy from a personal shop.
    PersonalShop(crate::personal_shop::PersonalShopRequest),
    MiniGame(crate::mini_game::MiniGameRequest),
    /// Hire, stock, keep, close, or buy from a hired merchant.
    HiredMerchant(crate::hired_merchant::HiredMerchantRequest),
    /// Take everything from Fredrick's Store Bank, or close it.
    StoreBank(crate::store_bank::StoreBankRequest),
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
    /// The player summoned or sent back pets (`player.pets` already
    /// changed); show everyone on the map.
    PetsChanged(Vec<game::pet::PetChange>),
    /// The player held up or put away a chalkboard (`player.chalkboard`
    /// already changed); refresh their field avatar and show the others
    /// this CHALKBOARD.
    ChalkboardChanged(Packet),
    /// Berserk turned on or off (`player.berserk` already changed); refresh
    /// the player's field avatar and show the others this
    /// SHOW_FOREIGN_EFFECT.
    BerserkChanged(Packet),
    /// The player showed or stopped an effect item (`player.item_effect`
    /// already changed); refresh their field avatar and show the others
    /// this SHOW_ITEM_EFFECT.
    ItemEffectChanged(Packet),
    /// The GM hid or came out of hiding (`player.hidden` already changed);
    /// the field takes the GM from, or shows the GM to, those not GMs.
    HiddenChanged,
    /// Close the NPC dialog, shop or storage the player has open, as a new
    /// NPC click would.
    CloseDialogs,
    /// A GM's `!givenx`: add `amount` to the `cash_type` balance of the
    /// account of the character named `target` (online or not), or of the
    /// GM's own account, and tell the GM the new balance.
    GiveCash {
        target: Option<String>,
        cash_type: db::cash_shop::CashType,
        amount: i32,
    },
    /// A GM's block, unblock or disconnect of another account.
    Discipline(Discipline),
    /// A GM's `!coupon`: make a Cash Shop coupon giving `rewards`, with
    /// `code` or else a new random one, good for `days` days or forever.
    MakeCoupon {
        code: Option<String>,
        days: Option<u32>,
        rewards: Vec<db::coupon::Reward>,
    },
    /// The player's level or job changed (`player.stats` already did);
    /// refresh their field avatar and show the others these packets.
    AvatarChanged(Vec<Packet>),
    /// Look on the channel for the character named `name`, to take the
    /// player to them with `rock`.
    RockToCharacter {
        name: String,
        rock: crate::packet::handle::world::Rock,
    },
    /// Look on the channel for the character a Party Potion names, to
    /// morph them.
    MorphAnother(crate::packet::handle::world::PartyPotion),
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
    Note(NoteRequest),
    Guild(GuildRequest),
    Alliance(AllianceRequest),
    Family(FamilyRequest),
    Ring(RingRequest),
    /// One of the couple closed the priest's line `step` at the altar
    /// (report b075).
    WeddingTalk {
        step: u8,
    },
    /// A guest at the Cathedral's altar blessed the couple (report b075).
    WeddingBless,
    WeddingGift(GiftRequest),
    /// The player cast Mystic Door at `at` on their map, leading to `town`,
    /// for `lasts`. It replaces any door they had open.
    OpenDoor {
        town: i32,
        at: (i16, i16),
        lasts: std::time::Duration,
    },
    /// The player pressed up in `owner`'s Mystic Door, in its town or in its
    /// field.
    UseDoor {
        owner: i32,
        from_town: bool,
    },
    /// Report `victim` to the GMs: a conversation (with `chat_log`) or an
    /// illegal program.
    Claim {
        conversation: bool,
        victim: String,
        reason: u8,
        description: String,
        chat_log: String,
    },
    /// Raise (or drop) the fame of `target`, who must be on the player's map.
    GiveFame {
        target: i32,
        up: bool,
    },
    /// Tell the player where a character is (`/find`).
    Find {
        target_name: String,
    },
    /// Send `packet` to every player `scope` covers, such as a megaphone's
    /// line.
    Broadcast {
        scope: BroadcastScope,
        packet: Packet,
    },
    Messenger(MessengerRequest),
    /// Queue the banner of the avatar megaphone `item_id` in Cash `slot`
    /// for the whole world; the item is used up only if it's queued
    /// (`megaphone::spend_avatar_megaphone`).
    AvatarMegaphone {
        slot: i16,
        item_id: i32,
        banner: Packet,
    },
    /// Put a Maple TV message, from the item in Cash `slot`, in the TV's
    /// line; the item is used up only if it gets in
    /// (`maple_tv::spend_maple_tv`).
    MapleTv {
        slot: i16,
        tv: crate::maple_tv::MapleTv,
    },
    ChangeChannel {
        channel_id: u8,
    },
    /// Add `amount` EXP from outside a kill or quest (gacha EXP), levelling
    /// up as far as it covers, with the stat update but no message.
    GainExp(i32),
    /// Tell the player's event (`ClientActor::tell_event`).
    Event(crate::events::EventCommand),
    /// Leave the field for the Cash Shop.
    EnterCashShop,
    /// Return from the Cash Shop to the map the player left.
    LeaveCashShop,
    /// Buy something or save the wish list, in the Cash Shop.
    CashShop(crate::cash_shop::CashShopRequest),
    /// Cancel the character's Name Change request with the receipt in Cash
    /// `slot`, or say there is none.
    CancelNameChange {
        slot: i16,
    },
    Disconnect,
}

/// A skill a player used that reached their party: the members in the
/// slots set in `slots` (a bit each, the first slot highest, as USE_SKILL
/// has them) get its buffs, or `heal` HP from Heal, if they are alive and
/// in its area. Echo of Hero reaches everyone on the map instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartySkill {
    pub skill_id: i32,
    pub level: i32,
    pub slots: u8,
    pub heal: i32,
}

/// What a GM's `/start` or `/reset` chat line asks of the event on their
/// field (report b169).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GmEvent {
    Start,
    Reset,
}

impl GmEvent {
    /// The event command a GM's '/' line names, if any.
    pub fn parse(line: &str) -> Option<Self> {
        let command = line.split_whitespace().next()?.to_ascii_lowercase();
        match command.as_str() {
            "/start" => Some(Self::Start),
            "/reset" => Some(Self::Reset),
            _ => None,
        }
    }
}

/// What a GM's `/snowball`, `/coconut`, `/ox`, `/fitness`, `/ola` or `/endevent`
/// chat line asks of their channel's event: open it to players, or close it
/// (report b170).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GmEntry {
    /// Event NPCs take up to `limit` players to the game field `map_id`.
    Open {
        map_id: i32,
        limit: u16,
    },
    Close,
}

impl GmEntry {
    /// How many players an event takes unless the GM says (Cosmic).
    pub const LIMIT: u16 = 50;

    /// The entry command a GM's '/' line names, if any: the game and its
    /// field, then a limit if they give one.
    pub fn parse(line: &str) -> Option<Self> {
        let mut words = line.split_whitespace();
        let command = words.next()?.to_ascii_lowercase();
        let map_id = match command.as_str() {
            "/endevent" => return Some(Self::Close),
            "/snowball" | "/snowball1" => 109_060_000,
            "/snowball2" => 109_060_002,
            "/snowball3" => 109_060_004,
            "/coconut" | "/coconut1" => 109_080_000,
            "/coconut2" => 109_080_001,
            "/coconut3" => 109_080_002,
            "/ox" => 109_020_001,
            // Four words, one test (report b172).
            "/fitness1" | "/fitness2" | "/fitness3" | "/fitness4" => 109_040_000,
            // The first digit is the variant (report b172); what the second
            // means isn't known, so each opens the variant's first level.
            "/ola10" | "/ola11" | "/ola12" => 109_030_001,
            // v83 has no word for it (report b172); this one is ours.
            "/jewel" => 109_010_000,
            "/ola20" | "/ola21" | "/ola22" => 109_030_101,
            "/ola30" | "/ola31" | "/ola32" => 109_030_201,
            "/ola40" | "/ola41" | "/ola42" => 109_030_301,
            "/ola50" | "/ola51" | "/ola52" => 109_030_401,
            _ => return None,
        };
        let limit = match words.next() {
            Some(word) => word.parse().ok().filter(|limit| *limit > 0)?,
            None => Self::LIMIT,
        };
        Some(Self::Open { map_id, limit })
    }
}

/// A request a player makes of their field, which owns its mobs.
#[derive(Debug)]
pub enum FieldRequest {
    /// The controller moved a mob, maybe as it acted (`action` << 1 |
    /// facing left, 0xFF for none) with `skill_data`, and wants it to
    /// attack next (`want_attack`).
    MobMove {
        object_id: i32,
        move_id: i16,
        want_attack: bool,
        action: u8,
        skill_data: i32,
        start: (i16, i16),
        movement: Vec<u8>,
    },
    /// How far the player is from a firstAttack mob.
    MobDistance { object_id: i32, distance: i32 },
    /// A mob that bombs has a player in reach.
    MobBomb { object_id: i32 },
    /// A trap that hurts mobs hurt one for `damage`, as the client saw it.
    FieldDamageMob { object_id: i32, damage: i32 },
    /// A mob touched a friendly mob (`damagedByMob`), which the server
    /// hurts.
    MobDamageMobFriendly { attacker: i32, victim: i32 },
    /// A mob's attack hurt another, a Dazzled one, for `damage`.
    MobDamageMob {
        attacker: i32,
        victim: i32,
        damage: i32,
    },
    /// The NPCs' controller chose an idle action for one: what followed the
    /// object id (action, line, and a path for an NPC that walks).
    NpcAction { object_id: i32, action: Vec<u8> },
    /// A validated attack: `broadcast` for the others, its skill (0 for a
    /// basic attack), total damage per mob, and what it may put on each mob
    /// it hits and leaves alive, and what it takes from them. Each mob with
    /// the counter (PCounter, or MCounter for a spell) hurts the player
    /// back. Meso Explosion's `blast` sets off its drops, and its hits
    /// count only as far as they could.
    Attack {
        broadcast: Packet,
        skill: i32,
        counter: game::MobStat,
        /// How many damage lines it dealt each mob.
        lines: u8,
        hits: Vec<(i32, i64)>,
        inflicts: Vec<crate::skills::Inflict>,
        takes: crate::skills::Takes,
        blast: Option<crate::meso_explosion::Blast>,
        /// Pickpocket's mesos, per mob hit (`skills::pickpocket`).
        pockets: Vec<(i32, Vec<i32>)>,
    },
    /// Power Guard sent `damage` back at the mob `object_id` that touched
    /// the player, if it is still a `mob_id`.
    Reflect {
        object_id: i32,
        mob_id: i32,
        damage: i64,
    },
    /// The player's spell left a mist where they stand, facing left or
    /// not, that puts `inflict` on the mobs in it.
    Mist {
        skill_id: i32,
        level: i32,
        left: bool,
        inflict: crate::skills::Inflict,
    },
    /// The player used the cash mist item `item_id`: its cloud covers
    /// `area` around them for `lasts`, unless one of theirs from that item
    /// is still there or they stand in another's.
    ItemMist {
        item_id: i32,
        area: (i32, i32, i32, i32),
        lasts: std::time::Duration,
    },
    /// Show everyone else on the map who sees the player this packet,
    /// about them: a hidden GM's goes only to GMs.
    Broadcast(Packet),
    /// The player picked up an item that puts a mob skill's disease on
    /// everyone else here (the Crazy Skull 2022268).
    DiseaseOthers(game::Disease),
    /// Show everyone else on the map this packet, which is about the map,
    /// whoever sees the player.
    BroadcastAll(Packet),
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
    /// Blow the weather of the 512xxxx item `item_id` over the map with
    /// its `message`, in place of any that blows, for 30 s.
    Weather { item_id: i32, message: String },
    /// Fly the kite (508xxxx) `item_id` with the player's `message` above
    /// where they stand, for an hour.
    Kite { item_id: i32, message: String },
    /// The player's attack hit Coconut Season nut `index` (b167).
    CoconutHit { index: u16 },
    /// The player's attack hit a Snowball field's `target` (b168).
    SnowballHit { target: u8, delay: u16 },
    /// The player walked into a Snowball field's ball (b168).
    IntoSnowball,
    /// A GM runs the field's event (QUESTIONS b169 #1).
    GmEvent(GmEvent),
    /// Pick up the drop `object_id`, answered with `ServerMessage::PickedUp`;
    /// the player stands at (x, y).
    Pickup {
        object_id: i32,
        x: i16,
        y: i16,
        /// The quest drops the player may take.
        needs: crate::loot::QuestNeeds,
    },
    /// Throw `loot` down where the player stands; anyone may pick it up at
    /// once. With `vanish` it falls and fades instead (quest items).
    Throw {
        loot: crate::loot::Loot,
        vanish: bool,
    },
    /// `count` of `item_id`, knocked out of the player's bag by a mob's
    /// hit, fall one by one around them for anyone at once: the first at
    /// their feet, the rest further out to either side in turn.
    Shed { item_id: i32, count: i16 },
    /// Swing at the reactor `object_id`, or use `skill` by it. The `flags`
    /// are bit 0 facing left and bit 1 on a foothold; others see the hit
    /// `delay` ms later.
    HitReactor {
        object_id: i32,
        flags: i32,
        delay: u16,
        skill: Option<i32>,
    },
    /// The player stepped onto the reactor `object_id`, set off by touch, or
    /// off it.
    TouchReactor { object_id: i32, touching: bool },
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
    /// The player opened a summoning sack: bring out these mobs on the
    /// ground below them.
    SummonMobs(Vec<i32>),
    /// A script brings out `mob_id` on the ground below `at`, unless one
    /// is out already.
    BringOutMob { mob_id: i32, at: (i16, i16) },
    /// A script brings out NPC `npc_id` on the ground below `at`, unless
    /// it is here already.
    BringOutNpc { npc_id: i32, at: (i16, i16) },
    /// A script brings out a `mob_id` on the ground below each of `at`.
    BringOutMobs { mob_id: i32, at: Vec<(i16, i16)> },
    /// A script puts the reactors named `name` here in `state`
    /// (`scripts::Effect::SetReactorState`).
    SetReactorState { name: &'static str, state: u8 },
    /// A script opens the reactors named `name` here for as long as their
    /// open state lasts (`scripts::Effect::OpenReactor`).
    OpenReactor { name: &'static str },
    /// A script has NPC `npc_id` here play its special action `action`
    /// (`scripts::Effect::NpcSpecialAction`).
    NpcSpecialAction { npc_id: i32, action: &'static str },
    /// A script sends away every mob here that isn't friendly
    /// (`scripts::Effect::ClearMobs`).
    ClearMobs,
    /// The Snow Spirit brings out a Holiday PQ arena's snowman
    /// (`scripts::Effect::BringOutSnowman`).
    BringOutSnowman,
    /// The Snow Spirit brings out Scrooge where the whole snowman stands
    /// (`scripts::Effect::BringOutScrooge`).
    BringOutScrooge,
    /// The player threw the catch item in USE `slot` at the mob `object_id`:
    /// catch it if it is a `catch.mob` within `catch.area` of them, below
    /// `catch.mob_hp` percent of its HP. Answered with
    /// `ServerMessage::Caught`.
    Catch {
        object_id: i32,
        slot: i16,
        item_id: i32,
        catch: game_data::items::CatchInfo,
    },
    /// Send away the player's summon of this skill, or all of them.
    Unsummon {
        skill_id: Option<i32>,
        why: crate::packet::build::world::summon::Gone,
    },
    /// The player's pet at `index` in their pet list, standing at (x, y),
    /// would pick up the drop `object_id`. It may take mesos with `mesos`
    /// and items with `items`, but not the ids in `ignored` (mesos as
    /// `game::pet::IGNORED_MESOS`). Answered with
    /// `ServerMessage::PetPickedUp`.
    PetPickup {
        index: u8,
        object_id: i32,
        x: i16,
        y: i16,
        mesos: bool,
        items: bool,
        ignored: Vec<i32>,
        /// The quest drops the player may take.
        needs: crate::loot::QuestNeeds,
    },
    /// The player's client moved their pet at `index` in their pet list.
    PetMove {
        index: u8,
        start: (i16, i16),
        movement: Vec<u8>,
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
            (Op::AdminCommand, InGame(&world::AdminCommandHandler)),
            (Op::AdminLog, InGame(&world::AdminLogHandler)),
            (Op::AdminBroadcast, InGame(&world::AdminBroadcastHandler)),
            (Op::GmSlide, InGame(&world::GmSlideHandler)),
            (Op::PlayerMove, InGame(&world::PlayerMoveHandler)),
            (Op::ChangeChannel, InGame(&world::ChangeChannelHandler)),
            (Op::ChangeMap, InGame(&world::ChangeMapHandler)),
            (Op::EnterCashShop, InGame(&world::EnterCashShopHandler)),
            (Op::CheckCash, InGame(&world::CheckCashHandler)),
            (Op::CouponCode, InGame(&world::CouponCodeHandler)),
            (Op::RaiseRecord, InGame(&world::RaiseRecordHandler)),
            (Op::RaiseFeed, InGame(&world::RaiseFeedHandler)),
            (Op::NameChangeRules, InGame(&world::NameChangeRulesHandler)),
            (Op::CheckCharName, InGame(&world::CheckNewNameHandler)),
            (
                Op::CashShopOperation,
                InGame(&world::CashShopOperationHandler),
            ),
            (
                Op::PlayerMapTransfer,
                InGame(&world::PlayerMapTransferHandler),
            ),
            (
                Op::PartySearchStart,
                InGame(&world::PartySearchStartHandler),
            ),
            (Op::PartySearchStop, InGame(&world::PartySearchStopHandler)),
            (
                Op::ContiStateRequest,
                InGame(&world::ContiStateRequestHandler),
            ),
            (Op::ChangeKeybinds, InGame(&world::ChangeKeybindsHandler)),
            (Op::QuickSlot, InGame(&world::QuickSlotHandler)),
            (
                Op::MonsterBookCover,
                InGame(&world::MonsterBookCoverHandler),
            ),
            (Op::AllChat, InGame(&world::AllChatHandler)),
            (Op::FaceExpression, InGame(&world::FaceExpressionHandler)),
            (Op::UseItemEffect, InGame(&world::UseItemEffectHandler)),
            (Op::UseDeathItem, InGame(&world::UseDeathItemHandler)),
            (Op::UseChair, InGame(&world::UseChairHandler)),
            (Op::CancelChair, InGame(&world::CancelChairHandler)),
            (Op::Whisper, InGame(&world::WhisperHandler)),
            (Op::SpouseChat, InGame(&world::SpouseChatHandler)),
            (Op::PartyOperation, InGame(&world::PartyOperationHandler)),
            (
                Op::DenyPartyRequest,
                InGame(&world::DenyPartyRequestHandler),
            ),
            (Op::MultiChat, InGame(&world::MultiChatHandler)),
            (Op::BuddyListModify, InGame(&world::BuddyListModifyHandler)),
            (Op::NoteAction, InGame(&world::NoteActionHandler)),
            (Op::GuildOperation, InGame(&world::GuildOperationHandler)),
            (
                Op::DenyGuildRequest,
                InGame(&world::DenyGuildRequestHandler),
            ),
            (Op::GuildBbs, InGame(&world::GuildBbsHandler)),
            (
                Op::OpenFamilyPedigree,
                InGame(&world::OpenFamilyPedigreeHandler),
            ),
            (Op::OpenFamily, InGame(&world::OpenFamilyHandler)),
            (Op::AddFamily, InGame(&world::AddFamilyHandler)),
            (
                Op::SeparateFamilyBySenior,
                InGame(&world::SeparateFamilyBySeniorHandler),
            ),
            (
                Op::SeparateFamilyByJunior,
                InGame(&world::SeparateFamilyByJuniorHandler),
            ),
            (Op::AcceptFamily, InGame(&world::AcceptFamilyHandler)),
            (
                Op::ChangeFamilyMessage,
                InGame(&world::ChangeFamilyMessageHandler),
            ),
            (Op::UseFamily, InGame(&world::UseFamilyHandler)),
            (
                Op::FamilySummonResponse,
                InGame(&world::FamilySummonResponseHandler),
            ),
            (
                Op::AllianceOperation,
                InGame(&world::AllianceOperationHandler),
            ),
            (
                Op::DenyAllianceRequest,
                InGame(&world::DenyAllianceRequestHandler),
            ),
            (Op::Messenger, InGame(&world::MessengerHandler)),
            (
                Op::PlayerInteraction,
                InGame(&world::PlayerInteractionHandler),
            ),
            (Op::TalkToNpc, InGame(&world::NpcTalkHandler)),
            (Op::NpcTalkMore, InGame(&world::NpcTalkMoreHandler)),
            (Op::QuestAction, InGame(&world::QuestActionHandler)),
            (Op::NpcShopAction, InGame(&world::NpcShopActionHandler)),
            (Op::TrunkRequest, InGame(&world::TrunkRequestHandler)),
            (
                Op::EntrustedShopRequest,
                InGame(&world::EntrustedShopRequestHandler),
            ),
            (
                Op::StoreBankRequest,
                InGame(&world::StoreBankRequestHandler),
            ),
            (
                Op::RemoteShopRequest,
                InGame(&world::RemoteShopRequestHandler),
            ),
            (Op::DueyAction, InGame(&world::DueyActionHandler)),
            (Op::RpsAction, InGame(&world::RpsActionHandler)),
            (Op::MonsterCarnival, InGame(&world::MonsterCarnivalHandler)),
            (Op::Coconut, InGame(&world::CoconutHandler)),
            (Op::Snowball, InGame(&world::SnowballHandler)),
            (Op::LeftKnockback, InGame(&world::LeftKnockbackHandler)),
            (Op::RingAction, InGame(&world::RingActionHandler)),
            (Op::WeddingTalk, InGame(&world::WeddingTalkHandler)),
            (Op::WeddingBless, InGame(&world::WeddingBlessHandler)),
            (Op::WeddingAction, InGame(&world::WeddingActionHandler)),
            (
                Op::ChangeMapSpecial,
                InGame(&world::ChangeMapSpecialHandler),
            ),
            (Op::GatherItems, InGame(&world::GatherItemsHandler)),
            (Op::SortItems, InGame(&world::SortItemsHandler)),
            (Op::ItemMove, InGame(&world::ItemMoveHandler)),
            (Op::UseItem, InGame(&world::UseItemHandler)),
            (Op::UseItemReward, InGame(&world::UseItemRewardHandler)),
            (Op::UseSkillBook, InGame(&world::UseSkillBookHandler)),
            (Op::UseReturnScroll, InGame(&world::ReturnScrollHandler)),
            (Op::UseSummonBag, InGame(&world::UseSummonSackHandler)),
            (Op::UseCatchItem, InGame(&world::UseCatchItemHandler)),
            (Op::UseTeleportRock, InGame(&world::UseTeleportRockHandler)),
            (Op::UseOwl, InGame(&world::UseOwlHandler)),
            (Op::OwlOpen, InGame(&world::OwlOpenHandler)),
            (Op::OwlGo, InGame(&world::OwlGoHandler)),
            (Op::SkillMacro, InGame(&world::SkillMacroHandler)),
            (Op::UseSolomonItem, InGame(&world::UseSolomonItemHandler)),
            (Op::UseGachaExp, InGame(&world::UseGachaExpHandler)),
            (Op::UseTargetItem, InGame(&world::UseTargetItemHandler)),
            (
                Op::UseUpgradeScroll,
                InGame(&world::UseUpgradeScrollHandler),
            ),
            (Op::MoveLife, InGame(&world::MoveLifeHandler)),
            (Op::MobDistance, InGame(&world::MobDistanceHandler)),
            (Op::MobBomb, InGame(&world::MobBombHandler)),
            (Op::FieldDamageMob, InGame(&world::FieldDamageMobHandler)),
            (
                Op::MobDamageMobFriendly,
                InGame(&world::MobDamageMobFriendlyHandler),
            ),
            (Op::MobDamageMob, InGame(&world::MobDamageMobHandler)),
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
            (
                Op::TouchMonsterAttack,
                InGame(&world::AttackHandler(AttackKind::Body)),
            ),
            (Op::TakeDamage, InGame(&world::TakeDamageHandler)),
            (Op::ItemPickup, InGame(&world::ItemPickupHandler)),
            (Op::DamageReactor, InGame(&world::DamageReactorHandler)),
            (Op::TouchReactor, InGame(&world::TouchReactorHandler)),
            (Op::HealOverTime, InGame(&world::HealOverTimeHandler)),
            (Op::SpendAp, InGame(&world::SpendApHandler)),
            (Op::SpendSp, InGame(&world::SpendSpHandler)),
            (Op::UseSkill, InGame(&world::UseSkillHandler)),
            (Op::CancelBuff, InGame(&world::CancelBuffHandler)),
            (Op::CancelItemEffect, InGame(&world::CancelBuffHandler)),
            (Op::SkillEffect, InGame(&world::SkillEffectHandler)),
            (Op::ThrowGrenade, InGame(&world::ThrowGrenadeHandler)),
            (Op::MesoDrop, InGame(&world::MesoDropHandler)),
            (Op::CharInfoRequest, InGame(&world::CharInfoRequestHandler)),
            (Op::SpawnPet, InGame(&world::SpawnPetHandler)),
            (Op::DeadPet, InGame(&world::DeadPetHandler)),
            (Op::RemoteGachapon, InGame(&world::RemoteGachaponHandler)),
            (Op::WaterOfLife, InGame(&world::WaterOfLifeHandler)),
            (Op::MovePet, InGame(&world::MovePetHandler)),
            (Op::PetChat, InGame(&world::PetChatHandler)),
            (Op::PetCommand, InGame(&world::PetCommandHandler)),
            (Op::PetFood, InGame(&world::PetFoodHandler)),
            (Op::UseMountFood, InGame(&world::MountFoodHandler)),
            (Op::UseScriptedItem, InGame(&world::ScriptedItemHandler)),
            (Op::PetLoot, InGame(&world::PetLootHandler)),
            (Op::PetAutoPot, InGame(&world::PetAutoPotHandler)),
            (Op::PetExcludeItems, InGame(&world::PetExcludeItemsHandler)),
            (Op::UseCashItem, InGame(&world::UseCashItemHandler)),
            (Op::UseHammer, InGame(&world::UseHammerHandler)),
            (Op::ItemMake, InGame(&world::ItemMakeHandler)),
            (Op::CloseChalkboard, InGame(&world::CloseChalkboardHandler)),
            (Op::CalcDamageStat, InGame(&world::CalcDamageStatHandler)),
            (
                Op::TeleportRockList,
                InGame(&world::TeleportRockListHandler),
            ),
            (Op::GiveFame, InGame(&world::GiveFameHandler)),
            (Op::Claim, InGame(&world::ClaimHandler)),
            (Op::MoveSummon, InGame(&world::MoveSummonHandler)),
            (Op::SummonAttack, InGame(&world::SummonAttackHandler)),
            (Op::DamageSummon, InGame(&world::DamageSummonHandler)),
            (Op::Beholder, InGame(&world::BeholderHandler)),
            (Op::UseDoor, InGame(&world::UseDoorHandler)),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroadcastScope {
    /// All players in the world.
    World,
    /// All players in the world except the sender.
    WorldExcludeSelf,
    /// All players on this zero-based channel.
    Channel(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_opcode_is_unique_per_server() {
        // The maps would silently drop duplicates; check the table sizes match.
        assert_eq!(login_routes().len(), 13);
        assert_eq!(world_routes().len(), 140);
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
