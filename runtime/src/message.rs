//! Messages exchanged between actors, and the handles used to send them.
//!
//! Topology (arrows are channel sends):
//!
//! ```text
//!   ClientActor ──► FieldActor        (its current map: chat, movement, NPCs)
//!   ClientActor ──► WorldDirectory    (whispers, finds, broadcast, parties, messenger)
//!   FieldActor / WorldDirectory ──► ClientHandle (try_send, never awaited)
//! ```
//!
//! Clients await their sends into fields and the directory (bounded
//! backpressure), but fields and the directory never await a client: a client
//! whose queue is full is kicked instead. That makes wait cycles impossible.

use game::{AvatarLook, FieldAvatar, Guild, Player};
use net::packet::build::world::door::DoorView;
use net::packet::build::world::guild::InviteRefusal;
use net::packet::handle::world::{Rock, Whereabouts};
use net::{
    BroadcastScope, ClientId, FieldRequest, MessengerRequest, PartyRequest, PartySkill,
    WhisperDelivery,
};
use packet::Packet;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Capacity of each client's inbound queue. A client this far behind is
/// considered dead weight and disconnected.
pub const CLIENT_QUEUE_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FieldKey {
    pub channel_id: u8,
    pub map_id: i32,
    pub instance_id: u32,
}

impl FieldKey {
    pub fn new(channel_id: u8, map_id: i32) -> Self {
        Self {
            channel_id,
            map_id,
            instance_id: 0,
        }
    }
}

/// Messages delivered TO a client actor.
#[derive(Debug)]
pub enum ServerMessage {
    /// NPC identity validated by the player's field. Discard if stale.
    NpcConversation {
        npc_id: i32,
        field: FieldKey,
        request_id: u64,
    },
    /// A mob this player or their party killed; `exp` is their share.
    MobKilled {
        mob_id: i32,
        object_id: i32,
        exp: i32,
        party_bonus: i32,
        /// They hit it hardest of those still on the field, so they roll
        /// its loot.
        looter: bool,
        x: i16,
        y: i16,
        /// Taunt's Showdown: the percent more drops it gives.
        showdown: i32,
    },
    /// The player joined or left a party.
    PartyChanged {
        party_id: Option<i32>,
    },
    /// The player's invitation to `guild_id` stands; join it.
    JoinGuild {
        guild_id: i32,
    },
    /// Someone else changed the player's guild (expelled them); read it
    /// again.
    GuildChanged,
    /// A party member on `map_id` used a skill that reached this player,
    /// standing at `origin`.
    PartySkill {
        used: PartySkill,
        origin: (i16, i16),
        map_id: i32,
    },
    /// This player broke a reactor; its loot drops for them.
    ReactorBroken {
        reactor_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    },
    /// The field gave this player a drop to put in their inventory.
    PickedUp {
        drop: crate::actor::field_drops::Drop,
    },
    /// The field gave this player's pet at `pet` a drop it still shows;
    /// answer with `FieldMessage::PetLooted`.
    PetPickedUp {
        drop: crate::actor::field_drops::Drop,
        pet: u8,
    },
    /// Whether a quest NPC is on the player's map and within reach.
    QuestNpcChecked {
        request_id: u64,
        nearby: bool,
    },
    /// Someone on this channel looked for this player; answer with the map.
    Locate {
        reply_to: ClientHandle,
    },
    /// Someone on this channel used `rock` to reach this player; tell them
    /// where this player stands.
    RockLocate {
        reply_to: ClientHandle,
        rock: Rock,
    },
    /// Where the character this player used `rock` to reach stands, if they
    /// are on the channel.
    RockLocated {
        rock: Rock,
        found: Option<Whereabouts>,
    },
    /// `requester` (maybe this player) double-clicked this player; send them
    /// CHAR_INFO.
    CharInfoRequested {
        requester: ClientHandle,
    },
    /// `giver_name` raised (or dropped) this player's fame; tell `giver` how
    /// it went.
    Famed {
        giver: ClientHandle,
        giver_name: String,
        up: bool,
    },
    /// The fame this player gave `target` found nobody: they aren't on its
    /// map.
    FameMissed {
        target: i32,
    },
    /// A mob broke this player's Puppet: its buff ends.
    SummonBroken {
        skill_id: i32,
    },
    /// This player's Beholder heals or buffs them.
    Beholder(net::summons::Support),
    /// A mob's skill on `map_id` gave this player a disease.
    Diseased {
        disease: game::Disease,
        map_id: i32,
    },
    /// A mob's Dispel on `map_id` reached this player.
    Dispelled {
        map_id: i32,
    },
    /// A mob's Banish on `map_id` sends this player away.
    Banished {
        to: game_data::mobs::Banish,
        map_id: i32,
    },
    /// Mobs on `map_id` this player attacked countered it for `damage`.
    Countered {
        damage: i32,
        map_id: i32,
    },
    /// The Mystic Door this player entered on `from_map` leads to `map_id`.
    DoorWarp {
        from_map: i32,
        map_id: i32,
        exit: net::packet::handle::world::DoorExit,
    },
    /// A ship's trip moves everyone on `from_map` to `map_id`, at `portal`.
    ShipWarp {
        from_map: i32,
        map_id: i32,
        portal: &'static str,
    },
    /// News from trading room `room` on this player's field.
    Trade {
        room: i32,
        event: crate::actor::field_trades::TradeEvent,
    },
    /// News from personal shop `room` on this player's field.
    PersonalShop {
        room: i32,
        event: crate::actor::field_shops::ShopEvent,
    },
    SendPacket(Packet),
}

