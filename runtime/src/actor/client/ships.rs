use super::ClientActor;
use crate::error::RuntimeError;
use game::Player;
use net::conti::{self, Portal};
use net::packet::build::world::conti::build_clock_time;
use net::packet::handle::world::{warp, warp_to_spawn_point};

/// A player who logs in on a map nobody stays on, such as a ship under way,
/// starts on the map's `forcedReturn` instead (Cosmic applies it on save).
/// One that is an event's map, which nobody logs in to either, sends them
/// on by its own: Orbis's treasure room returns to Minerva's prize room,
/// and that to the tower's exit; Pig Town and Ludibrium's bonus map pass
/// on the same way.
pub(super) fn leave_forced_return_map(player: &mut Player) {
    let Ok(data) = net::get_game_data() else {
        return;
    };
    let forced_return = |map_id: i32| {
        data.field(map_id)
            .and_then(|field| field.forced_return)
            .filter(|&to| to != map_id && data.field_exists(to))
    };
    if let Some(to) = login_map(player.stats.map_id, forced_return) {
        player.move_to(to, 0);
    }
}

/// Where a player logging in on `map_id` starts instead, given each map's
/// `forced_return`, if anywhere.
fn login_map(map_id: i32, forced_return: impl Fn(i32) -> Option<i32>) -> Option<i32> {
    let mut to = forced_return(map_id)?;
    for _ in 0..3 {
        match forced_return(to) {
            Some(next) if net::events::event_map(to) => to = next,
            _ => break,
        }
    }
    Some(to)
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
        portal: Portal,
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
        let actions = match portal {
            Portal::Named(name) => warp(&mut game.player, map_id, name, channel_id)?,
            Portal::Id(id) => warp_to_spawn_point(&mut game.player, map_id, id, channel_id)?,
        };
        for action in actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_login_passes_on_through_event_maps_to_their_way_out() {
        let forced_return = |map_id| match map_id {
            // A ship under way returns to its station, which keeps players.
            200_090_000 => Some(101_000_300),
            101_000_300 => Some(101_000_000),
            // Orbis's treasure room, Minerva's prize room, the tower's exit.
            920_011_100 => Some(920_011_300),
            920_011_300 => Some(920_011_200),
            920_011_200 => Some(200_080_101),
            _ => None,
        };
        assert_eq!(login_map(200_090_000, forced_return), Some(101_000_300));
        assert_eq!(login_map(920_011_100, forced_return), Some(920_011_200));
        assert_eq!(login_map(920_011_300, forced_return), Some(920_011_200));
        assert_eq!(login_map(100_000_000, forced_return), None);
    }
}
