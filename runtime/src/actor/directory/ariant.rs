//! The directory keeps the rosters of the Ariant Coliseum's arenas,
//! squads of `EventKind::Ariant` (`net::ariant`, Cosmic's silent
//! `ExpeditionType.ARIANT` expeditions): opened and joined from the
//! lobby, where Cesar sends each player to the arena's entrance, and run
//! from there by their host. Nobody hears of them but the players they
//! take.

use super::squad::{member_ids, SquadEntry};
use super::WorldDirectory;
use crate::actor::event::{EventQueue, EventStart};
use crate::message::{ClientHandle, ServerMessage, SquadAnswer};
use game_data::server_content::keys;
use net::ariant::{self, Arena, Host, Lobby, StartRefusal, ARIANT_LEVELS, ARIANT_LOBBY};
use net::conti::Portal;
use net::events::EventKind;
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::squads::{Squad, SquadCommand};
use net::ClientId;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::info;

impl WorldDirectory {
    /// What Cesar in the lobby shows `from` of their channel's arenas.
    pub(super) fn ariant_lobby(&mut self, from: ClientId) -> Lobby {
        let Some(channel_id) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return Lobby {
                arenas: Vec::new(),
                registered: true,
            };
        };
        let registered = self.ariant_registered(from);
        let mut arenas = Vec::new();
        for arena in 1..=ariant::ARENAS {
            let Some(entry) = self.live_squad(channel_id, EventKind::Ariant(arena)) else {
                arenas.push(Arena::Empty);
                continue;
            };
            let squad = &entry.squad;
            arenas.push(if squad.started().is_some() {
                Arena::Busy
            } else {
                Arena::Gathering {
                    names: names(squad),
                    view: squad.view(from),
                }
            });
        }
        Lobby { arenas, registered }
    }

    /// What Cesar at an arena's entrance finds for `from`, there.
    pub(super) fn ariant_host(&mut self, from: ClientId) -> Host {
        let Some((channel_id, map_id)) = self
            .clients
            .get(&from)
            .map(|entry| (entry.channel_id, entry.presence.map_id))
        else {
            return Host::None;
        };
        let Some(arena) = ariant::arena_at(map_id) else {
            return Host::None;
        };
        let Some(entry) = self.live_squad(channel_id, EventKind::Ariant(arena)) else {
            return Host::None;
        };
        let squad = &entry.squad;
        if squad.started().is_some() {
            return Host::None;
        }
        if squad.leader() != from {
            return Host::Member(names(squad));
        }
        let members = squad.members().to_vec();
        let start = self.ariant_team(channel_id, arena, from).map(|_| ());
        Host::Leading { members, start }
    }

    /// `from` tells arena `arena`'s roster on their channel `command`.
    pub(super) fn ariant_command(
        &mut self,
        from: ClientId,
        arena: u8,
        command: SquadCommand,
        reply: oneshot::Sender<SquadAnswer>,
    ) {
        let kind = EventKind::Ariant(arena);
        let answer = match command {
            SquadCommand::OpenFor(cap) => self.open_arena(from, arena, cap),
            SquadCommand::Join => self.join_arena(from, arena),
            SquadCommand::Expel(who) => self.ban_from_arena(from, arena, who),
            SquadCommand::Stop => self.leave_arena(from, arena),
            SquadCommand::Start => self.start_arena(from, arena),
            // Cosmic's arenas have no way back in.
            SquadCommand::Open | SquadCommand::Rejoin => None,
        };
        let answer = answer.unwrap_or(SquadAnswer::Refused);
        // A host who can't take the reply can't start it.
        if let Err(SquadAnswer::Start(start)) = reply.send(answer) {
            self.event_gone(start.channel_id, kind, start.id);
            self.squads.remove(&(start.channel_id, kind));
        }
    }

    /// Where `from` is and their name, if online in the Coliseum's lobby,
    /// on no arena's roster on any channel.
    fn ariant_newcomer(&mut self, from: ClientId) -> Option<(u8, String)> {
        let entry = self.clients.get(&from)?;
        let presence = &entry.presence;
        if presence.map_id != ARIANT_LOBBY || entry.in_cash_shop {
            return None;
        }
        let (channel_id, name) = (entry.channel_id, presence.name.clone());
        (!self.ariant_registered(from)).then_some((channel_id, name))
    }

    /// Whether `from` is on an arena's roster on any channel.
    fn ariant_registered(&self, from: ClientId) -> bool {
        let now = Instant::now();
        self.squads.iter().any(|((_, kind), entry)| {
            matches!(kind, EventKind::Ariant(_))
                && !entry.squad.lapsed(now)
                && entry.squad.has(from)
        })
    }

    /// Whether `who` is online on channel `channel_id`.
    fn on_channel(&self, who: ClientId, channel_id: u8) -> bool {
        (self.clients.get(&who)).is_some_and(|entry| entry.channel_id == channel_id)
    }

    /// `from` opens arena `arena` for at most `cap`, if nobody holds it:
    /// their clock counts down the time it has to start.
    fn open_arena(&mut self, from: ClientId, arena: u8, cap: usize) -> Option<SquadAnswer> {
        let kind = EventKind::Ariant(arena);
        let (channel_id, name) = self.ariant_newcomer(from)?;
        if self.live_squad(channel_id, kind).is_some() {
            return None;
        }
        let now = Instant::now();
        let squad = Squad::open(kind, from, name, now).with_cap(cap);
        let left = squad.closes_in(now);
        self.squads
            .insert((channel_id, kind), SquadEntry { squad, event: None });
        info!(arena, channel_id, host = from, cap, "Ariant arena opened");
        self.send_packet(from, build_countdown(left.as_secs() as u32));
        Some(SquadAnswer::Done)
    }

    /// `from` joins arena `arena`'s roster, if they may: their clock counts
    /// down what time is left.
    fn join_arena(&mut self, from: ClientId, arena: u8) -> Option<SquadAnswer> {
        let (channel_id, name) = self.ariant_newcomer(from)?;
        let entry = self.live_squad(channel_id, EventKind::Ariant(arena))?;
        if !entry.squad.join(from, name) {
            return None;
        }
        let left = entry.squad.closes_in(Instant::now());
        info!(arena, channel_id, member = from, "Ariant arena joined");
        self.send_packet(from, build_countdown(left.as_secs() as u32));
        Some(SquadAnswer::Done)
    }

    /// Where `from` is, if online at arena `arena`'s entrance.
    fn at_entrance(&self, from: ClientId, arena: u8) -> Option<u8> {
        let entry = self.clients.get(&from)?;
        (entry.presence.map_id == ariant::entrance(arena) && !entry.in_cash_shop)
            .then_some(entry.channel_id)
    }

    /// The host, `from`, bans `who` from arena `arena`: they go back to
    /// the lobby, if at its entrance, and may not come back.
    fn ban_from_arena(&mut self, from: ClientId, arena: u8, who: ClientId) -> Option<SquadAnswer> {
        let channel_id = self.at_entrance(from, arena)?;
        let entry = self.live_squad(channel_id, EventKind::Ariant(arena))?;
        entry.squad.expel(from, who)?;
        info!(
            arena,
            channel_id,
            member = who,
            "Banned from an Ariant arena"
        );
        if !self.on_channel(who, channel_id) {
            return Some(SquadAnswer::Done);
        }
        self.send_packet(who, build_stop_clock());
        self.to_lobby(who, arena);
        Some(SquadAnswer::Done)
    }

    /// The host, `from`, leaves arena `arena` before its battle: those at
    /// its entrance hear of it and go back to the lobby, and its roster is
    /// no more.
    fn leave_arena(&mut self, from: ClientId, arena: u8) -> Option<SquadAnswer> {
        let kind = EventKind::Ariant(arena);
        let channel_id = self.at_entrance(from, arena)?;
        let entry = self.live_squad(channel_id, kind)?;
        if entry.squad.leader() != from || entry.squad.started().is_some() {
            return None;
        }
        let members = member_ids(&entry.squad);
        self.squads.remove(&(channel_id, kind));
        info!(
            arena,
            channel_id,
            host = from,
            "Ariant arena left by its host"
        );
        let entrance = ariant::entrance(arena);
        let text = net::content::text().get(keys::ARIANT_HOST_LEFT, &[]);
        let there: Vec<_> = (self.clients.values())
            .filter(|entry| entry.channel_id == channel_id && entry.presence.map_id == entrance)
            .map(|entry| entry.handle.client_id)
            .collect();
        for client in there {
            self.send_packet(client, build_broadcast(Broadcast::Pink, &text));
        }
        for member in members {
            if !self.on_channel(member, channel_id) {
                continue;
            }
            self.send_packet(member, build_stop_clock());
            self.to_lobby(member, arena);
        }
        Some(SquadAnswer::Done)
    }

    /// `who` goes from arena `arena`'s entrance back to the lobby, if
    /// there.
    fn to_lobby(&self, who: ClientId, arena: u8) {
        let message = ServerMessage::ShipWarp {
            from_map: ariant::entrance(arena),
            map_id: ARIANT_LOBBY,
            portal: Portal::Id(0),
        };
        self.tell(who, message);
    }

    /// Those on arena `arena`'s roster who would go in if its host, `from`,
    /// took it in now: all of them online on the channel, or why not
    /// (Cosmic `startAriantBattle`).
    fn ariant_team(
        &self,
        channel_id: u8,
        arena: u8,
        from: ClientId,
    ) -> Result<Vec<ClientHandle>, StartRefusal> {
        let kind = EventKind::Ariant(arena);
        let squad = match self.squads.get(&(channel_id, kind)) {
            Some(entry) if entry.squad.started().is_none() => &entry.squad,
            _ => return Err(StartRefusal::Busy),
        };
        if self.parties.of(from).is_some() {
            return Err(StartRefusal::HostInParty);
        }
        let online: Vec<_> = (squad.members().iter())
            .filter_map(|(id, _)| self.clients.get(id))
            .filter(|entry| entry.channel_id == channel_id)
            .collect();
        let (least, most) = (kind.event().least(), squad.cap());
        if !(least..=most).contains(&online.len()) {
            return Err(StartRefusal::Count { least, most });
        }
        for entry in &online {
            if entry.presence.map_id != ariant::entrance(arena) || entry.in_cash_shop {
                return Err(StartRefusal::Away);
            }
            if self.parties.of(entry.handle.client_id).is_some() {
                return Err(StartRefusal::InParty);
            }
            if !ARIANT_LEVELS.contains(&entry.presence.level) {
                return Err(StartRefusal::Levels);
            }
        }
        if self.event_busy(channel_id, kind) {
            return Err(StartRefusal::Busy);
        }
        Ok(online.iter().map(|entry| entry.handle.clone()).collect())
    }

    /// The host, `from`, takes arena `arena`'s players into its battle.
    fn start_arena(&mut self, from: ClientId, arena: u8) -> Option<SquadAnswer> {
        let kind = EventKind::Ariant(arena);
        let channel_id = self.at_entrance(from, arena)?;
        let entry = self.live_squad(channel_id, kind)?;
        if entry.squad.leader() != from {
            return None;
        }
        let team = self.ariant_team(channel_id, arena, from).ok()?;
        let entry = self.squads.get_mut(&(channel_id, kind))?;
        let roster = member_ids(&entry.squad);
        let id = self.next_event;
        self.next_event = self.next_event.wrapping_add(1).max(1);
        let ids: Vec<_> = team.iter().map(|member| member.client_id).collect();
        let queue = EventQueue::default();
        entry.squad.start(id);
        entry.event = Some(queue.joiner());
        self.event_opened(channel_id, kind, id);
        info!(arena, id, channel_id, host = from, team = ?ids, "Ariant battle began");
        for &member in &roster {
            if self.on_channel(member, channel_id) {
                self.send_packet(member, build_stop_clock());
            }
        }
        Some(SquadAnswer::Start(EventStart {
            id,
            kind,
            channel_id,
            leader: from,
            team,
            blessing: None,
            queue,
            roster: ids,
            couples: false,
        }))
    }
}

/// The names on `squad`'s roster, its host first.
fn names(squad: &Squad) -> Vec<String> {
    squad
        .members()
        .iter()
        .map(|(_, name)| name.clone())
        .collect()
}
