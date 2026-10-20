use crate::actor::guild_invites::GuildInvites;
use crate::actor::messenger::{Messengers, Outbox};
use crate::actor::parties::{self, Member, Parties, Party};
use crate::message::{ClientHandle, DirectoryMessage, ListedBuddy, Presence, ServerMessage};
use game::Guild;
use net::packet::build::world::buddy::{build_buddy_list, BuddyView};
use net::packet::build::world::door::{build_spawn_portal, DoorView};
use net::packet::build::world::guild::{
    build_guild_error, build_guild_info, build_guild_invite, build_invite_refused, GuildError,
    InviteRefusal,
};
use net::packet::build::world::map::{build_blocked_map, BlockedMap};
use net::packet::build::world::messaging::{build_find_result, build_whisper_result, FindLocation};
use net::packet::build::world::party::{
    build_party_chat, build_party_created, build_party_disbanded, build_party_door,
    build_party_invite, build_party_joined, build_party_leader_changed, build_party_left,
    build_party_member_hp, build_party_status, build_party_update, Departure, PartyMemberView,
    PartyStatus, PartyView,
};
use net::packet::handle::world::{DoorExit, Rock};
use net::{BroadcastScope, ClientId, MessengerRequest, PartyRequest, PartySkill, WhisperDelivery};
use packet::Packet;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// How often seats held through a channel change are checked.
const PURGE_INTERVAL: Duration = Duration::from_secs(10);

struct Entry {
    handle: ClientHandle,
    presence: Presence,
    channel_id: u8,
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
    guild_invites: GuildInvites,
    messengers: Messengers,
    /// Each player's Mystic Door, which their party may use too.
    doors: HashMap<ClientId, Door>,
}

impl WorldDirectory {
    pub fn new(rx: mpsc::Receiver<DirectoryMessage>) -> Self {
        Self {
            rx,
            clients: HashMap::new(),
            names: HashMap::new(),
            parties: Parties::default(),
            guild_invites: GuildInvites::default(),
            messengers: Messengers::default(),
            doors: HashMap::new(),
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
                    let out = self.messengers.purge(Instant::now());
                    self.deliver(out);
                }
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
            DirectoryMessage::Broadcast {
                from,
                scope,
                packet,
            } => self.broadcast(from, scope, packet),
            DirectoryMessage::Party { from, request } => {
                self.party_request(from, request, Instant::now())
            }
            DirectoryMessage::PartySkill { from, used, origin } => {
                self.party_skill(from, used, origin)
            }
            DirectoryMessage::BuddyList { owner, buddies } => self.send_buddy_list(owner, buddies),
            DirectoryMessage::GuildInfo { owner, guild } => self.send_guild_info(owner, guild),
            DirectoryMessage::Online { client_id, reply } => {
                let _ = reply.send(self.clients.contains_key(&client_id));
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

    fn register(&mut self, handle: ClientHandle, presence: Presence, channel_id: u8) {
        let client_id = handle.client_id;
        if let Some(previous) = self.clients.remove(&client_id) {
            // The same character connected again: only one session may live.
            warn!(
                client_id,
                "Duplicate login; disconnecting previous connection"
            );
            previous.handle.kick();
            self.names.remove(&previous.presence.name);
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
                let out = self
                    .messengers
                    .disconnected(client_id, changing_channel, Instant::now());
                self.deliver(out);
            }
            self.parties.forget_invites(client_id);
            self.guild_invites.forget(client_id);
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

    fn send_guild_info(&self, owner: ClientId, mut guild: Guild) {
        for member in &mut guild.members {
            member.online = self.clients.contains_key(&member.id);
        }
        self.send_packet(owner, build_guild_info(Some(&guild)));
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
                    self.send_to_party(&party, |_| build_party_leader_changed(member_id));
                }
            }
            PartyRequest::Chat { text } => {
                if let Some(party) = self.parties.of(from) {
                    for other in party.members.iter().filter(|m| m.id != from) {
                        self.send_packet(other.id, build_party_chat(&member.name, &text));
                    }
                }
            }
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
    /// joining order, who are on the user's map.
    fn party_skill(&self, from: ClientId, used: PartySkill, origin: (i16, i16)) {
        let (Some(entry), Some(party)) = (self.clients.get(&from), self.parties.of(from)) else {
            return;
        };
        for (slot, member) in party.members.iter().enumerate() {
            if member.id == from || used.slots & net::skills::party_bit(slot) == 0 {
                continue;
            }
            let Some(other) = self.clients.get(&member.id) else {
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
        let member = |id, name: &str, online| game::GuildMember {
            id,
            name: name.into(),
            job: 0,
            level: 10,
            rank: 3,
            online,
        };
        let guild = |online: [bool; 3]| Guild {
            id: 1,
            name: "Testers".into(),
            titles: game::guild::DEFAULT_TITLES.map(String::from),
            capacity: 10,
            emblem: Default::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 0,
            members: vec![
                member(1, "Alice", online[0]),
                member(2, "Bobby", online[1]),
                member(3, "Carol", online[2]),
            ],
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
}
