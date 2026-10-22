mod family;
mod marriage;

use crate::actor::avatar_banners::{AvatarBanners, Queued};
use crate::actor::event::EventStart;
use crate::actor::guild_invites::GuildInvites;
use crate::actor::maple_tv::{MapleTvLine, OnAir};
use crate::actor::messenger::{Messengers, Outbox, CHANNEL_CHANGE_GRACE};
use crate::actor::parties::{self, Member, Parties, Party};
use crate::actor::party_search::{PartySearches, Wanted};
use crate::message::{
    AllianceNews, ClientHandle, DirectoryMessage, ListedBuddy, Presence, ServerMessage,
};
use game::alliance::Alliance;
use game::{AvatarLook, Guild};
use game_data::server_content::keys;
use net::conti::Portal;
use net::dungeons::Refusal;
use net::events::{self, Candidate, EventKind};
use net::maple_tv::MapleTv;
use net::owl::Searches;
use net::packet::build::world::alliance;
use net::packet::build::world::buddy::{build_buddy_list, BuddyView};
use net::packet::build::world::door::{build_spawn_portal, DoorView};
use net::packet::build::world::guild::{
    build_guild_error, build_guild_info, build_guild_invite, build_guild_job_change,
    build_guild_level_up, build_invite_refused, build_member_level_job, build_member_online,
    GuildError, InviteRefusal,
};
use net::packet::build::world::map::{build_blocked_map, BlockedMap};
use net::packet::build::world::maple_tv::{build_remove_tv, build_send_tv, TvRefusal};
use net::packet::build::world::messaging::{
    build_broadcast, build_find_result, build_whisper_result, Broadcast, FindLocation,
};
use net::packet::build::world::party::{
    build_party_chat, build_party_created, build_party_disbanded, build_party_door,
    build_party_invite, build_party_joined, build_party_leader_changed, build_party_left,
    build_party_member_hp, build_party_search_invite, build_party_status, build_party_update,
    Departure, PartyMemberView, PartyStatus, PartyView,
};
use net::packet::handle::world::{DoorExit, PartyPotion, Rock};
use net::{BroadcastScope, ClientId, MessengerRequest, PartyRequest, PartySkill, WhisperDelivery};
use packet::Packet;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

/// How often messenger seats and guild members held through a channel
/// change are checked.
const PURGE_INTERVAL: Duration = Duration::from_secs(10);

/// How long a mini-dungeon's copy stays taken for a team that hasn't
/// arrived yet.
const DUNGEON_GRACE: Duration = Duration::from_secs(10);

/// Wait until `instant`, or forever without one.
async fn at(instant: Option<Instant>) {
    match instant {
        Some(instant) => tokio::time::sleep_until(instant.into()).await,
        None => std::future::pending().await,
    }
}

struct Entry {
    handle: ClientHandle,
    presence: Presence,
    channel_id: u8,
    /// In the Cash Shop, off every map until they leave it, which
    /// registers them again.
    in_cash_shop: bool,
}

impl Entry {
    fn member(&self) -> Member {
        Member {
            id: self.handle.client_id,
            name: self.presence.name.clone(),
            job: self.presence.job,
            level: self.presence.level,
        }
    }

    fn shares_map_with(&self, other: &Entry) -> bool {
        self.channel_id == other.channel_id && self.presence.map_id == other.presence.map_id
    }
}

/// A mini-dungeon's copy in use: who took it, and until when it is kept
/// for them should nobody be in it yet.
struct Run {
    leader: ClientId,
    reserved_until: Instant,
    /// The members who follow the leader in once they arrive, to `portal`.
    team: Vec<ClientId>,
    portal: u8,
}

/// A guild member changing channel, and when they left.
#[derive(Clone, Copy)]
struct Away {
    guild_id: i32,
    alliance_id: Option<i32>,
    left: Instant,
}

/// A Mystic Door, open on `channel_id`.
struct Door {
    channel_id: u8,
    view: DoorView,
}

/// World-wide directory of online characters: who is online, under which
/// name, on which channel and map. It owns no character state; it exists
/// for features that cross fields (whispers, world broadcasts, parties, and
/// buddies).
pub struct WorldDirectory {
    rx: mpsc::Receiver<DirectoryMessage>,
    clients: HashMap<ClientId, Entry>,
    names: HashMap<String, ClientId>,
    parties: Parties,
    party_searches: PartySearches,
    guild_invites: GuildInvites,
    /// Invitations to alliances, to guild masters, by alliance id.
    alliance_invites: GuildInvites,
    messengers: Messengers,
    /// Each player's Mystic Door, which their party may use too.
    doors: HashMap<ClientId, Door>,
    /// Guild members changing channel: their guild and alliance hear they
    /// logged out only if they don't arrive in time.
    guild_away: HashMap<ClientId, Away>,
    /// Requests to be someone's junior, by the senior asking.
    family_invites: GuildInvites,
    /// Family members changing channel, who are said to log out only if
    /// they don't arrive in time.
    family_away: HashMap<ClientId, family::Away>,
    /// Summonses waiting for their answer, by the member summoned.
    family_summons: HashMap<ClientId, family::Summons>,
    /// Proposals waiting for their answer, by the partner asked.
    proposals: HashMap<ClientId, marriage::Proposal>,
    /// The event each channel is playing of each kind, by id.
    events: HashMap<(u8, EventKind), u32>,
    next_event: u32,
    /// The wedding under way at each channel's venues.
    weddings: HashMap<(u8, game::wedding::Venue), marriage::Wedding>,
    avatar_banners: AvatarBanners,
    maple_tv: MapleTvLine,
    /// Mini-dungeon copies taken, by channel and copy.
    dungeons: HashMap<(u8, i32), Run>,
    /// What the Owl of Minerva was asked to find.
    owl_searches: Searches,
}

impl WorldDirectory {
    pub fn new(rx: mpsc::Receiver<DirectoryMessage>) -> Self {
        Self {
            rx,
            clients: HashMap::new(),
            names: HashMap::new(),
            parties: Parties::default(),
            party_searches: PartySearches::default(),
            guild_invites: GuildInvites::default(),
            alliance_invites: GuildInvites::default(),
            messengers: Messengers::default(),
            doors: HashMap::new(),
            guild_away: HashMap::new(),
            family_invites: GuildInvites::default(),
            family_away: HashMap::new(),
            family_summons: HashMap::new(),
            proposals: HashMap::new(),
            events: HashMap::new(),
            next_event: 1,
            weddings: HashMap::new(),
            avatar_banners: AvatarBanners::default(),
            maple_tv: MapleTvLine::default(),
            dungeons: HashMap::new(),
            owl_searches: Searches::default(),
        }
    }

