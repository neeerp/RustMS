//! The player's side of a party event (`crate::actor::event`): the ticket
//! that puts them in the event's own copy of its maps, the event's clock,
//! and getting out when it ends or they leave.

use super::ClientActor;
use crate::actor::event::EventTicket;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldKey};
use net::events::EventKind;
use net::npc_scripts::NpcScript;
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::handle::world::warp_to_spawn_point;
use tokio::sync::oneshot;
use tokio::time::Instant;

impl ClientActor {
    /// The field `map_id` is played in: the event's copy of it for a member
    /// holding a ticket to its maps, otherwise the channel's own.
    pub(super) fn field_key(&self, map_id: i32) -> FieldKey {
        match &self.event {
            Some(ticket) if ticket.holds(map_id) => FieldKey {
                channel_id: self.channel_id,
                map_id,
                instance_id: ticket.id,
            },
            _ => FieldKey::new(self.channel_id, map_id),
        }
    }

    /// The NPC who starts an event tells the player whether their party may
    /// go in.
    pub(super) async fn talk_about_event(&mut self, kind: EventKind) -> Result<(), RuntimeError> {
        let (reply, entry) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::EventEntry {
            from: self.client_id,
            kind,
            reply,
        })
        .await?;
        let Ok(entry) = entry.await else {
            return Ok(());
        };
        let script = match kind {
            EventKind::Kpq => NpcScript::lakelis(entry),
        };
        self.start_script(script).await
    }

    /// The party went in: the player takes their ticket to the event's
    /// first map, unless they have since left the party or the lobby, or
    /// are in the Cash Shop or another event.
    pub(super) async fn event_started(&mut self, ticket: EventTicket) -> Result<(), RuntimeError> {
        let lobby = ticket.kind.event().lobby;
        let waiting = self.party_id.is_some()
            && !self.in_cash_shop
            && self.event.is_none()
            && self
                .game
                .as_ref()
                .is_some_and(|game| game.player.stats.map_id == lobby);
        if !waiting {
            // Dropping the ticket hands it back.
            return Ok(());
        }
        let entry = ticket.kind.event().entry;
        self.event = Some(ticket);
        self.warp_within_event(entry).await
    }

    /// Event `id` ended: out of its maps, if the player still holds its
    /// ticket.
    pub(super) async fn event_ended(&mut self, id: u32) -> Result<(), RuntimeError> {
        if self.event.as_ref().is_none_or(|ticket| ticket.id != id) {
            return Ok(());
        }
        self.leave_event().await
    }

    /// Without a party there is no event: the player leaves it.
    pub(super) async fn left_party_in_event(&mut self) -> Result<(), RuntimeError> {
        if self.party_id.is_none() && self.event.is_some() {
            self.leave_event().await?;
        }
        Ok(())
    }

    /// Hand the ticket back, and go to the event's exit if on one of its
    /// maps.
    async fn leave_event(&mut self) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref() else {
            return Ok(());
        };
        let exit = ticket.kind.event().exit;
        if !self.in_cash_shop && ticket.holds(self.in_game()?.player.stats.map_id) {
            // Changing field to the exit hands the ticket back.
            return self.warp_within_event(exit).await;
        }
        self.drop_event_ticket().await
    }

    async fn warp_within_event(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let player = &mut self.in_game_mut()?.player;
        for action in warp_to_spawn_point(player, map_id, 0, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// The player is going to `map_id`: off the event's maps, they hand
    /// their ticket back.
    pub(super) async fn leaving_for(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        if self
            .event
            .as_ref()
            .is_some_and(|ticket| !ticket.holds(map_id))
        {
            self.drop_event_ticket().await?;
        }
        Ok(())
    }

    async fn drop_event_ticket(&mut self) -> Result<(), RuntimeError> {
        // Dropping the ticket hands it back.
        if self.event.take().is_some() {
            if self.game.is_some() && !self.in_cash_shop {
                self.send(&mut build_stop_clock()?).await?;
            }
        }
        Ok(())
    }

    /// On one of the event's maps the client shows the time left.
    pub(super) async fn show_event_clock(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(());
        };
        let left = ticket.ends.saturating_duration_since(Instant::now());
        let seconds = u32::try_from(left.as_secs()).unwrap_or(u32::MAX);
        self.send(&mut build_countdown(seconds)?).await
    }

    /// A player who leaves the game leaves the event.
    pub(super) fn forget_event(&mut self) {
        self.event = None;
    }
}
