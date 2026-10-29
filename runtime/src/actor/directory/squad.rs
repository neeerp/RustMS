//! The directory keeps each channel's squads (`net::squads`, Cosmic
//! `Expedition`): gathered at their NPC whatever the parties, they set out
//! into an event of their own (`Event::squad`), which those on the roster
//! may come back into while it runs. A squad that hasn't set out lapses
//! quietly when its time is up, found so the next time anyone asks.

use super::WorldDirectory;
use crate::actor::event::{EventJoiner, EventQueue, EventStart};
use crate::message::SquadAnswer;
use game_data::server_content::keys;
use net::events::EventKind;
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::build::world::messaging::build_item_line;
use net::squads::{Squad, SquadCommand, SquadView};
use net::ClientId;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::info;

/// A squad, and once it set out, the way back into its event.
pub(super) struct SquadEntry {
    pub(super) squad: Squad,
    pub(super) event: Option<EventJoiner>,
}

impl WorldDirectory {
    /// The squad of `kind` on `channel_id`, unless it lapsed, in which case
    /// it goes.
    pub(super) fn live_squad(
        &mut self,
        channel_id: u8,
        kind: EventKind,
    ) -> Option<&mut SquadEntry> {
        let key = (channel_id, kind);
        if self
            .squads
            .get(&key)
            .is_some_and(|entry| entry.squad.lapsed(Instant::now()))
        {
            info!(?kind, channel_id, "Squad lapsed");
            self.squads.remove(&key);
        }
        self.squads.get_mut(&key)
    }

