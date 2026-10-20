use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::conti::build_countdown;
use net::packet::handle::world::warp_to_spawn_point;
use std::time::Duration;
use tokio::time::Instant;

/// A map with a time limit the player is on: when their time is up, and
/// where they go then.
pub(super) struct MapTimer {
    map_id: i32,
    ends: Instant,
    to: i32,
}

impl ClientActor {
    /// On a map with `info/timeLimit` (or one a script sets,
    /// `npc_scripts::time_limit`) and another map to return to, the client
    /// shows the countdown and the player has that long. Any other
    /// map clears the timer, so coming back starts it again.
    pub(super) async fn start_map_timer(&mut self) -> Result<(), RuntimeError> {
        self.map_timer = None;
        let map_id = self.in_game()?.player.stats.map_id;
        let Ok(data) = net::get_game_data() else {
            return Ok(());
        };
        let Some(field) = data.field(map_id) else {
            return Ok(());
        };
        let seconds = field
            .time_limit
            .or_else(|| net::npc_scripts::time_limit(map_id));
        let (Some(seconds), Some(to)) = (seconds, field.forced_return) else {
            return Ok(());
        };
        if to == map_id || !data.field_exists(to) {
            return Ok(());
        }
        self.map_timer = Some(MapTimer {
            map_id,
            ends: Instant::now() + Duration::from_secs(u64::from(seconds)),
            to,
        });
        self.send(&mut build_countdown(seconds)?).await
    }

    /// When the player's time on their map is up.
    pub(super) fn map_timer_due(&self) -> Option<Instant> {
        self.map_timer.as_ref().map(|timer| timer.ends)
    }

    /// Time is up: the player goes to the map's `forcedReturn`, at its first
    /// portal, if still on the map and out of the Cash Shop.
    pub(super) async fn end_map_timer(&mut self) -> Result<(), RuntimeError> {
        let Some(timer) = self.map_timer.take() else {
            return Ok(());
        };
        if self.in_cash_shop {
            return Ok(());
        }
        let channel_id = self.channel_id;
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != timer.map_id {
            return Ok(());
        }
        for action in warp_to_spawn_point(&mut game.player, timer.to, 0, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }
}
