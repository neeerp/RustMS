//! The directory keeps each channel's Guild Quest line (`net::guild_quest`,
//! Cosmic `EventManager`'s guild queue): one guild plays at a time, and when
//! its run is over the next guild whose registrant is on the channel has its
//! turn. The directory starts that guild's event itself, since the turn may
//! come with nobody asking, and keeps the way into it for the guild's
//! members while it gathers.

use super::WorldDirectory;
use crate::actor::event::{EventJoiner, EventQueue, EventStart};
use crate::message::SquadAnswer;
use net::events::EventKind;
use net::guild_quest::{self, Command, Line, View, Waiting};
use net::packet::build::world::guild::build_quest_waitlist;
use net::ClientId;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::{info, warn};

/// A channel's line, and the run of the guild whose turn it is.
#[derive(Default)]
pub(super) struct GuildQuestLine {
    line: Line,
    run: Option<GuildQuestRun>,
}

struct GuildQuestRun {
    id: u32,
    guild_id: i32,
    joiner: EventJoiner,
    /// When its members may no longer come in, by the directory's reckoning;
    /// the event has the last word.
    gathers_until: Instant,
}

impl WorldDirectory {
    /// The guild and channel of `from`, online and not in the Cash Shop.
    fn guild_quest_asker(&self, from: ClientId) -> Option<(u8, Option<i32>)> {
        let entry = self.clients.get(&from)?;
        (!entry.in_cash_shop).then_some((entry.channel_id, entry.presence.guild_id))
    }

    /// Whether `guild_id` plays or waits on any channel.
    fn guild_quest_listed(&self, guild_id: i32) -> bool {
        self.guild_quests
            .values()
            .any(|channel| channel.line.has(guild_id))
    }

    /// What Shuang tells `from` of their channel's line.
    pub(super) fn guild_quest_view(&self, from: ClientId) -> View {
        let Some((channel_id, guild_id)) = self.guild_quest_asker(from) else {
            return View::default();
        };
        let channel = self.guild_quests.get(&channel_id);
        let waiting = channel.map_or(0, |channel| channel.line.waiting());
        View {
            waiting,
            full: channel.is_some_and(|channel| channel.line.full()),
            listed: guild_id.is_some_and(|guild_id| self.guild_quest_listed(guild_id)),
            gathering: guild_id.is_some_and(|guild_id| {
                channel
                    .and_then(|channel| channel.run.as_ref())
                    .is_some_and(|run| {
                        run.guild_id == guild_id && Instant::now() < run.gathers_until
                    })
            }),
        }
    }

    /// `from` asks Shuang `command`. Registering answers Done once the
    /// guild is in line; joining is answered by the guild's event.
    pub(super) fn guild_quest_command(
        &mut self,
        from: ClientId,
        command: Command,
        reply: oneshot::Sender<SquadAnswer>,
    ) {
        let answer = match command {
            Command::Register => self.register_guild(from),
            Command::Join => {
                if let Some(reply) = self.join_guild_quest(from, reply) {
                    let _ = reply.send(SquadAnswer::Refused);
                }
                return;
            }
        };
        let _ = reply.send(answer.unwrap_or(SquadAnswer::Refused));
    }

    /// `from`'s guild goes to the end of their channel's line, and plays at
    /// once if no one else does.
    fn register_guild(&mut self, from: ClientId) -> Option<SquadAnswer> {
        let (channel_id, guild_id) = self.guild_quest_asker(from)?;
        let guild_id = guild_id?;
        if self.guild_quest_listed(guild_id) {
            return None;
        }
        let channel = self.guild_quests.entry(channel_id).or_default();
        let place = channel
            .line
            .register(Waiting {
                guild_id,
                registrant: from,
            })
            .ok()?;
        info!(
            channel_id,
            guild_id,
            registrant = from,
            place,
            "Guild registered for the Guild Quest"
        );
        self.next_guild(channel_id);
        // Unless it plays at once, which it heard of.
        if let Some(place) = self.guild_quests.get(&channel_id).and_then(|channel| {
            channel
                .line
                .line()
                .position(|guild| guild.guild_id == guild_id)
        }) {
            let number = guild_quest::waitlist_number(Some(place + 1));
            self.tell_guild_quest(guild_id, channel_id, number);
        }
        Some(SquadAnswer::Done)
    }

