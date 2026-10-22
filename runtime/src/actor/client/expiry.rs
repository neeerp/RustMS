//! Items whose time ran out: the player's client is told and they go.

use super::ClientActor;
use crate::error::RuntimeError;
use net::items::{expire_items, next_expiry};
use net::packet::build::world::inventory::{
    build_char_look, build_inventory_operation, build_items_expired, build_seals_expired,
};
use net::packet::build::world::stat::build_hp_mp;
use net::WorldAction;
use std::time::SystemTime;
use tokio::time::Instant;

impl ClientActor {
    /// When the next of the player's items expires; not in the Cash Shop.
    pub(super) fn items_due(&self) -> Option<Instant> {
        if self.in_cash_shop {
            return None;
        }
        let wait = next_expiry(&self.game.as_ref()?.player)?
            .duration_since(SystemTime::now())
            .unwrap_or_default();
        Some(Instant::now() + wait)
    }

    /// Take the items whose time ran out, saying which, and show the others
    /// on the map the player's new look if one was worn.
    pub(super) async fn expire_items(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let expired = expire_items(player, SystemTime::now());
        if expired.changes.is_empty() {
            return Ok(());
        }
        let mut packets = Vec::new();
        if !expired.item_ids.is_empty() {
            packets.push(build_items_expired(&expired.item_ids)?);
        }
        if !expired.released.is_empty() {
            packets.push(build_seals_expired(&expired.released)?);
        }
        packets.push(build_inventory_operation(false, &expired.changes)?);
        if !expired.worn {
            return self.send_all(&mut packets).await;
        }
        if player.fit_hp_mp() {
            packets.push(build_hp_mp(&player.stats)?);
        }
        let look = build_char_look(player)?;
        self.send_all(&mut packets).await?;
        self.apply(WorldAction::LookChanged(look)).await
    }
}
