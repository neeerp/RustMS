//! The directory keeps each channel's Monster Carnival rooms
//! (`net::carnival`, Cosmic `MonsterCarnival`): the first party to pick a
//! free room takes it and waits there, and the next party's leader to pick
//! it challenges them. The waiting leader's yes brings the challengers into
//! the room's event. A challenge not answered in time lapses, found so the
//! next time anyone picks the room, and a leader has one out at a time.

use super::WorldDirectory;
use crate::actor::event::EventJoiner;
use crate::message::{CarnivalPick, ServerMessage};
use game_data::server_content::keys;
use net::carnival::{room_map, Challenger, Refusal, Room, ALL_ROOMS, CHALLENGE};
use net::events::{Entry, EventKind};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::ClientId;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::info;

/// A room taken by a party, and the way into its event.
pub(super) struct CarnivalRoom {
    id: u32,
    /// Who leads the party waiting there, who answers challenges.
    leader: ClientId,
    /// The waiting party's average level.
    level: u8,
    event: EventJoiner,
    /// The leader of the party whose challenge waits for an answer, and
    /// when it was put.
    challenge: Option<(ClientId, Instant)>,
    /// A challenge was accepted: the room is in battle, or about to be.
    battle: bool,
}

impl CarnivalRoom {
    /// The challenge waiting for an answer, unless it lapsed.
    fn challenge(&self, now: Instant) -> Option<ClientId> {
        self.challenge
            .filter(|&(_, at)| now.duration_since(at) < CHALLENGE)
            .map(|(leader, _)| leader)
    }
}