    pub async fn run(mut self) {
        let mut purge = tokio::time::interval(PURGE_INTERVAL);
        loop {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(message) => self.handle(message),
                    None => break,
                },
                _ = purge.tick() => {
                    let now = Instant::now();
                    let out = self.messengers.purge(now);
                    self.deliver(out);
                    self.purge_guild_away(now);
                    self.purge_family_away(now);
                    self.purge_family_summons(now);
                    self.run_party_searches(now);
                }
                _ = at(self.avatar_banners.until()) => {
                    if let Some(banner) = self.avatar_banners.next(Instant::now()) {
                        self.show_everyone(banner);
                    }
                }
                _ = at(self.maple_tv.until()) => self.next_tv(Instant::now()),
            }
        }
    }

    fn handle(&mut self, message: DirectoryMessage) {
        match message {
            DirectoryMessage::Register {
                handle,
                presence,
                channel_id,
            } => self.register(handle, presence, channel_id),
            DirectoryMessage::Unregister {
                client_id,
                connection_id,
                changing_channel,
            } => self.unregister(client_id, connection_id, changing_channel),
            DirectoryMessage::Presence {
                client_id,
                presence,
            } => self.update_presence(client_id, presence),
            DirectoryMessage::Whisper { from, delivery } => self.whisper(from, delivery),
            DirectoryMessage::Find { from, target_name } => self.find(from, &target_name),
            DirectoryMessage::RockLocate { from, name, rock } => {
                self.rock_locate(from, &name, rock)
            }
            DirectoryMessage::MorphAnother { from, potion } => self.morph_another(from, potion),
            DirectoryMessage::Broadcast {
                from,
                scope,
                packet,
            } => self.broadcast(from, scope, packet),
            DirectoryMessage::AvatarBanner { banner, reply } => {
                let queued = match self.avatar_banners.queue(banner, Instant::now()) {
                    Queued::Now(banner) => {
                        self.show_everyone(banner);
                        true
                    }
                    Queued::Later => true,
                    Queued::Refused => false,
                };
                let _ = reply.send(queued);
            }
            DirectoryMessage::MapleTv { tv, partner, reply } => {
                self.maple_tv(tv, partner, reply, Instant::now())
            }
            DirectoryMessage::OwlSearched { item_id } => self.owl_searches.record(item_id),
            DirectoryMessage::OwlPopular { reply } => {
                let _ = reply.send(self.owl_searches.popular());
            }
            DirectoryMessage::Party { from, request } => {
                self.party_request(from, request, Instant::now())
            }
            DirectoryMessage::PartySkill { from, used, origin } => {
                self.party_skill(from, used, origin)
            }
            DirectoryMessage::PartyItem { from, item_id } => self.party_item(from, item_id),
            DirectoryMessage::BuddyList { owner, buddies } => self.send_buddy_list(owner, buddies),
            DirectoryMessage::GuildInfo { owner, guild } => self.send_guild_info(owner, guild),
            DirectoryMessage::Online { client_id, reply } => {
                let _ = reply.send(self.clients.contains_key(&client_id));
            }
            DirectoryMessage::EnterDungeon {
                from,
                first,
                count,
                portal,
                reply,
            } => {
                let entered = self.enter_dungeon(from, first, count, Instant::now());
                match (reply.send(entered), entered) {
                    (Ok(()), Ok(copy)) => self.gather_team(from, copy, portal),
                    // A leader who can't take the reply doesn't go.
                    (Err(_), Ok(copy)) => {
                        warn!(leader = from, copy, "Mini-dungeon reply dropped");
                        if let Some(entry) = self.clients.get(&from) {
                            self.dungeons.remove(&(entry.channel_id, copy));
                        }
                    }
                    (_, Err(_)) => {}
                }
            }
            DirectoryMessage::MapOccupied {
                channel_id,
                map_id,
                reply,
            } => {
                let occupied = self
                    .clients
                    .values()
                    .any(|entry| entry.channel_id == channel_id && entry.presence.map_id == map_id);
                let _ = reply.send(occupied);
            }
            DirectoryMessage::EventEntry { from, kind, reply } => {
                let _ = reply.send(self.event_entry(from, kind).0);
            }
            DirectoryMessage::StartEvent { from, kind, reply } => {
                // A leader who can't take the reply can't start it.
                if let Err(Some(start)) = reply.send(self.start_event(from, kind)) {
                    self.events.remove(&(start.channel_id, kind));
                }
            }
            DirectoryMessage::EventClosed {
                channel_id,
                kind,
                id,
            } => {
                if self.events.get(&(channel_id, kind)) == Some(&id) {
                    self.events.remove(&(channel_id, kind));
                }
            }
            DirectoryMessage::Deliver { to, packet } => {
                for client in to {
                    if let Some(entry) = self.clients.get(&client) {
                        entry.handle.send_packet(packet.clone());
                    }
                }
            }
            DirectoryMessage::GuildInvite {
                from,
                guild_id,
                name,
            } => self.guild_invite(from, guild_id, &name, Instant::now()),
            DirectoryMessage::GuildAccept { from, guild_id } => {
                if self.guild_invites.accept(from, guild_id, Instant::now()) {
                    self.tell(from, ServerMessage::JoinGuild { guild_id });
                }
            }
            DirectoryMessage::GuildDeny {
                from,
                inviter,
                refusal,
            } => self.guild_deny(from, &inviter, refusal),
            DirectoryMessage::GuildChanged { client_id } => {
                self.tell(client_id, ServerMessage::GuildChanged)
            }
            DirectoryMessage::Propose {
                from,
                partner,
                item_id,
            } => self.propose(from, partner, item_id),
            DirectoryMessage::ProposalCancel { from } => self.cancel_proposal(from),
            DirectoryMessage::ProposalAnswer {
                from,
                proposer,
                accept,
            } => self.answer_proposal(from, proposer, accept),
            DirectoryMessage::ProposalRefused { from, refusal } => {
                self.refuse_proposal(from, refusal)
            }
            DirectoryMessage::Engaged { proposer, record } => self.engaged(proposer, record),
            DirectoryMessage::EnteredCashShop { from } => {
                self.forget_proposals(from);
                if let Some(entry) = self.clients.get_mut(&from) {
                    entry.in_cash_shop = true;
                }
            }
            DirectoryMessage::ReserveWedding {
                from,
                partner,
                venue,
                premium,
            } => self.reserve_wedding(from, partner, venue, premium),
            DirectoryMessage::StartWedding {
                from,
                partner,
                venue,
                marriage_id,
                reply,
            } => {
                let start = self.start_wedding(from, partner, venue, marriage_id);
                // A partner who can't take the reply can't start it.
                if let Err(Ok(start)) = reply.send(start) {
                    self.weddings.remove(&(start.channel_id, venue));
                }
            }
            DirectoryMessage::JoinWedding {
                from,
                venue,
                marriage_id,
                reply,
            } => self.join_wedding(from, venue, marriage_id, reply),
            DirectoryMessage::WeddingClosed {
                channel_id,
                venue,
                id,
            } => {
                if self
                    .weddings
                    .get(&(channel_id, venue))
                    .is_some_and(|wedding| wedding.id == id)
                {
                    self.weddings.remove(&(channel_id, venue));
                }
            }
            DirectoryMessage::Wedding { to, news } => self.tell(to, ServerMessage::Wedding(news)),
            DirectoryMessage::DeliverInvitation { invitation } => {
                self.deliver_invitation(invitation)
            }
            DirectoryMessage::ReturnInvitation { invitation } => self.return_invitation(invitation),
            DirectoryMessage::CoupleEnded { partner, record } => {
                self.tell(partner, ServerMessage::CoupleEnded(record))
            }
            DirectoryMessage::FamilyInvite { from, junior } => {
                self.family_invite(from, junior, Instant::now())
            }
            DirectoryMessage::FamilyAnswer {
                from,
                inviter,
                accept,
            } => self.family_answer(from, inviter, accept, Instant::now()),
            DirectoryMessage::FamilyChart { owner, chart } => self.send_family_chart(owner, chart),
            DirectoryMessage::FamilySeparate {
                from,
                senior,
                junior,
            } => {
                let level = |id| self.clients.get(&id).map(|entry| entry.presence.level);
                let levels = (level(senior), level(junior));
                self.tell(
                    from,
                    ServerMessage::Separate {
                        senior,
                        junior,
                        levels,
                    },
                );
            }
            DirectoryMessage::FamilyEarn { from, gain } => self.family_earn(from, gain),
            DirectoryMessage::FamilyLocate { from, target } => self.family_locate(from, target),
            DirectoryMessage::FamilySummon {
                from,
                target,
                name,
                map_id,
                map_name,
            } => self.family_summon(from, target, name, (map_id, &map_name), Instant::now()),
            DirectoryMessage::FamilySummonAnswer {
                from,
                summoner,
                accept,
            } => self.family_summon_answer(from, &summoner, accept),
            DirectoryMessage::FamilySummonFailed { from, summoner } => {
                self.family_summon_failed(from, summoner)
            }
            DirectoryMessage::FamilyPartyBuff { from, buff } => self.family_party_buff(from, buff),
            DirectoryMessage::FamilyBondingCount { from, juniors } => {
                let online = juniors
                    .iter()
                    .filter(|id| self.clients.contains_key(id))
                    .count();
                self.tell(from, ServerMessage::FamilyBondingCount(online));
            }
            DirectoryMessage::FamilyChanged { ids } => {
                for id in ids {
                    self.tell(id, ServerMessage::FamilyChanged);
                }
            }
            DirectoryMessage::GuildDisbanded { guild_id } => {
                self.guild_invites.forget_guild(guild_id)
            }
            DirectoryMessage::AllianceNews { to, news } => self.alliance_news(&to, news),
            DirectoryMessage::AllianceInvite {
                from,
                alliance_id,
                master,
                guild_name,
            } => self.alliance_invite(from, alliance_id, master, &guild_name, Instant::now()),
            DirectoryMessage::AllianceAccept { from, alliance_id } => {
                if self
                    .alliance_invites
                    .accept(from, alliance_id, Instant::now())
                {
                    self.tell(from, ServerMessage::JoinAlliance { alliance_id });
                }
            }
            DirectoryMessage::AllianceDeny {
                from,
                inviter,
                guild_name,
            } => {
                let Some(&inviter) = self.names.get(&inviter) else {
                    // Gone offline: no one to tell, but the card is answered.
                    return self.alliance_invites.forget(from);
                };
                if self.alliance_invites.decline(from, inviter) {
                    let text = net::content::text()
                        .get(keys::ALLIANCE_DENIED, &[("guild", guild_name.as_str())]);
                    self.send_packet(inviter, build_broadcast(Broadcast::Pink, &text));
                }
            }
            DirectoryMessage::AllianceDisbanded { alliance_id } => {
                self.alliance_invites.forget_guild(alliance_id)
            }
            DirectoryMessage::PartyHere { from, reply } => {
                let _ = reply.send(self.party_here(from));
            }
            DirectoryMessage::Messenger { from, request } => self.messenger(from, request),
            DirectoryMessage::LookChanged { client_id, look } => {
                let out = self.messengers.look_changed(client_id, look);
                self.deliver(out);
            }
            DirectoryMessage::DoorOpened { owner, door } => self.door_opened(owner, door),
            DirectoryMessage::DoorClosed { owner } => {
                if self.doors.remove(&owner).is_some() {
                    self.tell_door(owner, None);
                }
            }
            DirectoryMessage::UseDoor {
                from,
                owner,
                from_town,
            } => self.use_door(from, owner, from_town),
        }
    }

    /// Whether `from`'s party may go into an event of `kind` on their
    /// channel, and who would go.
    fn event_entry(&self, from: ClientId, kind: EventKind) -> (events::Entry, Vec<ClientId>) {
        let Some(asker) = self.clients.get(&from) else {
            return (events::Entry::NoParty, Vec::new());
        };
        let Some(party) = self.parties.of(from) else {
            return (events::Entry::NoParty, Vec::new());
        };
        if party.leader != from {
            return (events::Entry::NotLeader, Vec::new());
        }
        let candidates: Vec<_> = party
            .members
            .iter()
            .filter_map(|member| self.clients.get(&member.id))
            // Those in the Cash Shop are on no map (Cosmic's
            // `isLoggedinWorld`).
            .filter(|entry| entry.channel_id == asker.channel_id && !entry.in_cash_shop)
            .map(|entry| Candidate {
                id: entry.handle.client_id,
                level: entry.presence.level,
                map_id: entry.presence.map_id,
            })
            .collect();
        let Some(team) = kind.event().team(from, &candidates) else {
            return (events::Entry::NotEligible, Vec::new());
        };
        if self.events.contains_key(&(asker.channel_id, kind)) {
            return (events::Entry::Busy, Vec::new());
        }
        (events::Entry::Ready, team)
    }

    /// Reserve an event of `kind` on `from`'s channel for their party, if
    /// it may still go.
    fn start_event(&mut self, from: ClientId, kind: EventKind) -> Option<EventStart> {
        let (events::Entry::Ready, team) = self.event_entry(from, kind) else {
            return None;
        };
        let channel_id = self.clients.get(&from)?.channel_id;
        let id = self.next_event;
        self.next_event = self.next_event.wrapping_add(1).max(1);
        self.events.insert((channel_id, kind), id);
        info!(
            ?kind,
            id,
            channel_id,
            leader = from,
            ?team,
            "Event reserved"
        );
        Some(EventStart {
            id,
            kind,
            channel_id,
            leader: from,
            team: team
                .iter()
                .filter_map(|member| self.clients.get(member))
                .map(|entry| entry.handle.clone())
                .collect(),
        })
    }

    /// Take the first free of the `count` mini-dungeon copies from `first`
    /// on `from`'s channel for them: free means nobody is in it and it isn't
    /// kept for a team on its way in. A party member who doesn't lead it
    /// can't.
    fn enter_dungeon(
        &mut self,
        from: ClientId,
        first: i32,
        count: i32,
        now: Instant,
    ) -> Result<i32, Refusal> {
        let channel_id = self.clients.get(&from).ok_or(Refusal::Full)?.channel_id;
        if self
            .parties
            .of(from)
            .is_some_and(|party| party.leader != from)
        {
            return Err(Refusal::NotLeader);
        }
        let occupied: HashSet<i32> = self
            .clients
            .values()
            .filter(|entry| entry.channel_id == channel_id)
            .map(|entry| entry.presence.map_id)
            .collect();
        let copy = (first..first + count)
            .find(|copy| {
                let kept = self
                    .dungeons
                    .get(&(channel_id, *copy))
                    .is_some_and(|run| run.reserved_until > now);
                !kept && !occupied.contains(copy)
            })
            .ok_or(Refusal::Full)?;
        self.dungeons.insert(
            (channel_id, copy),
            Run {
                leader: from,
                reserved_until: now + DUNGEON_GRACE,
                team: Vec::new(),
                portal: 0,
            },
        );
        info!(leader = from, channel_id, copy, "Mini-dungeon taken");
        Ok(copy)
    }

    /// The members of the party `from` leads who stand beside them follow
    /// `from` to `portal` of the copy they took, once `from` is in.
    fn gather_team(&mut self, from: ClientId, copy: i32, portal: u8) {
        let Some(channel_id) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return;
        };
        let mut team = self.party_here(from).unwrap_or_default();
        team.retain(|&member| member != from);
        if let Some(run) = self.dungeons.get_mut(&(channel_id, copy)) {
            run.team = team;
            run.portal = portal;
        }
    }

    /// `client_id` came to `map_id` from `from_map`. If they took it as a
    /// mini-dungeon's copy, the members still beside where they were follow.
    fn bring_team(&mut self, client_id: ClientId, channel_id: u8, map_id: i32, from_map: i32) {
        let Some(run) = self
            .dungeons
            .get_mut(&(channel_id, map_id))
            .filter(|run| run.leader == client_id)
        else {
            return;
        };
        let (team, portal) = (std::mem::take(&mut run.team), run.portal);
        for member in team {
            self.tell(
                member,
                ServerMessage::ShipWarp {
                    from_map,
                    map_id,
                    portal: Portal::Id(portal),
                },
            );
        }
    }

    /// `from` handed their party to `to`. A mini-dungeon's copy `from` took
    /// goes with it if `to` is in it too; if not, the run is over and
    /// everyone in it goes back to the dungeon's map (as in Cosmic).
    fn dungeon_leader_changed(&mut self, from: ClientId, to: ClientId) {
        let Some(entry) = self.clients.get(&from) else {
            return;
        };
        let (channel_id, map_id) = (entry.channel_id, entry.presence.map_id);
        let Some(run) = self
            .dungeons
            .get_mut(&(channel_id, map_id))
            .filter(|run| run.leader == from)
        else {
            return;
        };
        let beside = self
            .clients
            .get(&to)
            .is_some_and(|other| other.channel_id == channel_id && other.presence.map_id == map_id);
        if beside {
            run.leader = to;
            return;
        }
        if let Some(dungeon) = net::dungeons::of_copy(map_id) {
            for entry in self.clients.values() {
                if entry.channel_id == channel_id && entry.presence.map_id == map_id {
                    entry.handle.send(ServerMessage::ShipWarp {
                        from_map: map_id,
                        map_id: dungeon.base,
                        portal: Portal::Id(0),
                    });
                }
            }
        }
        self.dungeons.remove(&(channel_id, map_id));
    }

    /// `client_id` left `map_id` on `channel_id`. If it is a mini-dungeon's
    /// copy they took, the run is over: whoever is still in it goes back to
    /// the dungeon's map.
    fn left_dungeon(&mut self, client_id: ClientId, channel_id: u8, map_id: i32) {
        let key = (channel_id, map_id);
        if !matches!(self.dungeons.get(&key), Some(run) if run.leader == client_id) {
            return;
        }
        self.dungeons.remove(&key);
        let Some(dungeon) = net::dungeons::of_copy(map_id) else {
            return;
        };
        for entry in self.clients.values() {
            if entry.channel_id == channel_id
                && entry.presence.map_id == map_id
                && entry.handle.client_id != client_id
            {
                entry.handle.send(ServerMessage::ShipWarp {
                    from_map: map_id,
                    map_id: dungeon.base,
                    portal: Portal::Id(0),
                });
            }
        }
    }

    fn register(&mut self, handle: ClientHandle, presence: Presence, channel_id: u8) {
        let client_id = handle.client_id;
        let (guild_id, alliance_id) = (presence.guild_id, presence.alliance_id);
        let previous = self.clients.remove(&client_id);
        if let Some(previous) = &previous {
            // The same character connected again: only one session may live.
            warn!(
                client_id,
                "Duplicate login; disconnecting previous connection"
            );
            previous.handle.kick();
            self.names.remove(&previous.presence.name);
            // Its unregister will be stale; the search and the run end here
            // instead.
            self.party_searches.stop(client_id);
            self.left_dungeon(client_id, previous.channel_id, previous.presence.map_id);
            self.forget_proposals(client_id);
        }
        info!(
            client_id,
            name = presence.name,
            channel_id,
            "Character online"
        );
        self.names.insert(presence.name.clone(), client_id);
        let entry = Entry {
            handle,
            presence,
            channel_id,
            in_cash_shop: false,
        };
        let member = entry.member();
        self.clients.insert(client_id, entry);

        // Back online: rejoin the party view and its HP bars.
        self.parties.update(member);
        if let Some(party) = self.parties.of(client_id) {
            self.tell_party_changed(client_id, Some(party.id));
            self.send_party_update(party);
            self.exchange_hp(client_id);
        }
        let out = self.messengers.connected(client_id, channel_id);
        self.deliver(out);
        self.show_tv_to(client_id, Instant::now());

        // The guild already knows of a second login, or one from another
        // channel.
        let away = self.guild_away.remove(&client_id);
        if let (None, Some(guild_id)) = (&previous, guild_id) {
            if away.map(|away| away.guild_id) != Some(guild_id) {
                self.tell_guild_online(guild_id, client_id, true);
            }
            if let Some(alliance_id) = alliance_id {
                if away.and_then(|away| away.alliance_id) != Some(alliance_id) {
                    self.tell_alliance_online(alliance_id, guild_id, client_id, true);
                }
            }
        }
        let back = self.family_away.remove(&client_id).is_some();
        if previous.is_none() && !back {
            self.tell_relatives(client_id, true);
        }
        self.exchange_partner_maps(client_id);
    }

    fn unregister(&mut self, client_id: ClientId, connection_id: u64, changing_channel: bool) {
        // Ignore a stale unregister from a connection that was replaced.
        let is_current = self
            .clients
            .get(&client_id)
            .is_some_and(|entry| entry.handle.connection_id == connection_id);
        if is_current {
            self.doors.remove(&client_id);
            if let Some(entry) = self.clients.remove(&client_id) {
                self.names.remove(&entry.presence.name);
                info!(client_id, "Character offline");
                if let Some(guild_id) = entry.presence.guild_id {
                    let away = Away {
                        guild_id,
                        alliance_id: entry.presence.alliance_id,
                        left: Instant::now(),
                    };
                    if changing_channel {
                        self.guild_away.insert(client_id, away);
                    } else {
                        self.tell_offline(client_id, away);
                    }
                }
                self.family_left(&entry, changing_channel);
                if let Some(partner) = self.partner_naming(&entry.presence, client_id) {
                    self.tell(
                        partner,
                        ServerMessage::PartnerMap {
                            map_id: 0,
                            partner: 0,
                        },
                    );
                }
                let out = self
                    .messengers
                    .disconnected(client_id, changing_channel, Instant::now());
                self.deliver(out);
                self.left_dungeon(client_id, entry.channel_id, entry.presence.map_id);
            }
            self.parties.forget_invites(client_id);
            // A leader changing channel starts again on arrival (b034
            // section 7).
            self.party_searches.stop(client_id);
            self.guild_invites.forget(client_id);
            self.alliance_invites.forget(client_id);
            self.family_invites.forget(client_id);
            // Requests the player made go with them (keyed by the senior).
            self.family_invites.forget_guild(client_id);
            self.forget_proposals(client_id);
            if let Some(party) = self.parties.of(client_id) {
                self.send_party_update(party);
            }
        }
    }

    fn update_presence(&mut self, client_id: ClientId, presence: Presence) {
        let Some(entry) = self.clients.get_mut(&client_id) else {
            return;
        };
        let previous = std::mem::replace(&mut entry.presence, presence);
        let member = entry.member();
        let moved = previous.map_id != entry.presence.map_id;
        let hp_changed =
            (previous.hp, previous.max_hp) != (entry.presence.hp, entry.presence.max_hp);
        if moved {
            let (channel_id, map_id) = (entry.channel_id, entry.presence.map_id);
            self.left_dungeon(client_id, channel_id, previous.map_id);
            self.bring_team(client_id, channel_id, map_id, previous.map_id);
        }

        self.tell_partner_progress(client_id, &previous);
        // Guild members needn't be in a party.
        self.tell_guild_progress(client_id, &previous);
        self.tell_family_progress(client_id, &previous);

        let updated = self.parties.update(member).is_some();
        let Some(party) = self.parties.of(client_id) else {
            return;
        };
        if updated || moved {
            // Maps show in the party window; levels and jobs too.
            self.send_party_update(party);
        }
        if moved {
            self.exchange_hp(client_id);
        } else if hp_changed {
            self.send_hp_to_map(client_id);
        }
    }

    fn whisper(&self, from: ClientId, delivery: WhisperDelivery) {
        let Some(sender) = self.clients.get(&from) else {
            return;
        };
        let delivered = self
            .names
            .get(&delivery.target_name)
            .and_then(|id| self.clients.get(id))
            .is_some_and(|target| target.handle.send_packet(delivery.recipient_packet));
        let answer = if delivered {
            delivery.sender_success_packet
        } else {
            delivery.sender_failure_packet
        };
        if let Some(answer) = answer {
            sender.handle.send_packet(answer);
        }
    }

    /// Tell the asker where a character is: their channel if it isn't the
    /// asker's, otherwise their map, which only their own client knows.
    fn find(&self, from: ClientId, target_name: &str) {
        let Some(asker) = self.clients.get(&from) else {
            return;
        };
        let target = self
            .names
            .get(target_name)
            .and_then(|id| self.clients.get(id));
        let packet = match target {
            Some(target) if target.channel_id == asker.channel_id => {
                target.handle.send(ServerMessage::Locate {
                    reply_to: asker.handle.clone(),
                });
                return;
            }
            Some(target) => build_find_result(
                &target.presence.name,
                FindLocation::Channel(target.channel_id),
            ),
            None => build_whisper_result(target_name, false),
        };
        match packet {
            Ok(packet) => {
                asker.handle.send_packet(packet);
            }
            Err(e) => error!(client_id = from, "find result: {e}"),
        }
    }

    /// Ask the character named `name` where they stand, if they are on the
    /// asker's channel; otherwise tell the asker they weren't found.
    fn rock_locate(&self, from: ClientId, name: &str, rock: Rock) {
        let Some(asker) = self.clients.get(&from) else {
            return;
        };
        let target = self
            .names
            .get(name)
            .and_then(|id| self.clients.get(id))
            .filter(|target| target.channel_id == asker.channel_id);
        match target {
            Some(target) => target.handle.send(ServerMessage::RockLocate {
                reply_to: asker.handle.clone(),
                rock,
            }),
            None => asker
                .handle
                .send(ServerMessage::RockLocated { rock, found: None }),
        };
    }

    /// Hand a Party Potion to the character it names, if they are on the
    /// user's channel; otherwise tell the user they weren't found.
    fn morph_another(&self, from: ClientId, potion: PartyPotion) {
        let Some(user) = self.clients.get(&from) else {
            return;
        };
        let target = self
            .names
            .get(&potion.name)
            .and_then(|id| self.clients.get(id))
            .filter(|target| target.channel_id == user.channel_id);
        // A target that has just gone counts as not found, so the user's
        // client is always unlocked.
        let reached = target.is_some_and(|target| {
            target.handle.send(ServerMessage::MorphAnother {
                reply_to: user.handle.clone(),
                potion: potion.clone(),
            })
        });
        if !reached {
            user.handle.send(ServerMessage::MorphedAnother {
                potion,
                morphed: false,
            });
        }
    }

    fn messenger(&mut self, from: ClientId, request: MessengerRequest) {
        let Some(sender) = self.clients.get(&from) else {
            return;
        };
        let out = match request {
            MessengerRequest::Open { messenger_id, look } => self.messengers.open(
                from,
                &sender.presence.name,
                sender.channel_id,
                look,
                messenger_id,
            ),
            MessengerRequest::Leave => self.messengers.leave(from),
            MessengerRequest::Invite { target } => {
                let target_id = self.names.get(&target).copied();
                self.messengers.invite(from, target_id, &target)
            }
            MessengerRequest::Decline { inviter, blocking } => match self.names.get(&inviter) {
                Some(&inviter) => {
                    self.messengers
                        .decline(from, &sender.presence.name, inviter, blocking)
                }
                None => Outbox::new(),
            },
            MessengerRequest::Chat { line } => self.messengers.chat(from, &line),
        };
        self.deliver(out);
    }

    fn deliver(&self, out: Outbox) {
        for (to, packet) in out {
            if let Some(entry) = self.clients.get(&to) {
                entry.handle.send_packet(packet);
            }
        }
    }

    fn broadcast(&self, from: ClientId, scope: BroadcastScope, packet: Packet) {
        for (&client_id, entry) in &self.clients {
            let reached = match scope {
                BroadcastScope::World => true,
                BroadcastScope::WorldExcludeSelf => client_id != from,
                BroadcastScope::Channel(channel_id) => entry.channel_id == channel_id,
            };
            if reached {
                entry.handle.send_packet(packet.clone());
            }
        }
    }

    /// Put a Maple TV message in line, once it has its receiver's look.
    fn maple_tv(
        &mut self,
        tv: MapleTv,
        partner: Option<AvatarLook>,
        reply: oneshot::Sender<Result<(), TvRefusal>>,
        now: Instant,
    ) {
        if let (Some(name), None) = (&tv.receiver, &partner) {
            // The receiver's client adds the look and sends it back; if it
            // has gone, the dropped reply says so.
            match self.names.get(name).and_then(|id| self.clients.get(id)) {
                Some(receiver) => {
                    receiver
                        .handle
                        .send(ServerMessage::MapleTvPartner { tv, reply });
                }
                None => {
                    let _ = reply.send(Err(TvRefusal::WrongName));
                }
            }
            return;
        }
        let queued = self.maple_tv.queue(OnAir { tv, partner }, now);
        let _ = reply.send(queued.map(|_| ()).map_err(|()| TvRefusal::LineTooLong));
        if queued == Ok(true) {
            self.show_tv(now);
        }
    }

    /// The message up has had its time: clear it and show the next.
    fn next_tv(&mut self, now: Instant) {
        if let (Some(scope), _) = self.maple_tv.next(now) {
            match build_remove_tv() {
                Ok(packet) => self.broadcast(0, scope, packet),
                Err(e) => warn!(error = %e, "Failed to build REMOVE_TV"),
            }
        }
        self.show_tv(now);
    }

    /// The Maple TV packet of the message up, with the wait left in line.
    fn tv_packet(&self, now: Instant) -> Option<(BroadcastScope, Packet)> {
        let up = self.maple_tv.up()?;
        let wait = self.maple_tv.wait(now).as_secs() as i32;
        match build_send_tv(&up.tv, up.partner.as_ref(), wait) {
            Ok(packet) => Some((up.tv.scope, packet)),
            Err(e) => {
                warn!(error = %e, "Failed to build SEND_TV");
                None
            }
        }
    }

    fn show_tv(&self, now: Instant) {
        if let Some((scope, packet)) = self.tv_packet(now) {
            self.broadcast(0, scope, packet);
        }
    }

    /// One who comes online sees the message up, if it reaches them.
    fn show_tv_to(&self, client_id: ClientId, now: Instant) {
        let Some(entry) = self.clients.get(&client_id) else {
            return;
        };
        if let Some((scope, packet)) = self.tv_packet(now) {
            let reached = match scope {
                BroadcastScope::Channel(channel_id) => entry.channel_id == channel_id,
                _ => true,
            };
            if reached {
                entry.handle.send_packet(packet);
            }
        }
    }

    fn show_everyone(&self, packet: Packet) {
        for entry in self.clients.values() {
            entry.handle.send_packet(packet.clone());
        }
    }

    /// Only mutual buddies show their channel.
    fn send_buddy_list(&self, owner: ClientId, buddies: Vec<ListedBuddy>) {
        let views: Vec<_> = buddies
            .into_iter()
            .map(|buddy| BuddyView {
                channel: self
                    .clients
                    .get(&buddy.id)
                    .filter(|_| buddy.mutual)
                    .map(|entry| entry.channel_id),
                id: buddy.id,
                name: buddy.name,
                group: buddy.group,
                requesting: buddy.pending,
            })
            .collect();
        self.send_packet(owner, build_buddy_list(&views));
    }

    /// Send `owner` their guild with who is online.
    fn send_guild_info(&self, owner: ClientId, mut guild: Guild) {
        self.mark_online(&mut guild);
        self.send_packet(owner, build_guild_info(Some(&guild)));
    }

    /// Members online on any channel, or changing channel, show online,
    /// at the level and job they have now.
    fn mark_online(&self, guild: &mut Guild) {
        for member in &mut guild.members {
            match self.clients.get(&member.id) {
                Some(entry) => {
                    member.online = true;
                    member.level = entry.presence.level;
                    member.job = entry.presence.job;
                }
                None => member.online = self.guild_away.contains_key(&member.id),
            }
        }
    }

    /// Send the news to whichever of `to` are online, with the alliance's
    /// members marked as `mark_online` does.
    fn alliance_news(&self, to: &[ClientId], news: AllianceNews) {
        let mark = |alliance: &mut Alliance| {
            for guild in &mut alliance.guilds {
                self.mark_online(guild);
            }
        };
        let packet = match news {
            AllianceNews::Info(mut alliance) => {
                mark(&mut alliance);
                alliance::build_alliance_info(&alliance)
            }
            AllianceNews::GuildJoined {
                mut alliance,
                guild_id,
            } => {
                mark(&mut alliance);
                match alliance.guild(guild_id) {
                    Some(guild) => alliance::build_guild_joined(&alliance, guild),
                    None => return,
                }
            }
            AllianceNews::GuildLeft {
                mut alliance,
                mut guild,
                expelled,
            } => {
                mark(&mut alliance);
                self.mark_online(&mut guild);
                alliance::build_guild_left(&alliance, &guild, expelled)
            }
            AllianceNews::GuildChanged {
                mut alliance,
                guild_id,
            } => {
                mark(&mut alliance);
                match alliance.guild(guild_id) {
                    Some(guild) => alliance::build_guild_changed(alliance.id, guild),
                    None => return,
                }
            }
        };
        match packet {
            Ok(packet) => {
                for client in to {
                    if let Some(entry) = self.clients.get(client) {
                        entry.handle.send_packet(packet.clone());
                    }
                }
            }
            Err(error) => warn!(%error, "Failed to build alliance packet"),
        }
    }

    /// The members of alliance `alliance_id` online, on any channel, but
    /// for those of guild `guild_id`, who hear of each other as guild
    /// members.
    fn alliance_online(&self, alliance_id: i32, guild_id: i32) -> impl Iterator<Item = &Entry> {
        self.clients.values().filter(move |entry| {
            entry.presence.alliance_id == Some(alliance_id)
                && entry.presence.guild_id != Some(guild_id)
        })
    }

    /// Tell the alliance's other guilds that `member` entered or left the
    /// game; the client shows a card for it.
    fn tell_alliance_online(
        &self,
        alliance_id: i32,
        guild_id: i32,
        member: ClientId,
        online: bool,
    ) {
        let packet = match alliance::build_member_online(alliance_id, guild_id, member, online) {
            Ok(packet) => packet,
            Err(error) => return warn!(member, %error, "Failed to build packet"),
        };
        for entry in self.alliance_online(alliance_id, guild_id) {
            entry.handle.send_packet(packet.clone());
        }
    }

    /// `member` left the game: their guild and alliance hear so.
    fn tell_offline(&self, member: ClientId, away: Away) {
        self.tell_guild_online(away.guild_id, member, false);
        if let Some(alliance_id) = away.alliance_id {
            self.tell_alliance_online(alliance_id, away.guild_id, member, false);
        }
    }

    /// Pass on an alliance's invitation to the master of another guild,
    /// online anywhere in the world, unless they are considering another.
    fn alliance_invite(
        &mut self,
        from: ClientId,
        alliance_id: i32,
        master: ClientId,
        guild_name: &str,
        now: Instant,
    ) {
        let Some(inviter) = self.clients.get(&from) else {
            return;
        };
        let pink = |key| {
            let text = net::content::text().get(key, &[("guild", guild_name)]);
            build_broadcast(Broadcast::Pink, &text)
        };
        if !self.clients.contains_key(&master) {
            return self.send_packet(from, pink(keys::ALLIANCE_OFFLINE));
        }
        // A second card would be refused as another one showing, which
        // would take back the first.
        if self
            .alliance_invites
            .is_showing(master, from, alliance_id, now)
        {
            return;
        }
        let packet = alliance::build_alliance_invite(alliance_id, &inviter.presence.name);
        if self.alliance_invites.invite(alliance_id, from, master, now) {
            self.send_packet(master, packet);
        } else {
            self.send_packet(from, pink(keys::ALLIANCE_BUSY));
        }
    }

    /// The party `from` leads: its members on their map and channel, `from`
    /// first. None if they lead none.
    fn party_here(&self, from: ClientId) -> Option<Vec<ClientId>> {
        let leader = self.clients.get(&from)?;
        let party = self.parties.of(from).filter(|party| party.leader == from)?;
        let mut here = vec![from];
        here.extend(
            party
                .members
                .iter()
                .filter(|member| member.id != from)
                .filter_map(|member| self.clients.get(&member.id))
                .filter(|entry| entry.shares_map_with(leader))
                .map(|entry| entry.handle.client_id),
        );
        Some(here)
    }

    /// The members of guild `guild_id` online, on any channel.
    fn guild_online(&self, guild_id: i32) -> impl Iterator<Item = (ClientId, &Entry)> {
        self.clients
            .iter()
            .filter(move |(_, entry)| entry.presence.guild_id == Some(guild_id))
            .map(|(&id, entry)| (id, entry))
    }

    /// Tell the rest of `member`'s guild they entered or left the game. The
    /// client would show the card about themself, so they aren't told.
    fn tell_guild_online(&self, guild_id: i32, member: ClientId, online: bool) {
        let packet = match build_member_online(guild_id, member, online) {
            Ok(packet) => packet,
            Err(error) => return warn!(member, %error, "Failed to build packet"),
        };
        for (id, entry) in self.guild_online(guild_id) {
            if id != member {
                entry.handle.send_packet(packet.clone());
            }
        }
    }

    /// Members who changed channel and didn't arrive in time left the game.
    fn purge_guild_away(&mut self, now: Instant) {
        let gone: Vec<_> = self
            .guild_away
            .iter()
            .filter(|(_, away)| now.duration_since(away.left) >= CHANNEL_CHANGE_GRACE)
            .map(|(&id, &away)| (id, away))
            .collect();
        for (id, away) in gone {
            self.guild_away.remove(&id);
            self.tell_offline(id, away);
        }
    }

    /// A member's new level or job changes the guild list for everyone; the
    /// others also read a line for each level gained and for a new job.
    fn tell_guild_progress(&self, client_id: ClientId, previous: &Presence) {
        let Some(now) = self.clients.get(&client_id).map(|entry| &entry.presence) else {
            return;
        };
        let Some(guild_id) = now.guild_id.filter(|&id| previous.guild_id == Some(id)) else {
            return;
        };
        if (previous.level, previous.job) == (now.level, now.job) {
            return;
        }
        let built = |packets: Vec<Result<Packet, net::error::NetworkError>>| {
            packets
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .inspect_err(|error| warn!(client_id, %error, "Failed to build packet"))
                .unwrap_or_default()
        };
        let everyone = built(vec![build_member_level_job(
            guild_id, client_id, now.level, now.job,
        )]);
        let mut news: Vec<_> = (previous.level..now.level)
            .map(|level| build_guild_level_up(level + 1, &now.name))
            .collect();
        if previous.job != now.job {
            news.push(build_guild_job_change(now.job, &now.name));
        }
        let others = built(news);
        if let Some(alliance_id) = now
            .alliance_id
            .filter(|&id| previous.alliance_id == Some(id))
        {
            let packet = built(vec![alliance::build_member_level_job(
                alliance_id,
                guild_id,
                client_id,
                now.level,
                now.job,
            )]);
            for entry in self.alliance_online(alliance_id, guild_id) {
                for packet in &packet {
                    entry.handle.send_packet(packet.clone());
                }
            }
        }
        for (id, entry) in self.guild_online(guild_id) {
            let extra = if id == client_id {
                &[][..]
            } else {
                &others[..]
            };
            for packet in everyone.iter().chain(extra) {
                entry.handle.send_packet(packet.clone());
            }
        }
    }

    /// Pass on an invitation to someone on the inviter's channel who is not
    /// in a guild.
    fn guild_invite(&mut self, from: ClientId, guild_id: i32, name: &str, now: Instant) {
        let Some(inviter) = self.clients.get(&from) else {
            return;
        };
        let invitee = self.names.get(name).and_then(|id| self.clients.get(id));
        let Some(invitee) = invitee.filter(|invitee| {
            invitee.handle.client_id != from && invitee.channel_id == inviter.channel_id
        }) else {
            return self.send_packet(from, build_guild_error(GuildError::NotInChannel));
        };
        if invitee.presence.guild_id.is_some() {
            return self.send_packet(from, build_guild_error(GuildError::InviteeInGuild));
        }
        let (id, name) = (invitee.handle.client_id, invitee.presence.name.clone());
        let packet = build_guild_invite(guild_id, &inviter.presence.name);
        if self.guild_invites.invite(guild_id, from, id, now) {
            self.send_packet(id, packet);
        } else {
            let refused = build_invite_refused(InviteRefusal::AnotherInvitation, &name);
            self.send_packet(from, refused);
        }
    }

    /// Tell `inviter` that `from` turned their invitation down. One still
    /// shown, from the same guild, is kept: the client refuses a second.
    fn guild_deny(&mut self, from: ClientId, inviter: &str, refusal: InviteRefusal) {
        let (Some(&inviter), Some(entry)) = (self.names.get(inviter), self.clients.get(&from))
        else {
            return;
        };
        let pending = if refusal == InviteRefusal::AnotherInvitation {
            self.guild_invites.is_invited(from, inviter)
        } else {
            self.guild_invites.decline(from, inviter)
        };
        if pending {
            self.send_packet(inviter, build_invite_refused(refusal, &entry.presence.name));
        }
    }

    fn party_request(&mut self, from: ClientId, request: PartyRequest, now: Instant) {
        let Some(member) = self.clients.get(&from).map(Entry::member) else {
            return;
        };
        match request {
            PartyRequest::Create => self.create_party(member),
            PartyRequest::Invite { name } => self.invite(member, &name, now),
            PartyRequest::Accept { party_id } => {
                if let Some(party) = self.parties.accept(member.clone(), party_id, now).cloned() {
                    self.tell_party_changed(from, Some(party.id));
                    self.send_to_party(&party, |view| build_party_joined(view, &member.name));
                    self.exchange_hp(from);
                }
            }
            PartyRequest::Decline { inviter, status } => {
                let Some(&inviter) = self.names.get(&inviter) else {
                    return;
                };
                if status == PartyStatus::BusyWithInvite {
                    if self.parties.is_invited(from, inviter) {
                        self.send_status(inviter, status, &member.name);
                    }
                } else if self.parties.decline(from, inviter) {
                    self.send_status(inviter, status, &member.name);
                }
            }
            PartyRequest::Leave => match self.parties.leave(from) {
                Some(parties::Departure::Disbanded(party)) => {
                    for member in &party.members {
                        self.send_packet(member.id, build_party_disbanded(party.id, from));
                        self.tell_party_changed(member.id, None);
                    }
                }
                Some(parties::Departure::Left(party)) => {
                    self.send_departure(&party, &member, Departure::Left);
                }
                None => {}
            },
            PartyRequest::Expel { member_id } => {
                if let Some((party, name)) = self.parties.expel(from, member_id) {
                    let expelled = Member {
                        id: member_id,
                        name,
                        job: 0,
                        level: 0,
                    };
                    self.send_departure(&party, &expelled, Departure::Expelled);
                }
            }
            PartyRequest::ChangeLeader { member_id } => {
                if !self.clients.contains_key(&member_id) {
                    return;
                }
                if let Some(party) = self.parties.change_leader(from, member_id).cloned() {
                    // An event they're in follows; the first to tell it wins.
                    for who in [from, member_id] {
                        self.tell(who, ServerMessage::LeaderHandedOver { to: member_id });
                    }
                    self.send_to_party(&party, |_| build_party_leader_changed(member_id));
                    self.dungeon_leader_changed(from, member_id);
                }
            }
            PartyRequest::Chat { text } => {
                if let Some(party) = self.parties.of(from) {
                    for other in party.members.iter().filter(|m| m.id != from) {
                        self.send_packet(other.id, build_party_chat(&member.name, &text));
                    }
                }
            }
            PartyRequest::SearchStart {
                min_level,
                max_level,
                members,
                jobs,
            } => {
                let wanted = Wanted {
                    min_level,
                    max_level,
                    members,
                    jobs,
                };
                // v83 refuses all of these itself, so nothing answers them.
                let Some(party) = self.parties.of(from).filter(|party| party.leader == from) else {
                    return;
                };
                if !wanted.is_valid(member.level, party.members.len()) {
                    return;
                }
                let party_id = party.id;
                self.party_searches.start(from, party_id, wanted);
                self.party_search(from, party_id, wanted, now);
            }
            PartyRequest::SearchStop => self.party_searches.stop(from),
        }
    }

    /// One round of every party search: each invites one more character.
    /// The reference server's cadence, every ten seconds (b035).
    fn run_party_searches(&mut self, now: Instant) {
        for (leader, party_id, wanted) in self.party_searches.all() {
            self.party_search(leader, party_id, wanted, now);
        }
    }

    /// Invite the highest-level character on `leader`'s channel who fits
    /// the search, has no party, and hasn't been asked to join one, unless
    /// the party is gone, has a new leader, or is as big as wanted, which
    /// ends the search.
    fn party_search(&mut self, leader: ClientId, party_id: i32, wanted: Wanted, now: Instant) {
        let party = self
            .parties
            .of(leader)
            .filter(|party| party.id == party_id && party.leader == leader);
        let (Some(party), Some(entry)) = (party, self.clients.get(&leader)) else {
            return self.party_searches.stop(leader);
        };
        if party.members.len() >= wanted.members as usize {
            return self.party_searches.stop(leader);
        }
        let found = self
            .clients
            .values()
            .filter(|candidate| {
                let id = candidate.handle.client_id;
                candidate.channel_id == entry.channel_id
                    && id != leader
                    && self.parties.of(id).is_none()
                    && !self.parties.is_asked(id, now)
                    && !self.party_searches.has_invited(leader, id)
                    && wanted.fits(candidate.presence.level, candidate.presence.job)
            })
            .max_by_key(|candidate| {
                (
                    candidate.presence.level,
                    Reverse(candidate.handle.client_id),
                )
            })
            .map(|candidate| candidate.handle.client_id);
        let Some(found) = found else {
            return;
        };
        let name = entry.presence.name.clone();
        if self.parties.search_invite(leader, found, now).is_ok() {
            self.party_searches.invited(leader, found);
            self.send_packet(found, build_party_search_invite(party_id, &name));
        }
    }

    fn create_party(&mut self, leader: Member) {
        let id = leader.id;
        match self.parties.create(leader).cloned() {
            Ok(party) => {
                let door = self.doors.get(&id).map(|door| &door.view);
                self.send_packet(id, build_party_created(party.id, door));
                self.tell_party_changed(id, Some(party.id));
                self.send_party_update(&party);
            }
            Err(status) => self.send_status(id, status, ""),
        }
    }

    /// Invite someone online, creating a party for the inviter if needed.
    fn invite(&mut self, inviter: Member, name: &str, now: Instant) {
        let from = inviter.id;
        let invitee = match self.names.get(name) {
            Some(&invitee) if invitee != from => invitee,
            _ => return self.send_status(from, PartyStatus::NotFound, name),
        };
        if self.parties.of(from).is_none() {
            self.create_party(inviter.clone());
        }
        match self.parties.invite(from, invitee, now) {
            Ok(()) => {
                if let Some(party) = self.parties.of(from) {
                    self.send_packet(invitee, build_party_invite(party.id, &inviter.name));
                }
            }
            Err(status) => self.send_status(from, status, name),
        }
    }

    /// Tell the party, and whoever left it, that `who` is gone.
    fn send_departure(&self, party: &Party, who: &Member, how: Departure) {
        self.send_to_party(party, |view| build_party_left(view, who.id, &who.name, how));
        if let Some(entry) = self.clients.get(&who.id) {
            let view = self.party_view(party, entry.channel_id);
            self.send_packet(who.id, build_party_left(&view, who.id, &who.name, how));
        }
        self.tell_party_changed(who.id, None);
    }

    /// The party as a member on `channel_id` sees it: maps only show for
    /// members on the same channel.
    fn party_view(&self, party: &Party, channel_id: u8) -> PartyView {
        let members = party
            .members
            .iter()
            .map(|member| {
                let entry = self.clients.get(&member.id);
                PartyMemberView {
                    id: member.id,
                    name: member.name.clone(),
                    job: member.job,
                    level: member.level,
                    channel: entry.map(|e| e.channel_id),
                    map_id: entry
                        .filter(|e| e.channel_id == channel_id)
                        .map_or(0, |e| e.presence.map_id),
                    door: self
                        .doors
                        .get(&member.id)
                        .filter(|door| door.channel_id == channel_id)
                        .map(|door| door.view),
                }
            })
            .collect();
        PartyView {
            id: party.id,
            leader: party.leader,
            members,
        }
    }

    /// Send every online member a packet built from their view of the party.
    fn send_to_party(
        &self,
        party: &Party,
        build: impl Fn(&PartyView) -> Result<Packet, net::error::NetworkError>,
    ) {
        for member in &party.members {
            if let Some(entry) = self.clients.get(&member.id) {
                let view = self.party_view(party, entry.channel_id);
                self.send_packet(member.id, build(&view));
            }
        }
    }

    fn send_party_update(&self, party: &Party) {
        self.send_to_party(party, build_party_update);
    }

    /// Show `client`'s HP to party members on their map, and theirs to
    /// `client`.
    fn exchange_hp(&self, client: ClientId) {
        let Some(entry) = self.clients.get(&client) else {
            return;
        };
        for (other, other_entry) in self.party_neighbours(client, entry) {
            let presence = &other_entry.presence;
            self.send_packet(
                client,
                build_party_member_hp(other, presence.hp.into(), presence.max_hp.into()),
            );
        }
        self.send_hp_to_map(client);
    }

    fn send_hp_to_map(&self, client: ClientId) {
        let Some(entry) = self.clients.get(&client) else {
            return;
        };
        let (hp, max_hp) = (entry.presence.hp.into(), entry.presence.max_hp.into());
        for (other, _) in self.party_neighbours(client, entry) {
            self.send_packet(other, build_party_member_hp(client, hp, max_hp));
        }
    }

    /// Pass a party skill on to the members in its slots, the party's
    /// joining order, who are on the user's map; Echo of Hero and the GM's
    /// area skills, to everyone else on the map.
    fn party_skill(&self, from: ClientId, used: PartySkill, origin: (i16, i16)) {
        let Some(entry) = self.clients.get(&from) else {
            return;
        };
        let reached: Vec<ClientId> = if net::skills::reaches_everyone(used.skill_id) {
            self.clients.keys().copied().collect()
        } else if let Some(party) = self.parties.of(from) {
            let in_slots = party
                .members
                .iter()
                .enumerate()
                .filter(|(slot, _)| used.slots & net::skills::party_bit(*slot) != 0);
            in_slots.map(|(_, member)| member.id).collect()
        } else {
            return;
        };
        for id in reached.into_iter().filter(|&id| id != from) {
            let Some(other) = self.clients.get(&id) else {
                continue;
            };
            if other.shares_map_with(entry) {
                other.handle.send(ServerMessage::PartySkill {
                    used: used.clone(),
                    origin,
                    map_id: entry.presence.map_id,
                });
            }
        }
    }

    /// Pass a picked-up party potion on to the other party members on the
    /// picker's map.
    fn party_item(&self, from: ClientId, item_id: i32) {
        let (Some(entry), Some(party)) = (self.clients.get(&from), self.parties.of(from)) else {
            return;
        };
        for member in party.members.iter().filter(|member| member.id != from) {
            let Some(other) = self.clients.get(&member.id) else {
                continue;
            };
            if other.shares_map_with(entry) {
                other.handle.send(ServerMessage::PartyItem {
                    item_id,
                    map_id: entry.presence.map_id,
                });
            }
        }
    }

    fn door_opened(&mut self, owner: ClientId, view: DoorView) {
        let Some(entry) = self.clients.get(&owner) else {
            return;
        };
        let channel_id = entry.channel_id;
        self.doors.insert(owner, Door { channel_id, view });
        self.tell_door(owner, Some(&view));
    }

    /// Tell `owner`, and their party on their channel, of their door or its
    /// end: the owner keeps it for when they are alone (SPAWN_PORTAL), the
    /// party in the owner's slot.
    fn tell_door(&self, owner: ClientId, door: Option<&DoorView>) {
        let Some(entry) = self.clients.get(&owner) else {
            return;
        };
        self.send_packet(owner, build_spawn_portal(door));
        let Some(party) = self.parties.of(owner) else {
            return;
        };
        let Some(slot) = party.members.iter().position(|m| m.id == owner) else {
            return;
        };
        for member in &party.members {
            let on_channel = self
                .clients
                .get(&member.id)
                .is_some_and(|other| other.channel_id == entry.channel_id);
            if on_channel {
                self.send_packet(member.id, build_party_door(slot as u8, door));
            }
        }
    }

    /// `from` entered `owner`'s door: theirs or a party member's, on their
    /// channel, standing in its town or its field. Send them through, or
    /// tell them it is for the party only.
    fn use_door(&self, from: ClientId, owner: ClientId, from_town: bool) {
        let Some(entry) = self.clients.get(&from) else {
            return;
        };
        let party = self.parties.of(from);
        let slot = match party {
            Some(party) => party.members.iter().position(|m| m.id == owner),
            None => (owner == from).then_some(0),
        };
        let door = self
            .doors
            .get(&owner)
            .filter(|door| door.channel_id == entry.channel_id);
        let warp = slot.zip(door).and_then(|(slot, door)| {
            let door = &door.view;
            let map_id = entry.presence.map_id;
            if from_town && map_id == door.town {
                let exit = DoorExit::Field {
                    x: door.x,
                    y: door.y,
                };
                Some((door.field, exit))
            } else if !from_town && map_id == door.field {
                Some((door.town, DoorExit::Town { slot }))
            } else {
                None
            }
        });
        match warp {
            Some((map_id, exit)) => {
                entry.handle.send(ServerMessage::DoorWarp {
                    from_map: entry.presence.map_id,
                    map_id,
                    exit,
                });
            }
            None => self.send_packet(from, build_blocked_map(BlockedMap::PartyOnly)),
        }
    }

    /// Online party members on the same map as `client`.
    fn party_neighbours<'a>(
        &'a self,
        client: ClientId,
        entry: &'a Entry,
    ) -> impl Iterator<Item = (ClientId, &'a Entry)> + 'a {
        self.parties
            .of(client)
            .into_iter()
            .flat_map(|party| party.members.iter())
            .filter(move |member| member.id != client)
            .filter_map(|member| Some((member.id, self.clients.get(&member.id)?)))
            .filter(move |(_, other)| other.shares_map_with(entry))
    }

    /// The partner `presence` names, if they are online and name
    /// `client_id` back.
    fn partner_naming(&self, presence: &Presence, client_id: ClientId) -> Option<ClientId> {
        let partner = presence.partner?;
        let entry = self.clients.get(&partner)?;
        (entry.presence.partner == Some(client_id)).then_some(partner)
    }

    /// Tell an online couple each other's maps (report b057). The client
    /// marks the partner on its minimap and world map.
    fn exchange_partner_maps(&self, client_id: ClientId) {
        let Some(entry) = self.clients.get(&client_id) else {
            return;
        };
        let Some(partner) = self.partner_naming(&entry.presence, client_id) else {
            return;
        };
        let map_id = |id| self.clients.get(&id).map_or(0, |e| e.presence.map_id);
        self.tell(
            partner,
            ServerMessage::PartnerMap {
                map_id: map_id(client_id),
                partner: client_id,
            },
        );
        self.tell(
            client_id,
            ServerMessage::PartnerMap {
                map_id: map_id(partner),
                partner,
            },
        );
    }

    /// A couple formed or ended, or a partner moved map.
    fn tell_partner_progress(&self, client_id: ClientId, previous: &Presence) {
        let Some(entry) = self.clients.get(&client_id) else {
            return;
        };
        if entry.presence.partner != previous.partner {
            // An ended couple both forget the marks; the second to see it
            // no longer names the first, so neither is told twice.
            if let Some(old) = self.partner_naming(previous, client_id) {
                let none = || ServerMessage::PartnerMap {
                    map_id: 0,
                    partner: 0,
                };
                self.tell(old, none());
                self.tell(client_id, none());
            }
            self.exchange_partner_maps(client_id);
        } else if entry.presence.map_id != previous.map_id {
            if let Some(partner) = self.partner_naming(&entry.presence, client_id) {
                self.tell(
                    partner,
                    ServerMessage::PartnerMap {
                        map_id: entry.presence.map_id,
                        partner: client_id,
                    },
                );
            }
        }
    }

    /// Pass a message to `client`'s actor, if they are online.
    fn tell(&self, client: ClientId, message: ServerMessage) {
        if let Some(entry) = self.clients.get(&client) {
            entry.handle.send(message);
        }
    }

    fn tell_party_changed(&self, client: ClientId, party_id: Option<i32>) {
        if let Some(entry) = self.clients.get(&client) {
            entry.handle.send(ServerMessage::PartyChanged { party_id });
        }
    }

    fn send_status(&self, client: ClientId, status: PartyStatus, name: &str) {
        self.send_packet(client, build_party_status(status, name));
    }

    fn send_packet(&self, client: ClientId, packet: Result<Packet, net::error::NetworkError>) {
        let Some(entry) = self.clients.get(&client) else {
            return;
        };
        match packet {
            Ok(packet) => {
                entry.handle.send_packet(packet);
            }
            Err(error) => warn!(client, %error, "Failed to build packet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use net::packet::build::world::buddy::build_buddy_channel;
    use net::packet::op::SendOpcode;
    use tokio_util::sync::CancellationToken;

    fn handle(
        client_id: ClientId,
        connection_id: u64,
    ) -> (
        ClientHandle,
        mpsc::Receiver<ServerMessage>,
        CancellationToken,
    ) {
        let (tx, rx) = mpsc::channel(4);
        let kill = CancellationToken::new();
        (
            ClientHandle::new(client_id, connection_id, tx, kill.clone()),
            rx,
            kill,
        )
    }

    fn presence(name: &str, map_id: i32) -> Presence {
        Presence {
            name: name.into(),
            job: 0,
            level: 10,
            map_id,
            hp: 50,
            max_hp: 50,
            guild_id: None,
            alliance_id: None,
            family: Default::default(),
            partner: None,
        }
    }

    /// A registered client with room for every message the test sends.
    fn online(
        directory: &mut WorldDirectory,
        id: ClientId,
        name: &str,
        map_id: i32,
    ) -> mpsc::Receiver<ServerMessage> {
        let (tx, rx) = mpsc::channel(64);
        let handle = ClientHandle::new(id, id as u64, tx, CancellationToken::new());
        directory.register(handle, presence(name, map_id), 0);
        rx
    }

    /// A registered member of guild `guild_id`, or of none, on connection
    /// `connection`.
    fn member_online(
        directory: &mut WorldDirectory,
        id: ClientId,
        name: &str,
        guild_id: Option<i32>,
        connection: u64,
    ) -> mpsc::Receiver<ServerMessage> {
        let (tx, rx) = mpsc::channel(64);
        let handle = ClientHandle::new(id, connection, tx, CancellationToken::new());
        let mut presence = presence(name, 10000);
        presence.guild_id = guild_id;
        directory.register(handle, presence, 0);
        rx
    }

    /// The packets a client got.
    fn packets(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<Vec<u8>> {
        let mut seen = Vec::new();
        while let Ok(message) = rx.try_recv() {
            if let ServerMessage::SendPacket(packet) = message {
                seen.push(packet.bytes);
            }
        }
        seen
    }

    /// Guild 1, "Testers", with members (id, name, online) at level 10.
    fn testers(members: &[(ClientId, &str, bool)]) -> Guild {
        Guild {
            id: 1,
            name: "Testers".into(),
            titles: game::guild::DEFAULT_TITLES.map(String::from),
            capacity: 10,
            emblem: Default::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 0,
            members: members
                .iter()
                .map(|&(id, name, online)| game::GuildMember {
                    id,
                    name: name.into(),
                    job: 0,
                    level: 10,
                    rank: 3,
                    alliance_rank: 5,
                    online,
                })
                .collect(),
        }
    }

    /// What a client got: party changes and packet opcodes, with the
    /// PARTY_OPERATION sub-op.
    fn received(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<String> {
        let mut seen = Vec::new();
        while let Ok(message) = rx.try_recv() {
            seen.push(match message {
                ServerMessage::PartyChanged { party_id } => format!("party {party_id:?}"),
                ServerMessage::SendPacket(packet) => {
                    let opcode = packet.opcode();
                    if opcode == SendOpcode::PartyOperation as i16 {
                        format!("op {:#x}", packet.bytes[2])
                    } else {
                        format!("opcode {opcode:#x}")
                    }
                }
                other => format!("{other:?}"),
            });
        }
        seen
    }

    #[test]
    fn a_party_search_invites_one_fitting_character_a_round() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut seen = HashMap::new();
        for (id, name, job, level, channel) in [
            (1, "Alice", 0, 10, 0),
            (2, "Bobby", 0, 12, 0),
            (3, "Carol", 1000, 15, 0),
            (4, "Davey", 0, 30, 0),
            (5, "Erinn", 110, 14, 0),
            (6, "Frank", 0, 16, 1),
        ] {
            let (tx, rx) = mpsc::channel(64);
            let handle = ClientHandle::new(id, id as u64, tx, CancellationToken::new());
            let mut presence = presence(name, 10000);
            presence.job = job;
            presence.level = level;
            directory.register(handle, presence, channel);
            seen.insert(id, rx);
        }
        let now = Instant::now();
        let mut heard = |id| received(seen.get_mut(&id).unwrap());
        let beginners = PartyRequest::SearchStart {
            min_level: 5,
            max_level: 20,
            members: 3,
            jobs: 1 << 1,
        };

        // Only a party's leader searches, and only as v83 allows.
        directory.party_request(1, beginners.clone(), now);
        assert!(directory.party_searches.all().is_empty());
        directory.party_request(1, PartyRequest::Create, now);
        heard(1);
        let wide = PartyRequest::SearchStart {
            min_level: 5,
            max_level: 40,
            members: 3,
            jobs: 1 << 1,
        };
        directory.party_request(1, wide, now);
        assert!(directory.party_searches.all().is_empty());

        // Starting asks the highest level who fits at once, then one more
        // each round: not the warrior, the level 30, or another channel.
        directory.party_request(1, beginners.clone(), now);
        assert_eq!(heard(3), ["op 0x4"]);
        directory.run_party_searches(now);
        assert_eq!(heard(2), ["op 0x4"]);
        directory.run_party_searches(now);
        for id in [4, 5, 6] {
            assert!(heard(id).is_empty(), "{id} was asked");
        }

        // Once the party is as big as wanted, the search ends.
        directory.party_request(3, PartyRequest::Accept { party_id: 1 }, now);
        directory.party_request(2, PartyRequest::Accept { party_id: 1 }, now);
        assert_eq!(directory.parties.of(1).unwrap().members.len(), 3);
        directory.run_party_searches(now);
        assert!(directory.party_searches.all().is_empty());

        // 0xDF stops one.
        let more = PartyRequest::SearchStart {
            min_level: 5,
            max_level: 20,
            members: 6,
            jobs: 1 << 1,
        };
        directory.party_request(1, more, now);
        assert_eq!(directory.party_searches.all().len(), 1);
        directory.party_request(1, PartyRequest::SearchStop, now);
        assert!(directory.party_searches.all().is_empty());
    }

    #[test]
    fn members_in_the_cash_shop_cannot_go_into_an_event() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let lobby = events::HPQ.lobby;
        let _seen: Vec<_> = [(1, "Alice"), (2, "Bobby"), (3, "Carol")]
            .into_iter()
            .map(|(id, name)| online(&mut directory, id, name, lobby))
            .collect();
        let now = Instant::now();
        directory.party_request(1, PartyRequest::Create, now);
        for (id, name) in [(2, "Bobby"), (3, "Carol")] {
            let invite = PartyRequest::Invite { name: name.into() };
            directory.party_request(1, invite, now);
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        let entry = |directory: &WorldDirectory| directory.event_entry(1, EventKind::Hpq);
        assert_eq!(entry(&directory), (events::Entry::Ready, vec![1, 2, 3]));

        // Carol's map is still the lobby's, but she isn't on it.
        directory.handle(DirectoryMessage::EnteredCashShop { from: 3 });
        assert_eq!(entry(&directory).0, events::Entry::NotEligible);

        // Leaving the shop registers her again, on the map.
        let _carol = online(&mut directory, 3, "Carol", lobby);
        assert_eq!(entry(&directory).0, events::Entry::Ready);
    }

    #[test]
    fn a_party_skill_reaches_the_members_in_its_slots_on_the_map() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut bobby = online(&mut directory, 2, "Bobby", 10000);
        let mut carol = online(&mut directory, 3, "Carol", 20000);
        let mut dave = online(&mut directory, 4, "Dave", 10000);
        let now = Instant::now();
        for (id, name) in [(2, "Bobby"), (3, "Carol"), (4, "Dave")] {
            let invite = PartyRequest::Invite { name: name.into() };
            directory.party_request(1, invite, now);
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        for rx in [&mut alice, &mut bobby, &mut carol, &mut dave] {
            received(rx);
        }

        // Bobby, in slot 1, reached Alice (0), himself (1), and Carol (2) on
        // another map, but not Dave (3).
        let used = PartySkill {
            skill_id: 2301004,
            level: 20,
            slots: 0x38,
            heal: 0,
        };
        directory.party_skill(2, used.clone(), (5, 6));
        let reached = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut seen = Vec::new();
            while let Ok(message) = rx.try_recv() {
                if let ServerMessage::PartySkill {
                    used,
                    origin,
                    map_id,
                } = message
                {
                    seen.push((used, origin, map_id));
                }
            }
            seen
        };
        assert_eq!(reached(&mut alice), [(used, (5, 6), 10000)]);
        assert!(reached(&mut bobby).is_empty());
        assert!(reached(&mut carol).is_empty());
        assert!(reached(&mut dave).is_empty());
    }

    #[test]
    fn a_party_potion_reaches_the_other_members_on_the_map() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut bobby = online(&mut directory, 2, "Bobby", 10000);
        let mut carol = online(&mut directory, 3, "Carol", 20000);
        let mut erin = online(&mut directory, 5, "Erin", 10000);
        let now = Instant::now();
        for (id, name) in [(2, "Bobby"), (3, "Carol")] {
            let invite = PartyRequest::Invite { name: name.into() };
            directory.party_request(1, invite, now);
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        for rx in [&mut alice, &mut bobby, &mut carol, &mut erin] {
            received(rx);
        }
        let reached = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut seen = Vec::new();
            while let Ok(message) = rx.try_recv() {
                if let ServerMessage::PartyItem { item_id, map_id } = message {
                    seen.push((item_id, map_id));
                }
            }
            seen
        };

        // Bobby picked it up: Alice gets it, Carol on another map and Erin
        // outside the party don't.
        directory.party_item(2, 2022160);
        assert_eq!(reached(&mut alice), [(2022160, 10000)]);
        for rx in [&mut bobby, &mut carol, &mut erin] {
            assert!(reached(rx).is_empty());
        }

        // Without a party it is the picker's alone.
        directory.party_item(5, 2022160);
        for rx in [&mut alice, &mut bobby, &mut carol, &mut erin] {
            assert!(reached(rx).is_empty());
        }
    }

    #[test]
    fn echo_of_hero_reaches_everyone_on_the_map() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut bobby = online(&mut directory, 2, "Bobby", 10000);
        let mut carol = online(&mut directory, 3, "Carol", 20000);
        let used = PartySkill {
            skill_id: 1005,
            level: 1,
            slots: 0x3F,
            heal: 0,
        };
        directory.party_skill(1, used.clone(), (5, 6));
        let reached = |rx: &mut mpsc::Receiver<ServerMessage>| {
            std::iter::from_fn(|| rx.try_recv().ok())
                .filter(|message| matches!(message, ServerMessage::PartySkill { .. }))
                .count()
        };
        assert_eq!(reached(&mut alice), 0);
        assert_eq!(reached(&mut bobby), 1);
        assert_eq!(reached(&mut carol), 0);
    }

    #[test]
    fn invite_join_hp_chat_and_disband() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut bobby = online(&mut directory, 2, "Bobby", 10000);
        let mut carol = online(&mut directory, 3, "Carol", 20000);
        let now = Instant::now();
        let request = |directory: &mut WorldDirectory, from, request| {
            directory.party_request(from, request, now)
        };

        // Inviting without a party creates one first.
        request(
            &mut directory,
            1,
            PartyRequest::Invite {
                name: "Bobby".into(),
            },
        );
        assert_eq!(received(&mut alice), ["op 0x8", "party Some(1)", "op 0x7"]);
        assert_eq!(received(&mut bobby), ["op 0x4"]);
        request(
            &mut directory,
            1,
            PartyRequest::Invite {
                name: "Nobody".into(),
            },
        );
        assert_eq!(received(&mut alice), ["op 0x13"]);

        // Joining tells everyone and swaps HP with members on the same map.
        request(&mut directory, 2, PartyRequest::Accept { party_id: 1 });
        let hp = format!("opcode {:#x}", SendOpcode::UpdatePartyMemberHp as i16);
        assert_eq!(received(&mut alice), ["op 0xf".to_string(), hp.clone()]);
        assert_eq!(
            received(&mut bobby),
            ["party Some(1)".to_string(), "op 0xf".into(), hp.clone()]
        );

        // HP changes reach members on the map; a declined invite reaches
        // the inviter by name.
        let mut hurt = presence("Bobby", 10000);
        hurt.hp = 20;
        directory.update_presence(2, hurt);
        assert_eq!(received(&mut alice), [hp.clone()]);
        request(
            &mut directory,
            2,
            PartyRequest::Invite {
                name: "Carol".into(),
            },
        );
        assert_eq!(received(&mut carol), ["op 0x4"]);
        // Busy with another invitation: the inviter hears so, and the
        // invitation still waits.
        request(
            &mut directory,
            3,
            PartyRequest::Decline {
                inviter: "Bobby".into(),
                status: PartyStatus::BusyWithInvite,
            },
        );
        assert_eq!(received(&mut bobby), ["op 0x16"]);
        request(
            &mut directory,
            3,
            PartyRequest::Decline {
                inviter: "Bobby".into(),
                status: PartyStatus::Declined,
            },
        );
        assert_eq!(received(&mut bobby), ["op 0x17"]);

        let chat = format!("opcode {:#x}", SendOpcode::MultiChat as i16);
        request(&mut directory, 1, PartyRequest::Chat { text: "hi".into() });
        assert_eq!(received(&mut bobby), [chat]);
        assert!(received(&mut alice).is_empty());

        // Offline members stay in the party; the leader leaving ends it.
        directory.unregister(2, 2, false);
        assert_eq!(received(&mut alice), ["op 0x7"]);
        request(&mut directory, 1, PartyRequest::Leave);
        assert_eq!(received(&mut alice), ["op 0xc", "party None"]);
        let mut bobby = online(&mut directory, 2, "Bobby", 10000);
        assert!(received(&mut bobby).is_empty());
    }

    #[test]
    fn buddy_lists_show_channels_of_online_mutual_buddies_only() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let _bobby = online(&mut directory, 2, "Bobby", 10000);
        let _carol = online(&mut directory, 3, "Carol", 10000);
        let buddy = |id, name: &str, mutual| ListedBuddy {
            id,
            name: name.into(),
            group: "Default Group".into(),
            mutual,
            pending: false,
        };
        directory.handle(DirectoryMessage::BuddyList {
            owner: 1,
            buddies: vec![
                buddy(2, "Bobby", true),
                buddy(3, "Carol", false),
                buddy(4, "Dave", true),
            ],
        });
        let Ok(ServerMessage::SendPacket(packet)) = alice.try_recv() else {
            panic!("no buddy list");
        };
        let channels: Vec<i32> = (0..3)
            .map(|i| {
                let at = 4 + i * 39 + 18;
                i32::from_le_bytes(packet.bytes[at..at + 4].try_into().unwrap())
            })
            .collect();
        assert_eq!(channels, [0, -1, -1]);

        directory.handle(DirectoryMessage::Deliver {
            to: vec![1, 4],
            packet: build_buddy_channel(2, Some(0)).unwrap(),
        });
        let buddy_list = format!("opcode {:#x}", SendOpcode::BuddyList as i16);
        assert_eq!(received(&mut alice), [buddy_list]);
    }

    #[test]
    fn guild_info_marks_members_online_on_any_channel() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let (bobby, _bobby_rx, _) = handle(2, 2);
        directory.register(bobby, presence("Bobby", 20000), 2);
        let guild = |online: [bool; 3]| {
            testers(&[
                (1, "Alice", online[0]),
                (2, "Bobby", online[1]),
                (3, "Carol", online[2]),
            ])
        };
        directory.handle(DirectoryMessage::GuildInfo {
            owner: 1,
            guild: guild([false; 3]),
        });
        let Ok(ServerMessage::SendPacket(packet)) = alice.try_recv() else {
            panic!("no guild info");
        };
        let expected = build_guild_info(Some(&guild([true, true, false]))).unwrap();
        assert_eq!(packet.bytes, expected.bytes);
    }

    #[test]
    fn guild_members_hear_who_enters_and_leaves_the_game() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = member_online(&mut directory, 1, "Alice", Some(4), 1);
        let mut carol = member_online(&mut directory, 3, "Carol", Some(5), 3);
        let mut bobby = member_online(&mut directory, 2, "Bobby", Some(4), 2);
        let online = |online| vec![build_member_online(4, 2, online).unwrap().bytes];
        assert_eq!(packets(&mut alice), online(true));
        assert!(packets(&mut bobby).is_empty(), "not about themself");
        assert!(packets(&mut carol).is_empty(), "another guild");

        // A second login says nothing, and neither does the first
        // connection's late unregister.
        let _again = member_online(&mut directory, 2, "Bobby", Some(4), 12);
        directory.unregister(2, 2, false);
        assert!(packets(&mut alice).is_empty());

        directory.unregister(2, 12, false);
        assert_eq!(packets(&mut alice), online(false));
        assert!(packets(&mut carol).is_empty());
    }

    #[test]
    fn a_channel_change_is_silent_unless_it_takes_too_long() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = member_online(&mut directory, 1, "Alice", Some(1), 1);
        let _bobby = member_online(&mut directory, 2, "Bobby", Some(1), 2);
        packets(&mut alice);

        // On the way, the guild still shows them online.
        directory.unregister(2, 2, true);
        directory.purge_guild_away(Instant::now());
        assert!(packets(&mut alice).is_empty());
        directory.send_guild_info(1, testers(&[(1, "Alice", false), (2, "Bobby", false)]));
        let info = build_guild_info(Some(&testers(&[(1, "Alice", true), (2, "Bobby", true)])));
        assert_eq!(packets(&mut alice), [info.unwrap().bytes]);
        let _bobby = member_online(&mut directory, 2, "Bobby", Some(1), 22);
        assert!(packets(&mut alice).is_empty(), "back on another channel");

        // Not back in time: they logged out, and in again later.
        directory.unregister(2, 22, true);
        directory.purge_guild_away(Instant::now() + CHANNEL_CHANGE_GRACE);
        let online = |online| build_member_online(1, 2, online).unwrap().bytes;
        assert_eq!(packets(&mut alice), [online(false)]);
        let _bobby = member_online(&mut directory, 2, "Bobby", Some(1), 32);
        assert_eq!(packets(&mut alice), [online(true)]);
    }

    #[test]
    fn levels_and_jobs_reach_the_guild() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = member_online(&mut directory, 1, "Alice", Some(4), 1);
        let mut carol = member_online(&mut directory, 3, "Carol", None, 3);
        let mut bobby = member_online(&mut directory, 2, "Bobby", Some(4), 2);
        packets(&mut alice);
        let level_job = |level, job| build_member_level_job(4, 2, level, job).unwrap().bytes;
        let level_up = |level| build_guild_level_up(level, "Bobby").unwrap().bytes;

        // Bobby, in no party, gains two levels at once.
        let mut grown = presence("Bobby", 10000);
        grown.guild_id = Some(4);
        grown.level = 12;
        directory.update_presence(2, grown.clone());
        assert_eq!(packets(&mut bobby), [level_job(12, 0)]);
        assert_eq!(
            packets(&mut alice),
            [level_job(12, 0), level_up(11), level_up(12)]
        );
        assert!(packets(&mut carol).is_empty());

        grown.job = 100;
        directory.update_presence(2, grown.clone());
        assert_eq!(packets(&mut bobby), [level_job(12, 100)]);
        let job = build_guild_job_change(100, "Bobby").unwrap().bytes;
        assert_eq!(packets(&mut alice), [level_job(12, 100), job]);

        // HP and maps say nothing, nor does a level that comes with a change
        // of guild: leaving, joining or moving to another.
        grown.hp = 20;
        grown.map_id = 20000;
        directory.update_presence(2, grown.clone());
        for (guild_id, level) in [(None, 13), (Some(4), 14), (Some(5), 15)] {
            grown.guild_id = guild_id;
            grown.level = level;
            grown.job += 10;
            directory.update_presence(2, grown.clone());
        }
        assert!(packets(&mut alice).is_empty() && packets(&mut bobby).is_empty());
    }

    #[test]
    fn guild_invitations_stay_on_the_channel_and_answers_reach_the_inviter() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut bobby = online(&mut directory, 2, "Bobby", 20000);
        let (carol, _carol_rx, _) = handle(3, 3);
        directory.register(carol, presence("Carol", 10000), 1);
        let (dave, _dave_rx, _) = handle(4, 4);
        let mut guilded = presence("Dave", 10000);
        guilded.guild_id = Some(5);
        directory.register(dave, guilded, 0);
        let packets = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut seen = Vec::new();
            while let Ok(message) = rx.try_recv() {
                match message {
                    ServerMessage::SendPacket(packet) => seen.push(packet.bytes),
                    other => panic!("unexpected {other:?}"),
                }
            }
            seen
        };
        let invite = |directory: &mut WorldDirectory, name: &str| {
            directory.handle(DirectoryMessage::GuildInvite {
                from: 1,
                guild_id: 4,
                name: name.into(),
            })
        };
        let deny = |directory: &mut WorldDirectory, refusal| {
            directory.handle(DirectoryMessage::GuildDeny {
                from: 2,
                inviter: "Alice".into(),
                refusal,
            })
        };
        let error = |error| build_guild_error(error).unwrap().bytes;
        let refused = |refusal| build_invite_refused(refusal, "Bobby").unwrap().bytes;

        // Carol is on another channel, Dave in a guild.
        for name in ["Carol", "Nobody", "Alice"] {
            invite(&mut directory, name);
        }
        invite(&mut directory, "Dave");
        assert_eq!(
            packets(&mut alice),
            [
                error(GuildError::NotInChannel),
                error(GuildError::NotInChannel),
                error(GuildError::NotInChannel),
                error(GuildError::InviteeInGuild),
            ]
        );

        invite(&mut directory, "Bobby");
        assert_eq!(
            packets(&mut bobby),
            [build_guild_invite(4, "Alice").unwrap().bytes]
        );
        // Shown another invitation from the guild, the client refuses it and
        // keeps the first.
        deny(&mut directory, InviteRefusal::AnotherInvitation);
        assert_eq!(
            packets(&mut alice),
            [refused(InviteRefusal::AnotherInvitation)]
        );
        directory.handle(DirectoryMessage::GuildAccept {
            from: 2,
            guild_id: 4,
        });
        directory.handle(DirectoryMessage::GuildAccept {
            from: 2,
            guild_id: 4,
        });
        assert!(matches!(
            bobby.try_recv(),
            Ok(ServerMessage::JoinGuild { guild_id: 4 })
        ));
        assert!(bobby.try_recv().is_err());

        invite(&mut directory, "Bobby");
        packets(&mut bobby);
        deny(&mut directory, InviteRefusal::Declined);
        deny(&mut directory, InviteRefusal::Declined);
        assert_eq!(packets(&mut alice), [refused(InviteRefusal::Declined)]);
        directory.handle(DirectoryMessage::GuildAccept {
            from: 2,
            guild_id: 4,
        });
        assert!(bobby.try_recv().is_err());

        directory.handle(DirectoryMessage::GuildChanged { client_id: 2 });
        assert!(matches!(bobby.try_recv(), Ok(ServerMessage::GuildChanged)));
    }

    /// A registered member of guild `guild_id` in alliance `alliance_id`.
    fn allied_online(
        directory: &mut WorldDirectory,
        id: ClientId,
        name: &str,
        guild_id: i32,
        alliance_id: Option<i32>,
        connection: u64,
    ) -> mpsc::Receiver<ServerMessage> {
        let (tx, rx) = mpsc::channel(64);
        let handle = ClientHandle::new(id, connection, tx, CancellationToken::new());
        let mut presence = presence(name, 10000);
        presence.guild_id = Some(guild_id);
        presence.alliance_id = alliance_id;
        directory.register(handle, presence, 0);
        rx
    }

    #[test]
    fn the_alliances_other_guilds_hear_of_logins_levels_and_logouts() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = allied_online(&mut directory, 1, "Alice", 4, Some(7), 1);
        let mut carol = allied_online(&mut directory, 3, "Carol", 5, Some(7), 3);
        let mut erin = allied_online(&mut directory, 5, "Erin", 6, None, 5);
        packets(&mut alice);
        let mut bobby = allied_online(&mut directory, 2, "Bobby", 4, Some(7), 2);
        let guild_card = |online| build_member_online(4, 2, online).unwrap().bytes;
        let card = |online| {
            alliance::build_member_online(7, 4, 2, online)
                .unwrap()
                .bytes
        };
        assert_eq!(packets(&mut alice), [guild_card(true)], "as guild only");
        assert_eq!(packets(&mut carol), [card(true)]);
        assert!(packets(&mut erin).is_empty() && packets(&mut bobby).is_empty());

        let mut grown = presence("Bobby", 10000);
        grown.guild_id = Some(4);
        grown.alliance_id = Some(7);
        grown.level = 11;
        directory.update_presence(2, grown);
        let level_job = alliance::build_member_level_job(7, 4, 2, 11, 0)
            .unwrap()
            .bytes;
        assert_eq!(packets(&mut carol), [level_job]);
        assert!(packets(&mut erin).is_empty());

        // A channel change says nothing; leaving the game does.
        directory.unregister(2, 2, true);
        let _bobby = allied_online(&mut directory, 2, "Bobby", 4, Some(7), 12);
        assert!(packets(&mut carol).is_empty());
        directory.unregister(2, 12, false);
        assert_eq!(packets(&mut carol), [card(false)]);
        assert!(packets(&mut erin).is_empty());
    }

    #[test]
    fn alliance_invitations_reach_a_master_anywhere_and_answers_the_inviter() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", 10000);
        let mut dave = online(&mut directory, 4, "Dave", 10000);
        let (bobby, mut bobby_rx, _) = handle(2, 2);
        directory.register(bobby, presence("Bobby", 20000), 2);
        let pink = |key, guild: &str| {
            let text = net::content::text().get(key, &[("guild", guild)]);
            build_broadcast(Broadcast::Pink, &text).unwrap().bytes
        };
        let invite = |directory: &mut WorldDirectory, from, alliance_id, master| {
            directory.handle(DirectoryMessage::AllianceInvite {
                from,
                alliance_id,
                master,
                guild_name: "Rivals".into(),
            })
        };

        // Carol, the master, is offline.
        invite(&mut directory, 1, 7, 3);
        assert_eq!(
            packets(&mut alice),
            [pink(keys::ALLIANCE_OFFLINE, "Rivals")]
        );

        // Bobby is on another channel; another alliance must wait.
        invite(&mut directory, 1, 7, 2);
        assert_eq!(
            packets(&mut bobby_rx),
            [alliance::build_alliance_invite(7, "Alice").unwrap().bytes]
        );
        invite(&mut directory, 4, 8, 2);
        assert_eq!(packets(&mut dave), [pink(keys::ALLIANCE_BUSY, "Rivals")]);
        invite(&mut directory, 1, 7, 2);
        assert!(packets(&mut bobby_rx).is_empty(), "the card still shows");
        assert!(packets(&mut alice).is_empty());

        directory.handle(DirectoryMessage::AllianceDeny {
            from: 2,
            inviter: "Alice".into(),
            guild_name: "Rivals".into(),
        });
        assert_eq!(packets(&mut alice), [pink(keys::ALLIANCE_DENIED, "Rivals")]);
        directory.handle(DirectoryMessage::AllianceAccept {
            from: 2,
            alliance_id: 7,
        });
        assert!(bobby_rx.try_recv().is_err(), "declined already");

        invite(&mut directory, 1, 7, 2);
        packets(&mut bobby_rx);
        directory.handle(DirectoryMessage::AllianceAccept {
            from: 2,
            alliance_id: 7,
        });
        assert!(matches!(
            bobby_rx.try_recv(),
            Ok(ServerMessage::JoinAlliance { alliance_id: 7 })
        ));
    }

    #[test]
    fn party_here_lists_the_leader_then_members_on_their_map() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let _alice = online(&mut directory, 1, "Alice", 10000);
        let _bobby = online(&mut directory, 2, "Bobby", 10000);
        let _carol = online(&mut directory, 3, "Carol", 20000);
        let now = Instant::now();
        assert_eq!(directory.party_here(1), None, "in no party");
        for name in ["Bobby", "Carol"] {
            let name = name.into();
            directory.party_request(1, PartyRequest::Invite { name }, now);
        }
        for id in [2, 3] {
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        assert_eq!(directory.party_here(1), Some(vec![1, 2]));
        assert_eq!(directory.party_here(2), None, "not the leader");
    }

    /// The copies `rx` was sent to, and from where.
    fn ship_warps(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<(i32, i32)> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|message| match message {
                ServerMessage::ShipWarp {
                    from_map, map_id, ..
                } => Some((from_map, map_id)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_mini_dungeon_gives_each_team_a_copy_and_closes_when_its_leader_leaves() {
        const PIG_FARM: i32 = 100020000;
        const FIRST: i32 = 100020100;
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let _alice = online(&mut directory, 1, "Alice", PIG_FARM);
        let mut bobby = online(&mut directory, 2, "Bobby", PIG_FARM);
        let mut carol = online(&mut directory, 3, "Carol", 20000);
        let _dave = online(&mut directory, 4, "Dave", PIG_FARM);
        let now = Instant::now();
        for name in ["Bobby", "Carol"] {
            let name = name.into();
            directory.party_request(1, PartyRequest::Invite { name }, now);
        }
        for id in [2, 3] {
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        assert_eq!(
            directory.enter_dungeon(2, FIRST, 2, now),
            Err(Refusal::NotLeader)
        );
        assert_eq!(directory.enter_dungeon(1, FIRST, 2, now), Ok(FIRST));
        directory.gather_team(1, FIRST, 4);
        // The copy is kept while the team is on its way.
        assert_eq!(directory.enter_dungeon(4, FIRST, 2, now), Ok(FIRST + 1));
        assert_eq!(
            directory.enter_dungeon(4, FIRST, 2, now),
            Err(Refusal::Full)
        );
        // Only the member beside the leader follows, once the leader is in.
        assert_eq!(ship_warps(&mut bobby), []);
        directory.update_presence(1, presence("Alice", FIRST));
        assert_eq!(ship_warps(&mut bobby), [(PIG_FARM, FIRST)]);
        assert_eq!(ship_warps(&mut carol), []);
        directory.update_presence(2, presence("Bobby", FIRST));
        // A copy nobody came to is free after the grace; one in use isn't.
        let later = now + DUNGEON_GRACE;
        assert_eq!(directory.enter_dungeon(4, FIRST, 2, later), Ok(FIRST + 1));
        assert_eq!(
            directory.enter_dungeon(4, FIRST, 1, later),
            Err(Refusal::Full)
        );
        // A member leaving keeps it going; the leader leaving ends it.
        directory.update_presence(2, presence("Bobby", PIG_FARM));
        directory.update_presence(2, presence("Bobby", FIRST));
        assert!(directory.dungeons.contains_key(&(0, FIRST)));
        ship_warps(&mut bobby);
        directory.update_presence(1, presence("Alice", PIG_FARM));
        assert_eq!(ship_warps(&mut bobby), [(FIRST, PIG_FARM)]);
        assert!(!directory.dungeons.contains_key(&(0, FIRST)));

        // So does the leader logging out.
        directory.update_presence(2, presence("Bobby", PIG_FARM));
        assert_eq!(directory.enter_dungeon(1, FIRST, 1, later), Ok(FIRST));
        directory.update_presence(1, presence("Alice", FIRST));
        directory.update_presence(2, presence("Bobby", FIRST));
        ship_warps(&mut bobby);
        directory.unregister(1, 1, false);
        assert_eq!(ship_warps(&mut bobby), [(FIRST, PIG_FARM)]);

        // And the leader logging in again, whose old session's unregister
        // comes too late.
        let _alice = online(&mut directory, 1, "Alice", PIG_FARM);
        directory.update_presence(2, presence("Bobby", PIG_FARM));
        assert_eq!(directory.enter_dungeon(1, FIRST, 1, later), Ok(FIRST));
        directory.update_presence(1, presence("Alice", FIRST));
        directory.update_presence(2, presence("Bobby", FIRST));
        ship_warps(&mut bobby);
        let _alice = online(&mut directory, 1, "Alice", FIRST);
        assert_eq!(ship_warps(&mut bobby), [(FIRST, PIG_FARM)]);
    }

    #[test]
    fn a_mini_dungeon_follows_a_new_leader_in_it_and_closes_for_one_outside() {
        const PIG_FARM: i32 = 100020000;
        const FIRST: i32 = 100020100;
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let mut alice = online(&mut directory, 1, "Alice", PIG_FARM);
        let mut bobby = online(&mut directory, 2, "Bobby", PIG_FARM);
        let _carol = online(&mut directory, 3, "Carol", 20000);
        let now = Instant::now();
        for name in ["Bobby", "Carol"] {
            let name = name.into();
            directory.party_request(1, PartyRequest::Invite { name }, now);
        }
        for id in [2, 3] {
            directory.party_request(id, PartyRequest::Accept { party_id: 1 }, now);
        }
        assert_eq!(directory.enter_dungeon(1, FIRST, 1, now), Ok(FIRST));
        directory.gather_team(1, FIRST, 0);
        directory.update_presence(1, presence("Alice", FIRST));
        directory.update_presence(2, presence("Bobby", FIRST));
        ship_warps(&mut bobby);

        // Bobby, in the copy with her, takes it over: Alice may leave.
        directory.party_request(1, PartyRequest::ChangeLeader { member_id: 2 }, now);
        directory.update_presence(1, presence("Alice", PIG_FARM));
        assert_eq!(ship_warps(&mut bobby), []);
        assert!(directory.dungeons.contains_key(&(0, FIRST)));

        // Carol is elsewhere, so handing her the party ends the run.
        directory.update_presence(1, presence("Alice", FIRST));
        ship_warps(&mut alice);
        directory.party_request(2, PartyRequest::ChangeLeader { member_id: 3 }, now);
        assert_eq!(ship_warps(&mut alice), [(FIRST, PIG_FARM)]);
        assert_eq!(ship_warps(&mut bobby), [(FIRST, PIG_FARM)]);
        assert!(!directory.dungeons.contains_key(&(0, FIRST)));
    }

    #[test]
    fn duplicate_login_kicks_the_previous_connection_only() {
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let (first, _first_rx, first_kill) = handle(1, 10);
        let (second, _second_rx, second_kill) = handle(1, 11);

        directory.register(first, presence("dup", 10000), 0);
        directory.register(second, presence("dup", 10000), 0);
        assert!(first_kill.is_cancelled());
        assert!(!second_kill.is_cancelled());

        // The old connection's late unregister must not remove the new one.
        directory.unregister(1, 10, false);
        assert!(directory.names.contains_key("dup"));
        directory.unregister(1, 11, false);
        assert!(!directory.names.contains_key("dup"));
    }

    #[test]
    fn full_client_queue_kicks_instead_of_blocking() {
        let (client, _rx, kill) = handle(1, 1);
        for _ in 0..4 {
            assert!(client.send_packet(Packet::new_empty()));
        }
        assert!(!client.send_packet(Packet::new_empty()));
        assert!(kill.is_cancelled());
    }

    /// The PartnerMap messages a client got, as (map, partner).
    fn partner_maps(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<(i32, ClientId)> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|message| match message {
                ServerMessage::PartnerMap { map_id, partner } => Some((map_id, partner)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_couple_is_told_each_others_maps_while_both_are_online() {
        const HENESYS: i32 = 100000000;
        const PERION: i32 = 102000000;
        let (_tx, rx) = mpsc::channel(1);
        let mut directory = WorldDirectory::new(rx);
        let engaged = |name: &str, map_id, partner| Presence {
            partner: Some(partner),
            ..presence(name, map_id)
        };
        let mut alice = online(&mut directory, 1, "Alice", HENESYS);
        let mut bobby = online(&mut directory, 2, "Bobby", PERION);

        // Engaged: the first to know waits for the other.
        directory.update_presence(2, engaged("Bobby", PERION, 1));
        assert_eq!(partner_maps(&mut bobby), []);
        directory.update_presence(1, engaged("Alice", HENESYS, 2));
        assert_eq!(partner_maps(&mut alice), [(PERION, 2)]);
        assert_eq!(partner_maps(&mut bobby), [(HENESYS, 1)]);

        // A move tells only the other; HP doesn't.
        directory.update_presence(2, engaged("Bobby", HENESYS, 1));
        assert_eq!(partner_maps(&mut alice), [(HENESYS, 2)]);
        assert_eq!(partner_maps(&mut bobby), []);
        directory.update_presence(
            2,
            Presence {
                hp: 1,
                ..engaged("Bobby", HENESYS, 1)
            },
        );
        assert_eq!(partner_maps(&mut alice), []);

        // Logging off clears the mark; logging on exchanges again.
        directory.unregister(2, 2, false);
        assert_eq!(partner_maps(&mut alice), [(0, 0)]);
        let (tx, rx) = mpsc::channel(64);
        let mut bobby = rx;
        let handle = ClientHandle::new(2, 3, tx, CancellationToken::new());
        directory.register(handle, engaged("Bobby", PERION, 1), 1);
        assert_eq!(partner_maps(&mut alice), [(PERION, 2)]);
        assert_eq!(partner_maps(&mut bobby), [(HENESYS, 1)]);

        // Broken off: both forget, once each.
        directory.update_presence(1, presence("Alice", HENESYS));
        assert_eq!(partner_maps(&mut alice), [(0, 0)]);
        assert_eq!(partner_maps(&mut bobby), [(0, 0)]);
        directory.update_presence(2, presence("Bobby", PERION));
        assert_eq!(partner_maps(&mut alice), []);
        assert_eq!(partner_maps(&mut bobby), []);
    }
}
