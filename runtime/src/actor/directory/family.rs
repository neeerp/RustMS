//! The directory's part in families: asking someone to be a junior, and
//! telling relatives of logins (report b039 section 5).

use super::{Entry, WorldDirectory};
use crate::actor::messenger::CHANNEL_CHANGE_GRACE;
use crate::message::ServerMessage;
use game::family::{check_levels, FamilyChart, Refusal, Relatives};
use net::packet::build::world::family::{
    build_family_chart, build_family_refusal, build_join_request, build_join_result,
    build_relative_online,
};
use net::ClientId;
use std::time::Instant;

/// A family member changing channel, and when they left.
pub(super) struct Away {
    name: String,
    relatives: Relatives,
    left: Instant,
}

impl WorldDirectory {
    /// `from` asks `junior` to be their junior. The database's checks have
    /// passed; these need both online.
    pub(super) fn family_invite(&mut self, from: ClientId, junior: ClientId, now: Instant) {
        if let Some(refusal) = self.junior_refusal(from, junior) {
            self.send_packet(from, build_family_refusal(refusal));
            return;
        }
        // The junior's client would decline a second card for the first,
        // so the same senior asking again while it is up is ignored.
        if self.family_invites.is_pending(junior, from, now) {
            return;
        }
        let Some(name) = self.clients.get(&from).map(|e| e.presence.name.clone()) else {
            return;
        };
        // One senior at a time may ask.
        if !self.family_invites.invite(from, from, junior, now) {
            self.send_packet(from, build_family_refusal(Refusal::Busy));
            return;
        }
        self.send_packet(junior, build_join_request(from, &name));
    }

    /// Why `junior` can't be `senior`'s now: not online, on another map,
    /// or of the wrong level.
    fn junior_refusal(&self, senior: ClientId, junior: ClientId) -> Option<Refusal> {
        let senior = self.clients.get(&senior)?;
        match self.clients.get(&junior) {
            None => Some(Refusal::NotFound),
            Some(entry) if !entry.shares_map_with(senior) => Some(Refusal::NotSameMap),
            Some(entry) => check_levels(senior.presence.level, entry.presence.level).err(),
        }
    }

    /// `from` answers `inviter`'s request. A yes goes to the senior's actor,
    /// which records it; a no is passed on.
    pub(super) fn family_answer(
        &mut self,
        from: ClientId,
        inviter: ClientId,
        accept: bool,
        now: Instant,
    ) {
        let Some(name) = self
            .clients
            .get(&from)
            .map(|entry| entry.presence.name.clone())
        else {
            return;
        };
        if accept {
            if !self.family_invites.accept(from, inviter, now) {
                return;
            }
            // Either may have moved or levelled since the card came up.
            match self.junior_refusal(inviter, from) {
                None => self.tell(inviter, ServerMessage::AddJunior { junior: from, name }),
                Some(refusal) => self.send_packet(inviter, build_family_refusal(refusal)),
            }
        } else if self.family_invites.decline(from, inviter) {
            self.send_packet(inviter, build_join_result(false, &name));
        }
    }

    /// The members online are shown so, at their level and map now.
    pub(super) fn send_family_chart(&self, owner: ClientId, mut chart: FamilyChart) {
        for member in &mut chart.members {
            if let Some(entry) = self.clients.get(&member.id) {
                member.online = true;
                member.job = entry.presence.job;
                member.level = entry.presence.level;
                member.map_id = entry.presence.map_id;
            } else if self.family_away.contains_key(&member.id) {
                // Changing channel, as the guild list has it.
                member.online = true;
            }
        }
        self.send_packet(owner, build_family_chart(&chart));
    }

    /// Tells the senior and juniors online that `client_id` logged in or out.
    pub(super) fn tell_relatives(&self, client_id: ClientId, online: bool) {
        let Some(entry) = self.clients.get(&client_id) else {
            return;
        };
        self.tell_relatives_of(&entry.presence.name, &entry.presence.family, online);
    }

    fn tell_relatives_of(&self, name: &str, relatives: &Relatives, online: bool) {
        for id in relatives.ids() {
            self.send_packet(id, build_relative_online(online, name));
        }
    }

    /// A member left: changing channel, they are held for a while;
    /// otherwise their relatives hear at once.
    pub(super) fn family_left(&mut self, entry: &Entry, changing_channel: bool) {
        let relatives = &entry.presence.family;
        if relatives.senior.is_none() && relatives.juniors.is_empty() {
            return;
        }
        if changing_channel {
            let away = Away {
                name: entry.presence.name.clone(),
                relatives: relatives.clone(),
                left: Instant::now(),
            };
            self.family_away.insert(entry.handle.client_id, away);
        } else {
            self.tell_relatives_of(&entry.presence.name, relatives, false);
        }
    }

    /// Members who changed channel and didn't arrive in time left the game.
    pub(super) fn purge_family_away(&mut self, now: Instant) {
        let gone: Vec<_> = self
            .family_away
            .iter()
            .filter(|(_, away)| now.duration_since(away.left) >= CHANNEL_CHANGE_GRACE)
            .map(|(&id, _)| id)
            .collect();
        for id in gone {
            if let Some(away) = self.family_away.remove(&id) {
                self.tell_relatives_of(&away.name, &away.relatives, false);
            }
        }
    }
}
