use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::conti::build_countdown;
use net::packet::handle::world::warp_to_spawn_point;
use std::time::Duration;
use tokio::time::Instant;

/// A map with a time limit the player is on: when their time is up, and
/// where they go then, at which spawn point.
pub(super) struct MapTimer {
    map_id: i32,
    started: Instant,
    ends: Instant,
    to: (i32, u8),
}

impl ClientActor {
    /// On a map with `info/timeLimit` (or one a script sets,
    /// `npc_scripts::time_limit`) and another map to return to, the client
    /// shows the countdown and the player has that long. A ride
    /// (`rides::ride_on`) takes its own time and arrives at its own map. Any
    /// other map clears the timer, so coming back starts it again. A player
    /// too high for a Training Center (`net::map_rules::moved_out_to`) has
    /// no time at all there.
    pub(super) async fn start_map_timer(&mut self) -> Result<(), RuntimeError> {
        self.map_timer = None;
        let player = &self.in_game()?.player;
        let map_id = player.stats.map_id;
        let Ok(data) = net::get_game_data() else {
            return Ok(());
        };
        let Some(field) = data.field(map_id) else {
            return Ok(());
        };
        if let Some(to) =
            net::map_rules::moved_out_to(player, field).filter(|&to| data.field_exists(to))
        {
            self.map_timer = Some(MapTimer {
                map_id,
                started: Instant::now(),
                ends: Instant::now(),
                to: (to, 0),
            });
            return Ok(());
        }
        let limit = match net::rides::ride_on(map_id) {
            Some(ride) => Some((ride.countdown(), ride.to)),
            None => field
                .time_limit
                .or_else(|| net::npc_scripts::time_limit(map_id))
                .zip(field.forced_return.map(|to| (to, 0))),
        };
        let Some((seconds, to)) = limit else {
            return Ok(());
        };
        if to.0 == map_id || !data.field_exists(to.0) {
            return Ok(());
        }
        let started = Instant::now();
        self.map_timer = Some(MapTimer {
            map_id,
            started,
            ends: started + Duration::from_secs(u64::from(seconds)),
            to,
        });
        self.send(&mut build_countdown(seconds)?).await
    }

    /// How long the player has been on their timed map, as its timer
    /// counts.
    pub(super) fn time_on_map(&self) -> Option<Duration> {
        self.map_timer.as_ref().map(|timer| timer.started.elapsed())
    }

    /// When the player's time on their map is up.
    pub(super) fn map_timer_due(&self) -> Option<Instant> {
        self.map_timer.as_ref().map(|timer| timer.ends)
    }

    /// Time is up: the player goes to the map's `forcedReturn`, at its first
    /// portal, or to a ride's end, if still on the map and out of the Cash
    /// Shop.
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
        for action in warp_to_spawn_point(&mut game.player, timer.to.0, timer.to.1, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }
}
