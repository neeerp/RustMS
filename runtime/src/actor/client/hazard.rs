use super::ClientActor;
use crate::error::RuntimeError;
use game_data::Hazard;
use net::packet::handle::world::hurt_by_hazard;
use tokio::time::Instant;

/// A map that hurts the player on it now and then: when it next does.
pub(super) struct HazardTimer {
    map_id: i32,
    next: Instant,
    hazard: Hazard,
}

impl ClientActor {
    /// On a map with `info/decHP` the player is hurt once its interval
    /// has passed, and every interval after. Any other map clears it.
    pub(super) fn start_hazard(&mut self) -> Result<(), RuntimeError> {
        self.hazard = None;
        let map_id = self.in_game()?.player.stats.map_id;
        let Ok(data) = net::get_game_data() else {
            return Ok(());
        };
        let Some(hazard) = data.field(map_id).and_then(|field| field.hazard) else {
            return Ok(());
        };
        self.hazard = Some(HazardTimer {
            map_id,
            next: Instant::now() + hazard.every,
            hazard,
        });
        Ok(())
    }

    /// When the player's map next hurts them.
    pub(super) fn hazard_due(&self) -> Option<Instant> {
        self.hazard.as_ref().map(|timer| timer.next)
    }

    /// The map hurts the player, if they are still on it and out of the
    /// Cash Shop, and will again an interval later.
    pub(super) async fn hurt_by_map(&mut self) -> Result<(), RuntimeError> {
        let Some(timer) = self.hazard.as_mut() else {
            return Ok(());
        };
        timer.next += timer.hazard.every;
        let (map_id, hazard) = (timer.map_id, timer.hazard);
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id {
            return Ok(());
        }
        let result = hurt_by_hazard(&mut game.player, &hazard)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}
