//! A GM's event on the player's channel (report b170): a GM opens and
//! closes it, and an Event Assistant takes players to their team's spot.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldMessage};
use game::character::EVENT_RETURN;
use game_data::server_content::keys;
use net::npc_scripts::NpcScript;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::scripts::Effect;
use net::GmEntry;
use rand::Rng;
use tokio::sync::oneshot;

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
    /// them), keeps a team on its field, and goes to the team's spot.
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
        let spot = match net::get_game_data()?.field(map_id) {
            Some(field) => net::gm_event::spot(field, team, rand::thread_rng().gen()),
            None => 0,
        };
        let from = self.in_game()?.player.stats.map_id;
        Box::pin(self.apply_effects(&[
            Effect::SaveLocation(EVENT_RETURN, Some(from)),
            Effect::WarpToPortal(map_id, spot),
        ]))
        .await?;
        Ok(())
    }
}
