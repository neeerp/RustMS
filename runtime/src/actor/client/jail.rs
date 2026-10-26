//! A GM's `!jail` (row 205 S6): the jailed player stays in the jail map
//! until their time is up, however they leave it. The time is in the
//! database (`db::jail`), so a logout or a channel change doesn't end it;
//! `ServerMessage::JailChanged` says a GM changed it.

use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use net::gm_commands::{JAIL_EXIT, JAIL_MAP};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::handle::world::warp_to_spawn_point;
use std::time::SystemTime;
use tokio::time::Instant;

impl ClientActor {
    /// When the player's time in jail is up. Not in the Cash Shop, which
    /// they leave by entering the game again.
    pub(super) fn jail_due(&self) -> Option<Instant> {
        if self.in_cash_shop {
            return None;
        }
        let wait = self
            .jailed_until?
            .duration_since(SystemTime::now())
            .unwrap_or_default();
        Some(Instant::now() + wait)
    }

    /// A GM jailed the player or let them out: in they go, or out.
    pub(super) async fn jail_changed(&mut self) -> Result<(), RuntimeError> {
        let character_id = self.client_id;
        let now = SystemTime::now();
        let until = spawn_db(move || db::jail::jailed_until(character_id, now)).await?;
        let was_jailed = self.jailed_until.is_some();
        self.jailed_until = until;
        if self.in_cash_shop || self.game.is_none() {
            return Ok(());
        }
        match until {
            Some(until) => {
                let minutes = until
                    .duration_since(now)
                    .unwrap_or_default()
                    .as_secs()
                    .div_ceil(60);
                self.jail_line(&format!(
                    "A GM put you in jail. You will be let out in {minutes} minutes."
                ))
                .await?;
                self.hold_in_jail().await
            }
            None if was_jailed => self.let_out_of_jail().await,
            None => Ok(()),
        }
    }

    /// The player's time is up: out of the jail map they go, unless a GM
    /// added time this actor didn't hear about.
    pub(super) async fn end_jail(&mut self) -> Result<(), RuntimeError> {
        let character_id = self.client_id;
        let now = SystemTime::now();
        self.jailed_until = spawn_db(move || db::jail::jailed_until(character_id, now)).await?;
        if self.jailed_until.is_some() {
            return Ok(());
        }
        self.let_out_of_jail().await
    }

    /// A jailed player who reached another map goes back to jail. Not
    /// while dead: their revival's map change comes back here.
    pub(super) async fn hold_in_jail(&mut self) -> Result<(), RuntimeError> {
        let stats = &self.in_game()?.player.stats;
        if self.jailed_until.is_none() || stats.map_id == JAIL_MAP || stats.hp <= 0 {
            return Ok(());
        }
        self.warp_to(JAIL_MAP, 0).await
    }

    /// Not while dead: the jail map's return map is the cellar anyway.
    async fn let_out_of_jail(&mut self) -> Result<(), RuntimeError> {
        let stats = &self.in_game()?.player.stats;
        if stats.map_id != JAIL_MAP || stats.hp <= 0 {
            return Ok(());
        }
        self.jail_line("You are free to go.").await?;
        let (map_id, portal) = JAIL_EXIT;
        let spawn_point = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id)?.portal_by_name(portal))
            .and_then(|portal| u8::try_from(portal.id).ok())
            .unwrap_or(0);
        self.warp_to(map_id, spawn_point).await
    }

    async fn warp_to(&mut self, map_id: i32, spawn_point: u8) -> Result<(), RuntimeError> {
        let channel_id = self.channel_id;
        let player = &mut self.in_game_mut()?.player;
        for action in warp_to_spawn_point(player, map_id, spawn_point, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }

    async fn jail_line(&mut self, text: &str) -> Result<(), RuntimeError> {
        self.send(&mut build_broadcast(Broadcast::Pink, text)?)
            .await
    }
}