    /// What `from` finds of the squad of `kind` on their channel.
    pub(super) fn squad_view(&mut self, from: ClientId, kind: EventKind) -> SquadView {
        let Some(channel_id) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return SquadView::Taken;
        };
        self.live_squad(channel_id, kind)
            .map_or(SquadView::None, |entry| entry.squad.view(from))
    }

    /// `from` tells the squad of `kind` on their channel `command`. A
    /// member coming back is answered by its event.
    pub(super) fn squad_command(
        &mut self,
        from: ClientId,
        kind: EventKind,
        command: SquadCommand,
        reply: oneshot::Sender<SquadAnswer>,
    ) {
        if let EventKind::Ariant(arena) = kind {
            return self.ariant_command(from, arena, command, reply);
        }
        let answer = match command {
            SquadCommand::Rejoin => {
                if let Some(reply) = self.rejoin_squad(from, kind, reply) {
                    let _ = reply.send(SquadAnswer::Refused);
                }
                return;
            }
            SquadCommand::Open | SquadCommand::OpenFor(_) => self.open_squad(from, kind),
            SquadCommand::Join => self.join_squad(from, kind),
            SquadCommand::Expel(who) => self.expel_from_squad(from, kind, who),
            SquadCommand::Stop => self.stop_squad(from, kind),
            SquadCommand::Start => self.start_squad(from, kind),
        };
        let answer = answer.unwrap_or(SquadAnswer::Refused);
        // A leader who can't take the reply can't start it.
        if let Err(SquadAnswer::Start(start)) = reply.send(answer) {
            self.event_gone(start.channel_id, kind, start.id);
            self.squads.remove(&(start.channel_id, kind));
        }
    }

    /// Where `from` is and their name, if online on the lobby of `kind`,
    /// where its NPC stands.
    pub(super) fn squad_asker(&self, from: ClientId, kind: EventKind) -> Option<(u8, i32, String)> {
        let entry = self.clients.get(&from)?;
        let presence = &entry.presence;
        (presence.map_id == kind.event().lobby && !entry.in_cash_shop)
            .then(|| (entry.channel_id, presence.map_id, presence.name.clone()))
    }

    /// `from` gathers a squad of `kind` on their channel, if there is none:
    /// those on their map hear of it, and their clock counts down the time
    /// it has.
    fn open_squad(&mut self, from: ClientId, kind: EventKind) -> Option<SquadAnswer> {
        let says = net::squads::text(kind)?;
        let (channel_id, map_id, name) = self.squad_asker(from, kind)?;
        if self.live_squad(channel_id, kind).is_some() {
            return None;
        }
        let now = Instant::now();
        let squad = Squad::open(kind, from, name.clone(), now);
        let left = squad.closes_in(now);
        self.squads
            .insert((channel_id, kind), SquadEntry { squad, event: None });
        info!(?kind, channel_id, leader = from, "Squad opened");
        let beside: Vec<_> = self
            .clients
            .values()
            .filter(|entry| entry.channel_id == channel_id && entry.presence.map_id == map_id)
            .map(|entry| entry.handle.client_id)
            .collect();
        self.tell_squad(&beside, says.notice_opened, &name);
        self.send_packet(from, build_countdown(left.as_secs() as u32));
        Some(SquadAnswer::Done)
    }

    /// `from` joins the squad of `kind` on their channel: its members hear
    /// of it, and their clock counts down what time is left.
    fn join_squad(&mut self, from: ClientId, kind: EventKind) -> Option<SquadAnswer> {
        let (channel_id, _, name) = self.squad_asker(from, kind)?;
        let entry = self.live_squad(channel_id, kind)?;
        if !entry.squad.join(from, name.clone()) {
            return None;
        }
        let left = entry.squad.closes_in(Instant::now());
        let members = member_ids(&entry.squad);
        info!(?kind, channel_id, member = from, "Squad joined");
        self.tell_squad(&members, keys::SQUAD_NOTICE_JOINED, &name);
        self.send_packet(from, build_countdown(left.as_secs() as u32));
        Some(SquadAnswer::Done)
    }

    /// The leader, `from`, turns `who` out of the squad for good.
    fn expel_from_squad(
        &mut self,
        from: ClientId,
        kind: EventKind,
        who: ClientId,
    ) -> Option<SquadAnswer> {
        let (channel_id, _, _) = self.squad_asker(from, kind)?;
        let entry = self.live_squad(channel_id, kind)?;
        let name = entry.squad.expel(from, who)?;
        let mut members = member_ids(&entry.squad);
        members.push(who);
        info!(?kind, channel_id, member = who, "Squad member expelled");
        self.tell_squad(&members, keys::SQUAD_NOTICE_EXPELLED, &name);
        self.send_packet(who, build_stop_clock());
        Some(SquadAnswer::Done)
    }

    /// The leader, `from`, calls the squad off before it sets out.
    fn stop_squad(&mut self, from: ClientId, kind: EventKind) -> Option<SquadAnswer> {
        let (channel_id, _, name) = self.squad_asker(from, kind)?;
        let entry = self.live_squad(channel_id, kind)?;
        if entry.squad.leader() != from || entry.squad.started().is_some() {
            return None;
        }
        let members = member_ids(&entry.squad);
        self.squads.remove(&(channel_id, kind));
        info!(?kind, channel_id, leader = from, "Squad stopped");
        self.tell_squad(&members, keys::SQUAD_NOTICE_STOPPED, &name);
        for member in members {
            self.send_packet(member, build_stop_clock());
        }
        Some(SquadAnswer::Done)
    }

    /// The leader, `from`, takes the squad into its event: those of it on
    /// the event's lobby go, however few, and the rest may come in later
    /// (Cosmic `startInstance`, which counts the whole roster).
    fn start_squad(&mut self, from: ClientId, kind: EventKind) -> Option<SquadAnswer> {
        let says = net::squads::text(kind)?;
        let (channel_id, _, name) = self.squad_asker(from, kind)?;
        if self.event_busy(channel_id, kind) {
            return None;
        }
        let event = kind.event();
        let roster = {
            let entry = self.live_squad(channel_id, kind)?;
            if !entry.squad.may_start(from) {
                return None;
            }
            member_ids(&entry.squad)
        };
        let team: Vec<_> = roster
            .iter()
            .filter_map(|member| self.clients.get(member))
            .filter(|entry| {
                entry.channel_id == channel_id
                    && entry.presence.map_id == event.lobby
                    && !entry.in_cash_shop
            })
            .map(|entry| entry.handle.clone())
            .collect();
        if !team.iter().any(|member| member.client_id == from) {
            return None;
        }
        let entry = self.squads.get_mut(&(channel_id, kind))?;
        let id = self.next_event;
        self.next_event = self.next_event.wrapping_add(1).max(1);
        let ids: Vec<_> = team.iter().map(|member| member.client_id).collect();
        let queue = EventQueue::default();
        entry.squad.start(id);
        entry.event = Some(queue.joiner());
        self.event_opened(channel_id, kind, id);
        info!(?kind, id, channel_id, leader = from, team = ?ids, "Squad set out");
        self.tell_squad(&roster, says.notice_started, &name);
        for &member in &roster {
            self.send_packet(member, build_stop_clock());
        }
        Some(SquadAnswer::Start(EventStart {
            id,
            kind,
            channel_id,
            leader: from,
            team,
            blessing: None,
            queue,
            roster,
            couples: false,
        }))
    }

    /// `from`, on the roster of the squad of `kind` on their channel, goes
    /// back into its event, which answers `reply`. Returns `reply` if there
    /// is no way back.
    fn rejoin_squad(
        &mut self,
        from: ClientId,
        kind: EventKind,
        reply: oneshot::Sender<SquadAnswer>,
    ) -> Option<oneshot::Sender<SquadAnswer>> {
        let Some((channel_id, _, _)) = self.squad_asker(from, kind) else {
            return Some(reply);
        };
        let Some(handle) = self.clients.get(&from).map(|entry| entry.handle.clone()) else {
            return Some(reply);
        };
        let Some(entry) = self.live_squad(channel_id, kind) else {
            return Some(reply);
        };
        match &entry.event {
            Some(event) if entry.squad.has(from) => event.join(handle, reply),
            _ => Some(reply),
        }
    }

    /// A squad's notice about `name` to `members` (BROADCAST_MSG 6, Cosmic
    /// `serverNotice` 6).
    fn tell_squad(&self, members: &[ClientId], key: &str, name: &str) {
        let text = net::content::text().get(key, &[("name", name)]);
        for &member in members {
            self.send_packet(member, build_item_line(&text, 0));
        }
    }

    /// Event `id` of `kind` on `channel_id` is over or won: the squad that
    /// set out as it is done.
    pub(super) fn squad_closed(&mut self, channel_id: u8, kind: EventKind, id: u32) {
        let key = (channel_id, kind);
        if self
            .squads
            .get(&key)
            .is_some_and(|entry| entry.squad.started() == Some(id))
        {
            self.squads.remove(&key);
        }
    }
}

pub(super) fn member_ids(squad: &Squad) -> Vec<ClientId> {
    squad.members().iter().map(|&(id, _)| id).collect()
}
