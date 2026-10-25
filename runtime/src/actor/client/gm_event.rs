//! A GM's event on the player's channel (report b170): a GM opens and
//! closes it, and an Event Assistant takes players to their team's spot.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldMessage};
use game::character::EVENT_RETURN;
use game_data::server_content::keys;
use net::conti::Portal;
use net::npc_scripts::NpcScript;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::handle::world::portal_name;
use net::scripts::Effect;
use net::{FieldRequest, GmEntry, GmEvent};
use packet::Packet;
use rand::Rng;
use tokio::sync::oneshot;
use tokio::time::Instant;

impl ClientActor {
    /// A GM opens their channel's event, and tells the channel, or closes
    /// it.
    pub(super) async fn gm_entry(&mut self, entry: GmEntry) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let GmEntry::Open { map_id, limit } = entry else {
            return self
                .send_to_directory(DirectoryMessage::CloseGmEvent { channel_id })
                .await;
        };
        self.send_to_directory(DirectoryMessage::OpenGmEvent {
            channel_id,
            map_id,
            limit,
        })
        .await?;
        let map = net::items::map_name(map_id).unwrap_or_else(|| map_id.to_string());
        let notice = net::content::text().get(keys::GM_EVENT_OPENED, &[("map", &map)]);
        self.send_to_directory(DirectoryMessage::Broadcast {
            from: self.client_id,
            scope: net::BroadcastScope::Channel(channel_id),
            packet: build_broadcast(Broadcast::Notice, &notice)?,
        })
        .await
    }

    /// An Event Assistant, as the channel's event is open or not.
    pub(super) async fn talk_to_event_assistant(
        &mut self,
        npc_id: i32,
    ) -> Result<(), RuntimeError> {
        let (reply, open) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::GmEventOpen {
            channel_id: self.channel_id,
            reply,
        })
        .await?;
        let open = open.await.ok().flatten().is_some();
        self.start_script(NpcScript::event_assistant(npc_id, open))
            .await
    }

    /// The player takes a place at the channel's event (`npc_id` sent
    /// them) and goes to its lobby, or keeps a team on its field and goes
    /// to the team's spot.
    pub(super) async fn join_gm_event(&mut self, npc_id: i32) -> Result<(), RuntimeError> {
        let (reply, place) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::JoinGmEvent {
            channel_id: self.channel_id,
            reply,
        })
        .await?;
        let Some(map_id) = place.await.ok().flatten() else {
            return Box::pin(self.start_script(NpcScript::event_assistant_refused(npc_id))).await;
        };
        let from = self.in_game()?.player.stats.map_id;
        let (to, spot) = match net::gm_event::lobby(map_id) {
            Some(lobby) => (lobby, 0),
            None => (map_id, self.team_spot(map_id).await?),
        };
        Box::pin(self.apply_effects(&[
            Effect::SaveLocation(EVENT_RETURN, Some(from)),
            Effect::WarpToPortal(to, spot),
        ]))
        .await?;
        Ok(())
    }

    /// A GM's `/start` or `/reset`. In a Snowball lobby, `/start` starts the
    /// game on its field, if it isn't on yet, and sends everyone else
    /// there; again later, it sends whoever came since. On any map of a
    /// timed event, each of its maps starts or ends it, all from now.
    pub(super) async fn run_gm_event(&mut self, event: GmEvent) -> Result<(), RuntimeError> {
        let from = self.client_id;
        let lobby = self.in_game()?.player.stats.map_id;
        if let Some(timed) = net::gm_event::timed_event(lobby) {
            let started = (event == GmEvent::Start).then(Instant::now);
            for &map_id in timed.maps {
                self.services
                    .fields
                    .field(self.field_key(map_id))
                    .tx
                    .send(FieldMessage::RunTimedEvent { started })
                    .await
                    .map_err(|_| RuntimeError::ChannelSend)?;
            }
            return Ok(());
        }
        let field = net::gm_event::lobby_field(lobby).filter(|_| event == GmEvent::Start);
        let Some(map_id) = field else {
            let request = FieldRequest::GmEvent(event);
            return self
                .send_to_field(FieldMessage::FromClient { from, request })
                .await;
        };
        let request = FieldRequest::GmEvent(GmEvent::Start);
        self.services
            .fields
            .field(self.field_key(map_id))
            .tx
            .send(FieldMessage::FromClient { from, request })
            .await
            .map_err(|_| RuntimeError::ChannelSend)?;
        self.send_to_field(FieldMessage::SendToGmEvent { from, map_id })
            .await
    }

    /// Whether `packet`, a walk through a portal, is through a timed
    /// event's gate before the GM started it. The client doesn't stop the
    /// player itself (report b173).
    pub(super) async fn gm_gate_closed(&self, packet: &Packet) -> Result<bool, RuntimeError> {
        let Some(game) = self.game.as_ref().filter(|_| !self.in_cash_shop) else {
            return Ok(false);
        };
        let map_id = game.player.stats.map_id;
        if !portal_name(packet).is_some_and(|portal| net::gm_event::is_gate(map_id, &portal)) {
            return Ok(false);
        }
        let (reply, on) = oneshot::channel();
        game.field
            .tx
            .send(FieldMessage::TimedEventOn { reply })
            .await
            .map_err(|_| RuntimeError::ChannelSend)?;
        Ok(!on.await.unwrap_or(false))
    }

    /// The GM's `/start` in the lobby `from_map` sends the player on to
    /// their team's spot on the game field `map_id`.
    pub(super) async fn go_to_gm_event(
        &mut self,
        from_map: i32,
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        let here = self.game.as_ref().map(|game| game.player.stats.map_id);
        if self.in_cash_shop || here != Some(from_map) {
            return Ok(());
        }
        let spot = self.team_spot(map_id).await?;
        self.ship_warp(from_map, map_id, Portal::Id(spot)).await
    }

    /// A team kept for the player on the game field `map_id`, and its spot.
    async fn team_spot(&mut self, map_id: i32) -> Result<u8, RuntimeError> {
        let (reply, team) = oneshot::channel();
        let field = self.services.fields.field(self.field_key(map_id));
        field
            .tx
            .send(FieldMessage::ReserveTeam {
                client_id: self.client_id,
                reply,
            })
            .await
            .map_err(|_| RuntimeError::ChannelSend)?;
        let team = team.await.ok().flatten().unwrap_or(0);
        Ok(match net::get_game_data()?.field(map_id) {
            Some(field) => net::gm_event::spot(field, team, rand::thread_rng().gen()),
            None => 0,
        })
    }
}