/// A cheap, clonable way to reach one client connection.
#[derive(Clone, Debug)]
pub struct ClientHandle {
    pub client_id: ClientId,
    /// Distinguishes reconnects of the same character.
    pub connection_id: u64,
    tx: mpsc::Sender<ServerMessage>,
    kill: CancellationToken,
}

impl ClientHandle {
    pub fn new(
        client_id: ClientId,
        connection_id: u64,
        tx: mpsc::Sender<ServerMessage>,
        kill: CancellationToken,
    ) -> Self {
        Self {
            client_id,
            connection_id,
            tx,
            kill,
        }
    }

    /// Queue a message without waiting. A full queue means the client is not
    /// keeping up, so it is disconnected rather than allowed to stall the
    /// sender. Returns whether the message was queued.
    pub fn send(&self, message: ServerMessage) -> bool {
        match self.tx.try_send(message) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                warn!(
                    client_id = self.client_id,
                    "Client queue full; disconnecting slow client"
                );
                self.kill.cancel();
                false
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }

    pub fn send_packet(&self, packet: Packet) -> bool {
        self.send(ServerMessage::SendPacket(packet))
    }

    /// Ask the client actor to disconnect.
    pub fn kick(&self) {
        self.kill.cancel();
    }
}

/// Messages to a field actor.
#[derive(Debug)]
pub enum FieldMessage {
    Join {
        handle: ClientHandle,
        avatar: FieldAvatar,
        party: Option<i32>,
    },
    /// The occupant joined or left a party, which decides who shares EXP.
    SetParty {
        client_id: ClientId,
        party: Option<i32>,
    },
    Leave {
        client_id: ClientId,
    },
    /// The occupant's appearance or level changed.
    UpdateAvatar {
        client_id: ClientId,
        avatar: FieldAvatar,
        /// Sent to everyone else on the map (e.g. UPDATE_CHAR_LOOK).
        announce: Vec<Packet>,
    },
    Chat {
        from: ClientId,
        packet: Packet,
    },
    Move {
        from: ClientId,
        packet: Packet,
        start: (i16, i16),
        movement_bytes: Vec<u8>,
    },
    NpcInteraction {
        from: ClientId,
        object_id: i32,
        request_id: u64,
    },
    /// A player's request of the field (`net::handler::FieldRequest`).
    FromClient {
        from: ClientId,
        request: FieldRequest,
    },
    /// Put a player's loot from a kill on the ground.
    SpawnLoot {
        owner: ClientId,
        ownership: crate::actor::field_drops::Ownership,
        loot: Vec<net::loot::Loot>,
        x: i16,
        y: i16,
        dropper: i32,
    },
    /// The client couldn't take the drop (full inventory); put it back.
    ReturnDrop {
        drop: crate::actor::field_drops::Drop,
    },
    /// The pet at `pet` of `looter` took the drop it was given, or with
    /// `taken` false left it where it lies.
    PetLooted {
        looter: ClientId,
        pet: u8,
        drop: crate::actor::field_drops::Drop,
        taken: bool,
    },
    /// Check that an NPC with this template id is near the player before a
    /// quest action; answered with `ServerMessage::QuestNpcChecked`.
    QuestNpc {
        from: ClientId,
        npc_id: i32,
        request_id: u64,
    },
    /// `owner` cast Mystic Door here, at `at`.
    OpenDoor {
        owner: ClientId,
        at: (i16, i16),
    },
    /// `owner`'s Mystic Door here closed.
    CloseDoor {
        owner: ClientId,
    },
    /// A player's request of their trading room. Not a `FieldRequest`: the
    /// final answer carries a oneshot.
    Trade {
        from: ClientId,
        request: crate::actor::field_trades::RoomRequest,
    },
    /// A player's request of a personal shop; purchases carry oneshots.
    PersonalShop {
        from: ClientId,
        request: crate::actor::field_shops::ShopRoomRequest,
    },
}

