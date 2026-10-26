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

use game::alliance::Alliance;
use game::family::{FamilyBuff, Relatives};
use game::{AvatarLook, FieldAvatar, Guild, Player};
use net::events::{Entry, EventKind};
use net::maple_tv::MapleTv;
use net::packet::build::world::door::DoorView;
use net::packet::build::world::guild::InviteRefusal;
use net::packet::build::world::maple_tv::TvRefusal;
use net::packet::handle::world::{PartyPotion, Rock, Whereabouts};
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
/// considered dead weight and disconnected. Entering a map queues a packet
/// for each of its NPCs, players, mobs (and control of them), reactors and
/// drops while the client is still busy with the map change, close to 200
/// in Kerning City's subway, so this leaves room for the busiest maps.
pub const CLIENT_QUEUE_CAPACITY: usize = 1024;

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
    /// This player's Steal took an item from a mob, which drops one of its
    /// items for them where it stands.
    Stole {
        mob_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
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
    /// A GM jailed the player or let them out; read the time again.
    JailChanged,
    /// A GM blocked the player's account from chatting or lifted it; read
    /// it again.
    ChatBlockChanged,
    /// `from` (named so) proposes; ask the player unless they can't be
    /// asked.
    Proposed {
        from: ClientId,
        name: String,
    },
    /// The player said yes to `proposer`'s proposal with `item_id`: record
    /// the engagement and take the ring.
    ProposalAccepted {
        proposer: ClientId,
        item_id: i32,
    },
    /// The player's partner recorded the engagement; trade the ring box for
    /// its empty box.
    Engaged(game::marriage::MarriageRecord),
    /// The player's partner broke off the couple `record`; forget it and
    /// take back the token.
    CoupleEnded(game::marriage::MarriageRecord),
    /// Word between the partners of a wedding being booked (report b066).
    Wedding(WeddingNews),
    /// The player's partner is on `map_id`, or 0 with `partner` 0 for no
    /// partner online (report b057).
    PartnerMap {
        map_id: i32,
        partner: ClientId,
    },
    /// `junior` (named so) said yes to the player's request; add them.
    AddJunior {
        junior: ClientId,
        name: String,
    },
    /// The player's family changed; read it again.
    FamilyChanged,
    /// Part `junior` from `senior` for the player, at these levels where
    /// they are online.
    Separate {
        senior: ClientId,
        junior: ClientId,
        levels: (Option<u8>, Option<u8>),
    },
    /// The player earned their seniors `gain` Rep; the senior is of
    /// `senior_level` if online.
    EarnRep {
        gain: i32,
        senior_level: Option<u8>,
    },
    /// A family member is coming to this player (`trip`); tell them where
    /// this player stands.
    FamilyLocate {
        reply_to: ClientHandle,
        trip: FamilyTrip,
    },
    /// Where the family member this player is going to stands, if they are
    /// on the channel.
    FamilyLocated {
        trip: FamilyTrip,
        found: Option<Whereabouts>,
    },
    /// The summons this player paid for to bring `name` didn't happen;
    /// return the Rep and say why.
    SummonRefund {
        name: String,
        why: SummonFailure,
    },
    /// A family rate entitlement now works for the player: their own, or
    /// their party's.
    FamilyBuff(FamilyBuff),
    /// How many of the juniors the player's bonding entitlement needs are
    /// online.
    FamilyBondingCount(usize),
    /// The invitation the player's guild had to `alliance_id` stands; join
    /// it.
    JoinAlliance {
        alliance_id: i32,
    },
    /// A party member on `map_id` used a skill that reached this player,
    /// standing at `origin`.
    PartySkill {
        used: PartySkill,
        origin: (i16, i16),
        map_id: i32,
    },
    /// A party member on `map_id` picked up the party potion `item_id`.
    PartyItem {
        item_id: i32,
        map_id: i32,
    },
    /// This player broke a reactor on `field`; its loot drops for them, or
    /// it sends them somewhere.
    ReactorBroken {
        field: FieldKey,
        reactor_id: i32,
        object_id: i32,
        x: i16,
        y: i16,
    },
    /// The field gave this player a drop to put in their inventory.
    PickedUp {
        drop: crate::actor::field_drops::Drop,
    },
    /// This player caught a mob with the catch item `item_id` in USE
    /// `slot`, which makes `create`.
    Caught {
        slot: i16,
        item_id: i32,
        create: i32,
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
    /// Someone on this channel used a Party Potion on this player; morph
    /// them and say whether it took.
    MorphAnother {
        reply_to: ClientHandle,
        potion: PartyPotion,
    },
    /// Whether the character this player's `potion` named was morphed.
    MorphedAnother {
        potion: PartyPotion,
        morphed: bool,
    },
    /// Someone's Maple TV message names this player: hand it back to the
    /// world's line with this player's look.
    MapleTvPartner {
        tv: MapleTv,
        reply: tokio::sync::oneshot::Sender<Result<(), TvRefusal>>,
    },
    /// The world put this player's Maple TV message in line, or didn't: use
    /// up the item in Cash `slot`, or say why not.
    MapleTvQueued {
        slot: i16,
        item_id: i32,
        queued: Result<(), TvRefusal>,
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
    /// This player's attack on `map_id` took `hp` and `mp` from the mobs
    /// it hit (Drain, MP Eater).
    Absorbed {
        hp: i32,
        mp: i32,
        map_id: i32,
    },
    /// The Mystic Door this player entered on `from_map` leads to `map_id`.
    DoorWarp {
        from_map: i32,
        map_id: i32,
        exit: net::packet::handle::world::DoorExit,
    },
    /// A GM's `/start` in the lobby `from_map` sends the player to their
    /// team's spot on the game field `map_id`.
    GoToGmEvent {
        from_map: i32,
        map_id: i32,
    },
    /// A ship's trip moves everyone on `from_map` to `map_id`, at `portal`.
    ShipWarp {
        from_map: i32,
        map_id: i32,
        portal: net::conti::Portal,
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
    /// News from Omok or match cards game `room` on this player's field.
    MiniGame {
        room: i32,
        event: crate::actor::field_games::GameEvent,
    },
    /// News from `owner`'s hired merchant on this player's field.
    HiredMerchant {
        owner: i32,
        event: crate::actor::field_merchants::MerchantEvent,
    },
    /// The player's party went into an event; this is their ticket.
    EventStarted(crate::actor::event::EventTicket),
    /// Event `id` ended; out of its maps.
    EventEnded {
        id: u32,
    },
    /// The player's party passed to `to`, from them or to them; an event
    /// it is in goes with it.
    LeaderHandedOver {
        to: ClientId,
    },
    /// The party in event `id` lost what it protected there.
    EventFailed {
        id: u32,
    },
    /// The player's field saw what clears `stage` of the event they are
    /// in, as Zakum's last body dying on its altar: tell the event.
    ClearEventStage(u8),
    /// This player's attack on a copy of a Nett's Pyramid stage did this to
    /// its mobs, for their run's tally.
    Massacre(net::pyramid::Hits),
    /// This player's attack on a copy of a Monster Carnival battle field
    /// killed mobs worth this many CP.
    CarnivalCp(i32),
    /// Packets from event `id` for its members on its maps: a member not
    /// let in, or since gone, doesn't get them.
    EventPackets {
        id: u32,
        packets: Vec<Packet>,
    },
    /// The Nett's Pyramid run `id` reached a Pharaoh's Blessing: the
    /// player takes item `item_id`'s effect, if on its maps.
    PyramidBlessing {
        id: u32,
        item_id: i32,
    },
    /// The Nett's Pyramid run `id` has `left` uses of Rage of Pharaoh.
    PyramidRage {
        id: u32,
        left: i32,
    },
    /// A Nett's Pyramid run ended with `rank` and `exp`: the player gets
    /// them if they reached its result map, `exit`, and if it went through
    /// all five stages (`cleared`), the offer of its gem in the Shades.
    PyramidScore {
        exit: i32,
        rank: u8,
        exp: i32,
        cleared: bool,
    },
    /// The Monster Carnival battle `id` is over: the player's side `won` or
    /// not, with `points` CP in all, and their diseases end. 2042002
    /// grades them by it in the result room they go to next.
    CarnivalResult {
        id: u32,
        won: bool,
        points: i32,
    },
    /// The player's field summoned its boss, as Zakum on its altar: an
    /// event held there tells when it closes.
    BossSummoned,
    /// The player's field brought out a boss whose event's clock starts
    /// over, as Horntail on its cave: tell the event.
    RestartEventClock,
    /// A mob that leaves a buff (Mob.nx `buff`) was killed on the player's
    /// field: if alive, the player takes this item's effect.
    MobBuff(i32),
    /// A party challenges the one the player leads in a Monster Carnival
    /// waiting room: ask them.
    CarnivalChallenge {
        challenger: ClientId,
        challengers: Vec<net::carnival::Challenger>,
    },
    /// Event `id`'s time left is now `clock`, or it has no time limit.
    EventClock {
        id: u32,
        clock: Option<tokio::time::Instant>,
    },
    /// A reactor on the player's map gave everyone there `0` EXP.
    GainExp(i32),
    /// The party in event `id` cleared `stage`, for `exp` rather than the
    /// stage's own EXP if given.
    StageCleared {
        id: u32,
        stage: u8,
        exp: Option<i32>,
    },
    /// Event `id` takes its members on `from`, or all of them, to `to`,
    /// where the time left is `clock`.
    EventMoved {
        id: u32,
        from: Option<i32>,
        to: i32,
        clock: Option<tokio::time::Instant>,
    },
    /// The couple started their wedding; this is the player's ticket.
    WeddingStarted(crate::actor::wedding::WeddingTicket),
    /// Wedding `id` takes the couple to `to`, where the time left is
    /// `clock`.
    WeddingMoved {
        id: u32,
        to: i32,
        clock: Option<tokio::time::Instant>,
    },
    /// Wedding `id` ended, before the couple married if `failed`; out of
    /// its maps.
    WeddingEnded {
        id: u32,
        failed: bool,
    },
    /// The priest of wedding `id` shows `line` at its altar.
    WeddingPriest {
        id: u32,
        line: crate::actor::wedding::PriestLine,
    },
    /// Wedding `id` married the couple `record`, whom `blessings` guests
    /// there blessed; their time there now runs out at `clock`.
    Married {
        id: u32,
        record: game::marriage::MarriageRecord,
        blessings: u32,
        clock: tokio::time::Instant,
    },
    SendPacket(Packet),
    /// A packet for everyone in the world, sent on by this player's client
    /// (a field has no way to the directory).
    Announce(Packet),
    /// A quest drop's packet, which the player sees only if they still need
    /// `item_id` for `quest` (`net::loot::wants`).
    QuestDrop {
        quest: i16,
        item_id: i32,
        packet: Packet,
    },
}

/// Why a family member goes to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyTrip {
    /// The player spent the teleport entitlement.
    Teleport,
    /// `summoner` spent the summon entitlement and the player said yes.
    Summoned { summoner: ClientId },
}

/// Why a summons failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummonFailure {
    /// The member isn't on the summoner's channel, or left.
    NotFound,
    /// Someone else is summoning them.
    Busy,
    /// They said no, or didn't answer in time.
    Declined,
    /// The summoner moved, or the member may not leave where they are.
    Location,
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
        /// The player is warping within this field: it isn't left empty,
        /// so a solo room doesn't start afresh.
        staying: bool,
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
    /// A player's request of an Omok or match cards game.
    MiniGame {
        from: ClientId,
        request: crate::actor::field_games::GameRoomRequest,
    },
    /// A player's request of a hired merchant; purchases and closing carry
    /// oneshots.
    Merchant {
        from: ClientId,
        request: crate::actor::field_merchants::MerchantRequest,
    },
    /// Where everyone on the map stands, as of their last move.
    Positions {
        reply: tokio::sync::oneshot::Sender<Vec<(i16, i16)>>,
    },
    /// The open shops here selling `item_id`, for the Owl of Minerva.
    ShopSearch {
        item_id: i32,
        reply: tokio::sync::oneshot::Sender<Vec<net::owl::ShopHit>>,
    },
    /// Whether `name` would get into shop `room` here (`owl::go_result`).
    ShopGo {
        room: i32,
        name: String,
        reply: tokio::sync::oneshot::Sender<u8>,
    },
    /// Keep a team on the field's Coconut or Snowball game for a player an
    /// event NPC is sending there; None if it has no game.
    ReserveTeam {
        client_id: ClientId,
        reply: tokio::sync::oneshot::Sender<Option<u8>>,
    },
    /// A GM's `/start` (`started` then) or `/reset` (None) of the timed
    /// event this field is one of the maps of.
    RunTimedEvent {
        started: Option<tokio::time::Instant>,
    },
    /// Whether the timed event this field is one of the maps of is on, so
    /// its lobby's gate is open.
    TimedEventOn {
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    /// A GM's `/start` in this lobby: everyone but the GM `from` goes on
    /// to the game field `map_id`.
    SendToGmEvent {
        from: ClientId,
        map_id: i32,
    },
    /// How many monsters are alive on the map.
    MobCount {
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// How many drops, items or mesos, lie on the map.
    DropCount {
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// The states of the live reactors named `names` here, in turn, with
    /// `None` for a name none has.
    ReactorStates {
        names: Vec<&'static str>,
        reply: tokio::sync::oneshot::Sender<Vec<Option<u8>>>,
    },
    /// Set the live reactors named `name` here to `state`, as an event's
    /// gate shut while it runs (`net::events::Event::gate`).
    SetReactor {
        name: &'static str,
        state: u8,
    },
    /// How many Eak wants on each of the sealed room's platforms
    /// (`net::events::draw_split`), drawn when first asked.
    Split {
        reply: tokio::sync::oneshot::Sender<[u8; 3]>,
    },
    /// The right pad of each row of the map's climb
    /// (`net::events::draw_climb`), drawn when first asked.
    Climb {
        reply: tokio::sync::oneshot::Sender<Vec<u8>>,
    },
    /// The two of the tall room's levers Eak wants up
    /// (`net::events::draw_levers`), drawn when first asked.
    Levers {
        reply: tokio::sync::oneshot::Sender<[bool; 5]>,
    },
    /// In a jail under the Tower of Goddess's sealed room, what its way on
    /// makes of its levers against its draw (`net::events::draw_jail`, drawn
    /// when first asked); `None` on any other map.
    Jail {
        reply: tokio::sync::oneshot::Sender<Option<net::events::JailCheck>>,
    },
    /// Show everyone on the field a packet, as an event's line to a map
    /// of its copy.
    Show(Packet),
    /// Every mob here that isn't friendly goes, giving nothing, and no more
    /// come: an event won on this copy of the map (Cosmic
    /// `killAllMonstersNotFriendly`).
    ClearMobs,
    /// Every mob here that isn't friendly goes, giving nothing, and from
    /// now on `only` this mob comes, at every point of its own, or none:
    /// Guon took a Mark on the Pirate Ship's second stage
    /// (`EventCommand::HandInMarks`).
    SwapMobs {
        only: Option<i32>,
    },
    /// Bring out one `mob_id` on the ground below `at`: the Lord Pirate,
    /// as the first member goes past the Pirate Ship's doors
    /// (`EventCommand::PastTheDoors`).
    BringOut {
        mob_id: i32,
        at: (i16, i16),
    },
    /// `summoner` spent CP on `index` of the Monster Carnival window's tab
    /// `tab`, which everyone on the field hears of.
    CarnivalAnnounce {
        summoner: ClientId,
        tab: u8,
        index: u8,
    },
    /// `died` of side `side` died, which cost their side `lost` CP: everyone
    /// on the Monster Carnival field hears of it.
    CarnivalDied {
        died: ClientId,
        side: u8,
        lost: u8,
    },
    /// `summoner` spent CP on a `mob_id`, `index` of the Monster Carnival
    /// field's list, for their side `side`: whether it came out, or why
    /// not, such as `cap` of the side's summons already out.
    CarnivalSummon {
        summoner: ClientId,
        index: u8,
        mob_id: i32,
        side: u8,
        cap: usize,
        reply: tokio::sync::oneshot::Sender<Result<(), net::carnival::SpendRefusal>>,
    },
    /// `summoner` spent CP on `guardian`, `index` of the Monster Carnival
    /// field's guardian list, for their side `side`: whether it stands, or
    /// why not, such as `cap` of the side's already standing.
    CarnivalGuard {
        summoner: ClientId,
        index: u8,
        guardian: game_data::skills::CarnivalSkill,
        side: u8,
        cap: usize,
        reply: tokio::sync::oneshot::Sender<Result<(), net::carnival::SpendRefusal>>,
    },
    /// The event whose copy of the map this is ended: stop.
    Close,
}

/// A way to reach one field actor.
#[derive(Clone, Debug)]
pub struct FieldHandle {
    pub key: FieldKey,
    pub tx: mpsc::Sender<FieldMessage>,
}

/// What the world directory knows of an online character: enough for party
/// windows, HP bars, and guilds (invitations, and members' levels and jobs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presence {
    pub name: String,
    pub job: i16,
    pub level: u8,
    pub map_id: i32,
    pub hp: i16,
    pub max_hp: i16,
    pub guild_id: Option<i32>,
    /// The guild's alliance, for its members' logins, levels and jobs.
    pub alliance_id: Option<i32>,
    /// The senior and juniors, who hear of logins.
    pub family: Relatives,
    /// The other one of the couple, who is told the player's map.
    pub partner: Option<ClientId>,
    /// Whether the account is a GM's.
    pub gm: bool,
    /// A GM hidden by Hide, whom only GMs can find or whisper.
    pub hidden: bool,
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
            max_hp: player.effective_max_hp() as i16,
            guild_id: player.guild.as_ref().map(|guild| guild.id),
            alliance_id: player
                .guild
                .as_ref()
                .map(|guild| guild.alliance_id)
                .filter(|&id| id != 0),
            family: player.family.clone(),
            partner: player
                .marriage
                .as_ref()
                .map(|record| record.partner_of(player.id())),
            gm: player.gm_level > 0,
            hidden: player.hidden,
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

/// What a squad made of a player's command (`DirectoryMessage::Squad`).
#[derive(Debug)]
pub enum SquadAnswer {
    /// It no longer fits: the squad changed since its NPC looked.
    Refused,
    Done,
    /// The squad set out: start its event.
    Start(crate::actor::event::EventStart),
    /// Back into the squad's event, with this ticket.
    Rejoin(crate::actor::event::EventTicket),
}

/// What came of a leader's pick of a Monster Carnival room
/// (`DirectoryMessage::CarnivalPick`).
#[derive(Debug)]
pub enum CarnivalPick {
    /// The room was free: the party takes it, waiting for a challenge.
    Start(crate::actor::event::EventStart),
    /// The challenge was put to the party waiting there.
    Sent,
    Refused(net::carnival::Refusal),
}

/// Messages to the world directory.
#[derive(Debug)]
pub enum DirectoryMessage {
    Register {
        handle: ClientHandle,
        presence: Presence,
        channel_id: u8,
    },
    /// The character's map, level, job, HP, guild, family or partner
    /// changed.
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
    /// `from` picked up the party potion `item_id`: the other party
    /// members on their map get it too.
    PartyItem {
        from: ClientId,
        item_id: i32,
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
    /// Find the character a Party Potion names on the sender's channel, to
    /// morph them.
    MorphAnother {
        from: ClientId,
        potion: PartyPotion,
    },
    Broadcast {
        from: ClientId,
        scope: BroadcastScope,
        packet: Packet,
    },
    /// Show the world an avatar megaphone's banner in its turn; `reply`
    /// says whether it got in line.
    AvatarBanner {
        banner: Packet,
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    /// Put a Maple TV message in the TV's line. One naming a receiver goes
    /// to them for their look (`partner`) first. `reply` says whether it
    /// got in; dropped, the receiver wasn't found.
    MapleTv {
        tv: MapleTv,
        partner: Option<AvatarLook>,
        reply: tokio::sync::oneshot::Sender<Result<(), TvRefusal>>,
    },
    /// Disconnect whichever of `client_ids` are online; `reply` gets how
    /// many were.
    Kick {
        client_ids: Vec<ClientId>,
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// Someone searched for `item_id` with the Owl of Minerva.
    OwlSearched {
        item_id: i32,
    },
    /// The items searched for most with the Owl of Minerva.
    OwlPopular {
        reply: tokio::sync::oneshot::Sender<Vec<i32>>,
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
    /// Whether `client_id` is online.
    Online {
        client_id: ClientId,
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    /// Whether `from`'s party may go into an event of `kind` now.
    EventEntry {
        from: ClientId,
        kind: EventKind,
        reply: tokio::sync::oneshot::Sender<Entry>,
    },
    /// `from`, a party leader, takes the party into an event of `kind`:
    /// reserve it on their channel for the members who go, if they still
    /// may.
    StartEvent {
        from: ClientId,
        kind: EventKind,
        reply: tokio::sync::oneshot::Sender<Option<crate::actor::event::EventStart>>,
    },
    /// A GM opened the event on `map_id` to `limit` players on their
    /// channel, in place of any open there.
    OpenGmEvent {
        channel_id: u8,
        map_id: i32,
        limit: u16,
    },
    /// A GM closed entry to their channel's event.
    CloseGmEvent {
        channel_id: u8,
    },
    /// The map of the event open on the channel with places left, if any.
    GmEventOpen {
        channel_id: u8,
        reply: tokio::sync::oneshot::Sender<Option<i32>>,
    },
    /// Take a place at the event open on the channel: its map, or None if
    /// it closed or filled up.
    JoinGmEvent {
        channel_id: u8,
        reply: tokio::sync::oneshot::Sender<Option<i32>>,
    },
    /// A player gives back their place at the event on `map_id`, if it is
    /// still the one open on the channel.
    LeaveGmEvent {
        channel_id: u8,
        map_id: i32,
    },
    /// What `from` finds of the Monster Carnival's rooms on their channel.
    CarnivalRooms {
        from: ClientId,
        reply: tokio::sync::oneshot::Sender<Vec<net::carnival::Room>>,
    },
    /// `from`, a party leader, picks Monster Carnival room `room`: their
    /// party takes it if free, or challenges the party waiting there.
    CarnivalPick {
        from: ClientId,
        room: u8,
        reply: tokio::sync::oneshot::Sender<CarnivalPick>,
    },
    /// `from`, leading the party waiting in a Monster Carnival room,
    /// accepts or declines the challenge `challenger` put to them.
    CarnivalAnswer {
        from: ClientId,
        challenger: ClientId,
        accept: bool,
    },
    /// What `from` finds of the squad of `kind` on their channel, at its
    /// NPC (`net::squads`).
    SquadTalk {
        from: ClientId,
        kind: EventKind,
        reply: tokio::sync::oneshot::Sender<net::squads::SquadView>,
    },
    /// `from` tells the squad of `kind` on their channel `command`.
    Squad {
        from: ClientId,
        kind: EventKind,
        command: net::squads::SquadCommand,
        reply: tokio::sync::oneshot::Sender<SquadAnswer>,
    },
    /// Event `id` of `kind` on `channel_id` is over or won; the next party
    /// may go.
    EventClosed {
        channel_id: u8,
        kind: EventKind,
        id: u32,
    },
    /// `from` starts their booked wedding with `partner` at `venue`: if the
    /// partner stands beside them and the venue is free on their channel,
    /// reserve it for the couple.
    StartWedding {
        from: ClientId,
        partner: ClientId,
        venue: game::wedding::Venue,
        marriage_id: i32,
        reply: tokio::sync::oneshot::Sender<
            Result<crate::actor::wedding::WeddingStart, game::wedding::StartRefusal>,
        >,
    },
    /// `from`, invited to the wedding of `marriage_id` at `venue`, asks to
    /// go in. Answers their ticket if the wedding is under way on their
    /// channel and lets them in.
    JoinWedding {
        from: ClientId,
        venue: game::wedding::Venue,
        marriage_id: i32,
        reply: tokio::sync::oneshot::Sender<
            Result<crate::actor::wedding::WeddingTicket, game::wedding::GuestRefusal>,
        >,
    },
    /// `from` married: their guild and family read so.
    Married {
        from: ClientId,
    },
    /// Wedding `id` at `venue` on `channel_id` is over; the venue is free.
    WeddingClosed {
        channel_id: u8,
        venue: game::wedding::Venue,
        id: u32,
    },
    /// `from` takes a mini-dungeon's way in: the first free of the `count`
    /// copies from `first` on their channel is theirs, and the members of
    /// the party they lead who stand beside them go to its `portal`.
    /// Answers the copy, which `from` still has to go to.
    EnterDungeon {
        from: ClientId,
        first: i32,
        count: i32,
        portal: u8,
        reply: tokio::sync::oneshot::Sender<Result<i32, net::dungeons::Refusal>>,
    },
    /// Whether anyone is on `map_id` in `channel_id`.
    MapOccupied {
        channel_id: u8,
        map_id: i32,
        reply: tokio::sync::oneshot::Sender<bool>,
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
    /// A GM jailed `client_id` or let them out.
    JailChanged {
        client_id: ClientId,
    },
    /// A GM blocked the account of `client_ids` from chatting, or lifted
    /// it.
    ChatBlockChanged {
        client_ids: Vec<ClientId>,
    },
    /// `from` proposes to `partner` with the ring box `item_id`: ask them
    /// if they are on `from`'s map and of the level.
    Propose {
        from: ClientId,
        partner: ClientId,
        item_id: i32,
    },
    /// `from` withdrew their proposal.
    ProposalCancel {
        from: ClientId,
    },
    /// `from` answered `proposer`'s proposal.
    ProposalAnswer {
        from: ClientId,
        proposer: ClientId,
        accept: bool,
    },
    /// `from` can't be asked or can't take the ring after all; tell their
    /// proposer why.
    ProposalRefused {
        from: ClientId,
        refusal: game::marriage::Refusal,
    },
    /// `from` is engaged to `proposer` now; tell `proposer`.
    Engaged {
        proposer: ClientId,
        record: game::marriage::MarriageRecord,
    },
    /// The couple `record` is over; tell `partner`, who didn't break it off.
    CoupleEnded {
        partner: ClientId,
        record: game::marriage::MarriageRecord,
    },
    /// `from` books their wedding with `partner` at `venue`: ask the partner
    /// for their side if they are on `from`'s map, else tell `from` why not.
    ReserveWedding {
        from: ClientId,
        partner: ClientId,
        venue: game::wedding::Venue,
        premium: bool,
    },
    /// Pass word of a wedding being booked to the partner `to`.
    Wedding {
        to: ClientId,
        news: WeddingNews,
    },
    /// Hand `invitation` to its guest if they are online, else leave it at
    /// Duey.
    DeliverInvitation {
        invitation: game::wedding::Invitation,
    },
    /// `invitation` could be neither given nor left at Duey: its guest is
    /// off the list again; hand it back to its sender.
    ReturnInvitation {
        invitation: game::wedding::Invitation,
    },
    /// `from` went where they can't answer or be answered (the Cash Shop):
    /// their proposals end, and their party can't take them into an event.
    EnteredCashShop {
        from: ClientId,
    },
    /// Ask `junior`, if they are on `from`'s map and of the levels, to be
    /// `from`'s junior.
    FamilyInvite {
        from: ClientId,
        junior: ClientId,
    },
    /// `from` answered `inviter`'s junior request.
    FamilyAnswer {
        from: ClientId,
        inviter: ClientId,
        accept: bool,
    },
    /// `from` would part `junior` from `senior`; their levels now, for the
    /// fee.
    FamilySeparate {
        from: ClientId,
        senior: ClientId,
        junior: ClientId,
    },
    /// `from` earned their seniors `gain` Rep.
    FamilyEarn {
        from: ClientId,
        gain: i32,
    },
    /// `from` spent the teleport entitlement to go to `target`.
    FamilyLocate {
        from: ClientId,
        target: ClientId,
    },
    /// `from`, on `map_id` (named `map_name`), spent the summon
    /// entitlement to bring `target`, named `name`.
    FamilySummon {
        from: ClientId,
        target: ClientId,
        name: String,
        map_id: i32,
        map_name: String,
    },
    /// `from` answered the summons `summoner` sent them.
    FamilySummonAnswer {
        from: ClientId,
        summoner: String,
        accept: bool,
    },
    /// `from` said yes to `summoner` but may not leave their map.
    FamilySummonFailed {
        from: ClientId,
        summoner: ClientId,
    },
    /// `from` bought `buff` for their party, and has it: each other member
    /// on their channel gets it too.
    FamilyPartyBuff {
        from: ClientId,
        buff: FamilyBuff,
    },
    /// How many of `juniors` are online, for `from`'s bonding entitlement.
    FamilyBondingCount {
        from: ClientId,
        juniors: Vec<ClientId>,
    },
    /// These characters' families changed; those online read theirs again.
    FamilyChanged {
        ids: Vec<ClientId>,
    },
    /// Send `owner` the Family Tree with who is online, where.
    FamilyChart {
        owner: ClientId,
        chart: game::family::FamilyChart,
    },
    /// The guild is gone; its invitations no longer stand.
    GuildDisbanded {
        guild_id: i32,
    },
    /// Tell these alliance members the news, with who is online.
    AllianceNews {
        to: Vec<ClientId>,
        news: AllianceNews,
    },
    /// `from`, the master of `alliance_id`, invites the guild `guild_name`
    /// whose master is `master`.
    AllianceInvite {
        from: ClientId,
        alliance_id: i32,
        master: ClientId,
        guild_name: String,
    },
    /// `from` accepts the invitation to `alliance_id`; if it stands, they
    /// are told to join (`ServerMessage::JoinAlliance`).
    AllianceAccept {
        from: ClientId,
        alliance_id: i32,
    },
    /// `from`, the master of `guild_name`, turns down `inviter`'s
    /// invitation.
    AllianceDeny {
        from: ClientId,
        inviter: String,
        guild_name: String,
    },
    /// The alliance is gone; its invitations no longer stand.
    AllianceDisbanded {
        alliance_id: i32,
    },
    /// If `from` leads a party, its members on their map and channel,
    /// `from` first.
    PartyHere {
        from: ClientId,
        reply: tokio::sync::oneshot::Sender<Option<Vec<ClientId>>>,
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

/// What an alliance's members hear; the guilds it carries list who is
/// online.
#[derive(Clone, Debug)]
pub enum AllianceNews {
    /// The whole alliance.
    Info(Alliance),
    /// Guild `guild_id` joined it.
    GuildJoined { alliance: Alliance, guild_id: i32 },
    /// The guild left or was expelled; the alliance as it is without it.
    GuildLeft {
        alliance: Alliance,
        guild: Guild,
        expelled: bool,
    },
    /// Guild `guild_id`'s members changed.
    GuildChanged { alliance: Alliance, guild_id: i32 },
}

/// Word between the partners of a wedding being booked (report b066).
#[derive(Clone, Debug)]
pub enum WeddingNews {
    /// The partner `booker` books the couple's wedding at `venue`: check the
    /// player's side.
    Check {
        booker: ClientId,
        venue: game::wedding::Venue,
        premium: bool,
    },
    /// The partner's side of the booking the player asked for: whether they
    /// can't take part.
    Checked {
        venue: game::wedding::Venue,
        premium: bool,
        refusal: Option<game::wedding::PartnerRefusal>,
    },
    /// The partner booked the couple's wedding: take the invitations.
    Booked(game::wedding::Reservation),
    /// The player is invited to a wedding: take the invitation, or have it
    /// left at Duey without room for it.
    Invited(game::wedding::Invitation),
    /// The invitation the player sent could not reach its guest, who is
    /// off the list again: take it back.
    Returned(game::wedding::Invitation),
}