    /// `from` goes into their guild's run on their channel while it
    /// gathers. Returns `reply` if there is no way in.
    fn join_guild_quest(
        &mut self,
        from: ClientId,
        reply: oneshot::Sender<SquadAnswer>,
    ) -> Option<oneshot::Sender<SquadAnswer>> {
        let Some((channel_id, Some(guild_id))) = self.guild_quest_asker(from) else {
            return Some(reply);
        };
        let Some(handle) = self.clients.get(&from).map(|entry| entry.handle.clone()) else {
            return Some(reply);
        };
        match self
            .guild_quests
            .get(&channel_id)
            .and_then(|channel| channel.run.as_ref())
        {
            Some(run) if run.guild_id == guild_id => run.joiner.join(handle, reply),
            _ => Some(reply),
        }
    }

    /// If no guild plays on `channel_id`, the first in line whose
    /// registrant is on the channel takes its turn: its event opens, the
    /// registrant goes in if at the camp, and its members hear of it. Those
    /// passed over leave the line, and the rest hear their new places.
    fn next_guild(&mut self, channel_id: u8) {
        let kind = EventKind::GuildQuest;
        let Some((fields, directory)) = self
            .event_spawner
            .as_ref()
            .and_then(|(fields, directory)| Some((fields.clone(), directory.upgrade()?)))
        else {
            return;
        };
        if self.event_busy(channel_id, kind) {
            return;
        }
        let clients = &self.clients;
        let present = |registrant: ClientId| {
            clients
                .get(&registrant)
                .is_some_and(|entry| entry.channel_id == channel_id && !entry.in_cash_shop)
        };
        let channel = self.guild_quests.entry(channel_id).or_default();
        let (playing, passed) = channel.line.next(present);
        let waiting: Vec<_> = channel.line.line().map(|guild| guild.guild_id).collect();
        for guild in passed {
            info!(
                channel_id,
                guild_id = guild.guild_id,
                "Guild passed over: its registrant is away"
            );
            self.tell_guild_quest(guild.guild_id, channel_id, 0);
        }
        let Some(playing) = playing else {
            return;
        };
        for (place, guild_id) in waiting.into_iter().enumerate() {
            let number = guild_quest::waitlist_number(Some(place + 1));
            self.tell_guild_quest(guild_id, channel_id, number);
        }
        let id = self.next_event;
        self.next_event = self.next_event.wrapping_add(1).max(1);
        let queue = EventQueue::default();
        let joiner = queue.joiner();
        let lobby = kind.event().lobby;
        let team: Vec<_> = self
            .clients
            .get(&playing.registrant)
            .filter(|entry| entry.presence.map_id == lobby)
            .map(|entry| entry.handle.clone())
            .into_iter()
            .collect();
        self.event_opened(channel_id, kind, id);
        if let Some(channel) = self.guild_quests.get_mut(&channel_id) {
            channel.run = Some(GuildQuestRun {
                id,
                guild_id: playing.guild_id,
                joiner,
                gathers_until: Instant::now() + net::events::scaled(guild_quest::GATHERING),
            });
        }
        info!(
            channel_id,
            id,
            guild_id = playing.guild_id,
            "Guild Quest gathering"
        );
        self.tell_guild_quest(
            playing.guild_id,
            channel_id,
            guild_quest::waitlist_number(None),
        );
        crate::actor::event::spawn(
            EventStart {
                id,
                kind,
                channel_id,
                leader: playing.registrant,
                team,
                blessing: None,
                queue,
                roster: Vec::new(),
                couples: false,
            },
            fields,
            directory,
        );
    }

    /// Event `id` of `kind` on `channel_id` is over: its guild's turn is
    /// done, its notice goes, and the next guild may play.
    pub(super) fn guild_quest_closed(&mut self, channel_id: u8, kind: EventKind, id: u32) {
        if kind != EventKind::GuildQuest {
            return;
        }
        let Some(channel) = self.guild_quests.get_mut(&channel_id) else {
            return;
        };
        let Some(run) = channel.run.take_if(|run| run.id == id) else {
            return;
        };
        channel.line.done(run.guild_id);
        info!(channel_id, id, guild_id = run.guild_id, "Guild Quest over");
        self.tell_guild_quest(run.guild_id, channel_id, 0);
        self.next_guild(channel_id);
    }

    /// GUILD_OPERATION 0x4C to `guild_id`'s members online: where the guild
    /// stands on `channel_id`'s line (`guild_quest::waitlist_number`), or 0
    /// to clear it.
    fn tell_guild_quest(&self, guild_id: i32, channel_id: u8, number: i32) {
        let packet = match build_quest_waitlist(channel_id, number) {
            Ok(packet) => packet,
            Err(error) => return warn!(guild_id, %error, "Failed to build packet"),
        };
        for (_, entry) in self.guild_online(guild_id) {
            entry.handle.send_packet(packet.clone());
        }
    }
}
