use super::ClientActor;
use crate::error::RuntimeError;
use game::Player;
use net::conti;
use net::packet::build::world::conti::build_clock_time;
use net::packet::handle::world::warp;

/// A player who logs in on a map nobody stays on, such as a ship under way,
/// starts on the map's `forcedReturn` instead (Cosmic applies it on save).
pub(super) fn leave_forced_return_map(player: &mut Player) {
    let Ok(data) = net::get_game_data() else {
        return;
    };
    let to = data
        .field(player.stats.map_id)
        .and_then(|field| field.forced_return)
        .filter(|&to| to != player.stats.map_id && data.field_exists(to));
    if let Some(to) = to {
        player.move_to(to, 0);
    }
}

impl ClientActor {
    /// A station's wall clock shows the time of day once the player is in.
    pub(super) async fn set_wall_clock(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let has_clock = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
            .is_some_and(|field| field.clock);
        if !has_clock {
            return Ok(());
        }
        let (hour, minute, second) = conti::time_of_day(conti::now());
        self.send(&mut build_clock_time(hour, minute, second)?)
            .await
    }

    /// A ship's trip moves the player on, if they are still on its map and
    /// out of the Cash Shop.
    pub(super) async fn ship_warp(
        &mut self,
        from_map: i32,
        map_id: i32,
        portal: &str,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let channel_id = self.channel_id;
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != from_map {
            return Ok(());
        }
        for action in warp(&mut game.player, map_id, portal, channel_id)? {
            self.apply(action).await?;
        }
        Ok(())
    }
}