impl WorldDirectory {
    /// What Spiegelmann shows `from` of their channel's rooms, 1 to 9:
    /// the Carnival Field's and Carnival Field 2's.
    pub(super) fn carnival_rooms(&self, from: ClientId) -> Vec<Room> {
        let Some(channel_id) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return Vec::new();
        };
        ALL_ROOMS
            .map(|room| {
                let key = (channel_id, room);
                let Some(taken) = self.carnival_rooms.get(&key) else {
                    return Room::Free;
                };
                match self.waiting_size(taken, key) {
                    0 => Room::Battle,
                    _ if taken.battle => Room::Battle,
                    size => Room::Waiting {
                        size,
                        level: taken.level,
                    },
                }
            })
            .collect()
    }

    /// How many of the party waiting in room `key` are there now: those
    /// who left it, or the game, don't count.
    fn waiting_size(&self, taken: &CarnivalRoom, (channel_id, room): (u8, u8)) -> usize {
        let there = |id: &ClientId| {
            self.clients.get(id).is_some_and(|entry| {
                entry.channel_id == channel_id
                    && !entry.in_cash_shop
                    && entry.presence.map_id == room_map(room)
            })
        };
        match self.parties.of(taken.leader) {
            Some(party) => party.members.iter().filter(|m| there(&m.id)).count(),
            None => usize::from(there(&taken.leader)),
        }
    }

    /// `from`, a party's leader, picks room `room`: a free one their party
    /// takes, and one where a party waits they challenge, if their party is
    /// as big.
    pub(super) fn carnival_pick(
        &mut self,
        from: ClientId,
        room: u8,
        reply: oneshot::Sender<CarnivalPick>,
    ) {
        let answer = self.carnival_choice(from, room);
        // A leader who can't take the reply can't start it.
        if let Err(CarnivalPick::Start(start)) = reply.send(answer) {
            self.event_gone(start.channel_id, start.kind, start.id);
            self.carnival_rooms.remove(&(start.channel_id, room));
        }
    }

    fn carnival_choice(&mut self, from: ClientId, room: u8) -> CarnivalPick {
        let Some(channel_id) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return CarnivalPick::Refused(Refusal::NoParty);
        };
        if !ALL_ROOMS.contains(&room) {
            return CarnivalPick::Refused(Refusal::Full);
        }
        let kind = EventKind::Carnival(room);
        let key = (channel_id, room);
        let now = Instant::now();
        // A new pick takes back the leader's challenge of another room.
        for taken in self.carnival_rooms.values_mut() {
            if taken
                .challenge
                .is_some_and(|(challenger, _)| challenger == from)
            {
                taken.challenge = None;
            }
        }
        let Some(taken) = self.carnival_rooms.get(&key) else {
            return self.take_carnival_room(from, kind, key);
        };
        if taken.battle {
            return CarnivalPick::Refused(Refusal::Full);
        }
        if taken.challenge(now).is_some() {
            return CarnivalPick::Refused(Refusal::Facing);
        }
        let (size, leader) = (self.waiting_size(taken, key), taken.leader);
        if size == 0 {
            return CarnivalPick::Refused(Refusal::Full);
        }
        let team = match self.event_team(from, kind) {
            Ok(team) => team,
            Err(entry) => return CarnivalPick::Refused(refusal(entry)),
        };
        if team.len() != size {
            return CarnivalPick::Refused(Refusal::Unequal);
        }
        let challengers = team
            .iter()
            .filter_map(|member| self.clients.get(member))
            .map(|entry| Challenger {
                name: entry.presence.name.clone(),
                level: entry.presence.level,
            })
            .collect();
        let Some(waiting) = self.clients.get(&leader) else {
            return CarnivalPick::Refused(Refusal::Facing);
        };
        waiting.handle.send(ServerMessage::CarnivalChallenge {
            challenger: from,
            challengers,
        });
        if let Some(taken) = self.carnival_rooms.get_mut(&key) {
            taken.challenge = Some((from, now));
        }
        info!(
            room,
            channel_id,
            challenger = from,
            leader,
            "Carnival challenge"
        );
        CarnivalPick::Sent
    }

    /// `from`'s party takes room `kind` on its channel, to wait there.
    fn take_carnival_room(
        &mut self,
        from: ClientId,
        kind: EventKind,
        key: (u8, u8),
    ) -> CarnivalPick {
        let (entry, _) = self.event_entry(from, kind);
        let Some(start) = self.start_event(from, kind) else {
            return CarnivalPick::Refused(refusal(entry));
        };
        let levels: Vec<u32> = start
            .team
            .iter()
            .filter_map(|member| self.clients.get(&member.client_id))
            .map(|entry| u32::from(entry.presence.level))
            .collect();
        let level = levels.iter().sum::<u32>() / levels.len().max(1) as u32;
        self.carnival_rooms.insert(
            key,
            CarnivalRoom {
                id: start.id,
                leader: from,
                level: u8::try_from(level).unwrap_or(u8::MAX),
                event: start.queue.joiner(),
                challenge: None,
                battle: false,
            },
        );
        CarnivalPick::Start(start)
    }

    /// `from`, leading a party waiting in a room, answers the challenge
    /// `challenger` put to them, if it still stands. Yes brings the
    /// challengers in, if both parties are still as big; no tells their
    /// leader.
    pub(super) fn carnival_answer(&mut self, from: ClientId, challenger: ClientId, accept: bool) {
        let now = Instant::now();
        let Some(&key) = self.carnival_rooms.iter().find_map(|(key, taken)| {
            let put = taken.leader == from && taken.challenge(now) == Some(challenger);
            put.then_some(key)
        }) else {
            return;
        };
        let text = net::content::text();
        let Some(taken) = self.carnival_rooms.get(&key) else {
            return;
        };
        let (size, joiner) = (self.waiting_size(taken, key), taken.event.clone());
        if let Some(taken) = self.carnival_rooms.get_mut(&key) {
            taken.challenge = None;
        }
        if !accept {
            let declined = text.get(keys::CARNIVAL_CHALLENGE_DECLINED, &[]);
            self.send_packet(challenger, build_broadcast(Broadcast::Pink, &declined));
            return;
        }
        let team = self
            .event_team(challenger, EventKind::Carnival(key.1))
            .unwrap_or_default();
        if team.len() != size {
            let unequal = text.get(keys::CARNIVAL_CHALLENGE_UNEQUAL, &[]);
            self.send_packet(from, build_broadcast(Broadcast::Pink, &unequal));
            return;
        }
        let members = team
            .iter()
            .filter_map(|member| self.clients.get(member))
            .map(|entry| entry.handle.clone())
            .collect();
        if joiner.challengers(members, challenger) {
            info!(
                room = key.1,
                channel_id = key.0,
                challenger,
                leader = from,
                "Carnival accepted"
            );
            if let Some(taken) = self.carnival_rooms.get_mut(&key) {
                taken.battle = true;
            }
        }
    }

    /// Event `id` of `kind` on `channel_id` is over: its room is free.
    pub(super) fn carnival_closed(&mut self, channel_id: u8, kind: EventKind, id: u32) {
        let EventKind::Carnival(room) = kind else {
            return;
        };
        let key = (channel_id, room);
        if self
            .carnival_rooms
            .get(&key)
            .is_some_and(|taken| taken.id == id)
        {
            self.carnival_rooms.remove(&key);
        }
    }
}

/// Why a party may not take or challenge a room, as Spiegelmann says it.
fn refusal(entry: Entry) -> Refusal {
    match entry {
        Entry::NoParty => Refusal::NoParty,
        Entry::NotLeader => Refusal::NotLeader,
        Entry::NotEligible => Refusal::NotEligible,
        Entry::Busy | Entry::Ready => Refusal::Full,
    }
}