/// A way to reach one field actor.
#[derive(Clone, Debug)]
pub struct FieldHandle {
    pub key: FieldKey,
    pub tx: mpsc::Sender<FieldMessage>,
}

/// What the world directory knows of an online character: enough for party
/// windows, HP bars, and guild invitations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presence {
    pub name: String,
    pub job: i16,
    pub level: u8,
    pub map_id: i32,
    pub hp: i16,
    pub max_hp: i16,
    pub guild_id: Option<i32>,
}

impl Presence {
    pub fn of(player: &Player) -> Self {
        let stats = &player.stats;
        Self {
            name: stats.name.clone(),
            job: stats.job,
            level: stats.level,
            map_id: stats.map_id,
            hp: stats.hp,
            max_hp: stats.max_hp,
            guild_id: player.guild.as_ref().map(|guild| guild.id),
        }
    }
}

/// A buddy on someone's list, for the directory to fill in their channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedBuddy {
    pub id: ClientId,
    pub name: String,
    pub group: String,
    /// They list the owner too, so the owner may see their channel.
    pub mutual: bool,
    /// They asked to be added and the owner hasn't answered.
    pub pending: bool,
}

/// Messages to the world directory.
#[derive(Debug)]
pub enum DirectoryMessage {
    Register {
        handle: ClientHandle,
        presence: Presence,
        channel_id: u8,
    },
    /// The character's map, level, job, or HP changed.
    Presence {
        client_id: ClientId,
        presence: Presence,
    },
    Party {
        from: ClientId,
        request: PartyRequest,
    },
    /// `from` used a skill at `origin` on the party members in its slots.
    PartySkill {
        from: ClientId,
        used: PartySkill,
        origin: (i16, i16),
    },
    Unregister {
        client_id: ClientId,
        connection_id: u64,
        /// The client is moving to another channel and will register again.
        changing_channel: bool,
    },
    Whisper {
        from: ClientId,
        delivery: WhisperDelivery,
    },
    Find {
        from: ClientId,
        target_name: String,
    },
    /// Find the character named `name` on the sender's channel, to take the
    /// sender to them with `rock`.
    RockLocate {
        from: ClientId,
        name: String,
        rock: Rock,
    },
    Broadcast {
        from: ClientId,
        scope: BroadcastScope,
        packet: Packet,
    },
    /// Send `owner` their buddy list with the channels of those online.
    BuddyList {
        owner: ClientId,
        buddies: Vec<ListedBuddy>,
    },
    /// Send `owner` their guild with who is online.
    GuildInfo {
        owner: ClientId,
        guild: Guild,
    },
    /// Send a packet to whichever of these characters are online.
    Deliver {
        to: Vec<ClientId>,
        packet: Packet,
    },
    /// `from`, a master or jr. master of `guild_id`, invites the character
    /// called `name`.
    GuildInvite {
        from: ClientId,
        guild_id: i32,
        name: String,
    },
    /// `from` accepts the invitation to `guild_id`; if it stands, they are
    /// told to join (`ServerMessage::JoinGuild`).
    GuildAccept {
        from: ClientId,
        guild_id: i32,
    },
    /// `from` turns down `inviter`'s invitation.
    GuildDeny {
        from: ClientId,
        inviter: String,
        refusal: InviteRefusal,
    },
    /// `client_id` was put in or out of a guild by someone else.
    GuildChanged {
        client_id: ClientId,
    },
    Messenger {
        from: ClientId,
        request: MessengerRequest,
    },
    /// The client's worn equipment changed.
    LookChanged {
        client_id: ClientId,
        look: AvatarLook,
    },
    /// `owner` opened a Mystic Door, replacing any they had.
    DoorOpened {
        owner: ClientId,
        door: DoorView,
    },
    DoorClosed {
        owner: ClientId,
    },
    /// `from` entered `owner`'s Mystic Door, in its town or its field;
    /// answered with `ServerMessage::DoorWarp` if they may.
    UseDoor {
        from: ClientId,
        owner: ClientId,
        from_town: bool,
    },
}
