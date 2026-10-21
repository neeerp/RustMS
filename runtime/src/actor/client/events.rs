//! The player's side of a party event (`crate::actor::event`): the ticket
//! that puts them in the event's own copy of its maps, the event's clock,
//! its stages' gates, and getting out when it ends or they leave.

use super::quest::gain_exp;
use super::ClientActor;
use crate::actor::event::EventTicket;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldKey};
use net::events::{EventCommand, EventKind, KPQ};
use net::npc_scripts::NpcScript;
use net::packet::build::world::conti::{build_countdown, build_stop_clock};
use net::packet::build::world::effect::{
    build_field_effect, build_field_object, build_field_sound,
};
use net::packet::handle::world::{portal_name, warp_to_spawn_point};
use packet::Packet;
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

    /// Cloto sets the party quest's first stage for a player in it there.
    /// Returns whether she did; elsewhere she has nothing to say.
    pub(super) async fn talk_to_cloto(&mut self) -> Result<bool, RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|_| map_id == KPQ.entry) else {
            return Ok(false);
        };
        let Some(view) = ticket.cloto().await else {
            return Ok(false);
        };
        let script = NpcScript::cloto(view, &self.in_game()?.player);
        self.start_script(script).await?;
        Ok(true)
    }

    /// The player's script did something their event keeps track of.
    pub(super) async fn tell_event(&self, command: EventCommand) {
        if let Some(ticket) = &self.event {
            ticket.command(command).await;
        }
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

    /// Whether `packet`, a walk through a portal, would leave a stage before
    /// the party has cleared it. The client doesn't stop the player itself.
    pub(super) fn gate_closed(&self, packet: &Packet) -> bool {
        let (Some(ticket), Some(game)) = (&self.event, &self.game) else {
            return false;
        };
        let map_id = game.player.stats.map_id;
        if self.in_cash_shop || !ticket.holds(map_id) {
            return false;
        }
        portal_name(packet)
            .and_then(|portal| ticket.kind.event().gate_stage(map_id, &portal))
            .is_some_and(|stage| !ticket.cleared(stage))
    }

    /// The party cleared `stage` of event `id`: those on its map see and
    /// hear it and its gate opens, and everyone gets the stage's EXP.
    pub(super) async fn stage_cleared(&mut self, id: u32, stage: u8) -> Result<(), RuntimeError> {
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.id == id) else {
            return Ok(());
        };
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        let event = ticket.kind.event();
        let map_id = event.stage_map(stage);
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if next.stats.map_id == map_id {
            packets.push(build_field_effect("quest/party/clear")?);
            packets.push(build_field_sound("Party1/Clear")?);
            packets.push(build_field_object("gate")?);
        }
        gain_exp(&mut next, event.stage_exp(stage), &mut packets)?;
        self.commit_player(next, None, &mut packets).await?;
        self.send_all(&mut packets).await
    }

    /// A cleared stage's gate stays open for whoever comes onto its map.
    pub(super) async fn show_event_gate(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(ticket) = self.event.as_ref().filter(|ticket| ticket.holds(map_id)) else {
            return Ok(());
        };
        let event = ticket.kind.event();
        let cleared = event
            .stage_on(map_id)
            .is_some_and(|stage| ticket.cleared(stage));
        if !cleared {
            return Ok(());
        }
        self.send(&mut build_field_object("gate")?).await
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
