//! The directory's part in families: asking someone to be a junior,
//! telling relatives of logins (report b039 section 5), and finding and
//! summoning relatives for the entitlements (report b050).

use super::{Entry, WorldDirectory};
use crate::actor::messenger::CHANNEL_CHANGE_GRACE;
use crate::db::spawn_db;
use crate::message::{FamilyTrip, Presence, ServerMessage, SummonFailure};
use game::family::FamilyBuff;
use game::family::{check_levels, FamilyChart, Refusal, Relatives, ENTITLEMENTS, SUMMON};
use net::packet::build::world::family::{
    build_family_chart, build_family_job_change, build_family_level_up, build_family_refusal,
    build_join_request, build_join_result, build_relative_online, build_summon_request,
};
use net::ClientId;
use std::time::{Duration, Instant};
use tracing::warn;

/// How long a summons waits for its answer. The client's dialog stays up
/// until answered (report b050); a later answer is ignored.
const SUMMON_TIMEOUT: Duration = Duration::from_secs(120);

/// A summons waiting for the answer of the member summoned.
pub(super) struct Summons {
    summoner: ClientId,
    /// The member's name, for the summoner's notices.
    name: String,
    /// Where the summoner was; they must still be there.
    map_id: i32,
    asked: Instant,
}

/// Pay a summoner who has left back for a summons, in the background:
/// their actor is gone.
fn refund_summons(summoner: ClientId) {
    let cost = ENTITLEMENTS[SUMMON].cost;
    tokio::spawn(async move {
        let refund = spawn_db(move || db::family::refund(summoner, SUMMON as i16, cost));
        if let Err(error) = refund.await {
            warn!(summoner, %error, "Failed to refund a summons");
        }
    });
}

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

    /// `from` spent the teleport entitlement: ask `target` where they
    /// stand, if they are on `from`'s channel.
    pub(super) fn family_locate(&self, from: ClientId, target: ClientId) {
        let Some(asker) = self.clients.get(&from) else {
            return;
        };
        let trip = FamilyTrip::Teleport;
        match self
            .clients
            .get(&target)
            .filter(|entry| entry.channel_id == asker.channel_id && entry.found_by(asker))
        {
            Some(entry) => entry.handle.send(ServerMessage::FamilyLocate {
                reply_to: asker.handle.clone(),
                trip,
            }),
            None => asker
                .handle
                .send(ServerMessage::FamilyLocated { trip, found: None }),
        };
    }

    /// `from` paid to summon `target` to `map_id`: ask them, if they are on
    /// the channel and no one else is summoning them.
    pub(super) fn family_summon(
        &mut self,
        from: ClientId,
        target: ClientId,
        name: String,
        (map_id, map_name): (i32, &str),
        now: Instant,
    ) {
        let Some(summoner) = self.clients.get(&from) else {
            return;
        };
        let why = match self.clients.get(&target) {
            Some(entry) if entry.channel_id == summoner.channel_id => {
                if self.family_summons.contains_key(&target) {
                    SummonFailure::Busy
                } else {
                    let packet = build_summon_request(&summoner.presence.name, map_name);
                    self.send_packet(target, packet);
                    let summons = Summons {
                        summoner: from,
                        name,
                        map_id,
                        asked: now,
                    };
                    self.family_summons.insert(target, summons);
                    return;
                }
            }
            _ => SummonFailure::NotFound,
        };
        self.tell(from, ServerMessage::SummonRefund { name, why });
    }

    /// `from` answered `summoner`'s summons. On a yes, the summoner, if
    /// still where they summoned from, tells them where to go. An answer to
    /// an older summons, whose dialog the client still shows, is ignored.
    pub(super) fn family_summon_answer(&mut self, from: ClientId, summoner: &str, accept: bool) {
        let answered = self.family_summons.get(&from).is_some_and(|summons| {
            self.clients
                .get(&summons.summoner)
                .is_some_and(|entry| entry.presence.name == summoner)
        });
        if !answered {
            return;
        }
        let Some(summons) = self.family_summons.remove(&from) else {
            return;
        };
        let (Some(summoner), Some(member)) =
            (self.clients.get(&summons.summoner), self.clients.get(&from))
        else {
            return;
        };
        let why = if !accept {
            SummonFailure::Declined
        } else if summoner.channel_id != member.channel_id
            || summoner.presence.map_id != summons.map_id
        {
            SummonFailure::Location
        } else {
            summoner.handle.send(ServerMessage::FamilyLocate {
                reply_to: member.handle.clone(),
                trip: FamilyTrip::Summoned {
                    summoner: summons.summoner,
                },
            });
            return;
        };
        let name = summons.name;
        summoner
            .handle
            .send(ServerMessage::SummonRefund { name, why });
    }

    /// `from` said yes but couldn't go; their summoner is paid back.
    pub(super) fn family_summon_failed(&self, from: ClientId, summoner: ClientId) {
        let Some(name) = self.clients.get(&from).map(|e| e.presence.name.clone()) else {
            return;
        };
        let why = SummonFailure::Location;
        self.tell(summoner, ServerMessage::SummonRefund { name, why });
    }

    /// `client_id` left: a summons waiting on them is paid back. Those they
    /// sent are dropped and paid back too, without a word.
    fn forget_summons(&mut self, client_id: ClientId) {
        if let Some(summons) = self.family_summons.remove(&client_id) {
            let (name, why) = (summons.name, SummonFailure::NotFound);
            self.tell(summons.summoner, ServerMessage::SummonRefund { name, why });
        }
        let before = self.family_summons.len();
        self.family_summons
            .retain(|_, summons| summons.summoner != client_id);
        for _ in self.family_summons.len()..before {
            refund_summons(client_id);
        }
    }

    /// `from` bought `buff` for their party, and has it: each other member
    /// on their channel gets it too.
    pub(super) fn family_party_buff(&self, from: ClientId, buff: FamilyBuff) {
        let Some(channel) = self.clients.get(&from).map(|entry| entry.channel_id) else {
            return;
        };
        let Some(party) = self.parties.of(from) else {
            return;
        };
        for member in &party.members {
            let here = self
                .clients
                .get(&member.id)
                .is_some_and(|entry| entry.channel_id == channel);
            if member.id != from && here {
                self.tell(member.id, ServerMessage::FamilyBuff(buff));
            }
        }
    }

    /// Summonses unanswered for too long count as declined.
    pub(super) fn purge_family_summons(&mut self, now: Instant) {
        let late: Vec<_> = self
            .family_summons
            .iter()
            .filter(|(_, summons)| now.duration_since(summons.asked) >= SUMMON_TIMEOUT)
            .map(|(&id, _)| id)
            .collect();
        for id in late {
            if let Some(summons) = self.family_summons.remove(&id) {
                let (name, why) = (summons.name, SummonFailure::Declined);
                self.tell(summons.summoner, ServerMessage::SummonRefund { name, why });
            }
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

    /// `from` earned their seniors `gain` Rep: their actor applies it, with
    /// the senior's level if they are online, for the halving.
    pub(super) fn family_earn(&self, from: ClientId, gain: i32) {
        let Some(senior) = self
            .clients
            .get(&from)
            .and_then(|entry| entry.presence.family.senior)
        else {
            return;
        };
        let senior_level = self.clients.get(&senior).map(|entry| entry.presence.level);
        self.tell(from, ServerMessage::EarnRep { gain, senior_level });
    }

    /// The senior reads a line for each level a junior gains, with a
    /// warning once the junior has passed them; relatives read a line for
    /// a new job (report b049 section 5).
    pub(super) fn tell_family_progress(&self, client_id: ClientId, previous: &Presence) {
        let Some(now) = self.clients.get(&client_id).map(|entry| &entry.presence) else {
            return;
        };
        if now.family != previous.family {
            return;
        }
        if let Some(senior) = now.family.senior.and_then(|id| self.clients.get(&id)) {
            for level in previous.level.saturating_add(1)..=now.level {
                let halved = senior.presence.level < level;
                self.send_packet(
                    senior.handle.client_id,
                    build_family_level_up(halved, level, &now.name),
                );
            }
        }
        if previous.job != now.job {
            for id in now.family.ids() {
                self.send_packet(id, build_family_job_change(now.job, &now.name));
            }
        }
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
        self.forget_summons(entry.handle.client_id);
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
