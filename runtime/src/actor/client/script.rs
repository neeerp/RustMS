use super::npc_script::Applied;
use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat;
use net::scripts;
use tracing::debug;

impl ClientActor {
    /// Run the current map's `onUserEnter` script after joining it.
    pub(super) async fn run_map_enter_script(&mut self) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let Some(script) = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
            .and_then(|field| field.on_user_enter.clone())
        else {
            return Ok(());
        };
        match scripts::map_enter(&script, &self.in_game()?.player) {
            Some(effects) => self.apply_effects(&effects).await.map(drop),
            None => {
                debug!(map_id, script, "No native map entry script");
                Ok(())
            }
        }
    }

    /// The client touched a script portal on its map.
    pub(super) async fn touch_script_portal(&mut self, portal: &str) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let template = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
            .and_then(|field| field.portal_by_name(portal))
            .cloned();
        let script = template.as_ref().and_then(|portal| portal.script.clone());
        let opq = match (&script, &template) {
            (Some(script), Some(portal)) => self.opq_portal(script, portal).await?,
            _ => None,
        };
        if let Some(effects) = opq {
            self.apply_effects(&effects).await?;
        } else if let Some(script) = script.filter(|s| !self.blocked_scripts.contains(s)) {
            match scripts::portal(&script, &self.in_game()?.player) {
                Some(outcome) => {
                    if outcome.block {
                        self.blocked_scripts.insert(script);
                    }
                    let applied = self.apply_effects(&outcome.effects).await?;
                    if let (Applied::Refused, Some(text)) = (applied, outcome.occupied) {
                        let mut line = build_broadcast(Broadcast::Pink, &text)?;
                        self.send(&mut line).await?;
                    }
                }
                None => debug!(map_id, portal, script, "No native portal script"),
            }
        }
        // The client waits for this before it accepts the next portal.
        let mut enable = stat::build_stat_update(true, &[])?;
        self.send(&mut enable).await
    }

    /// The player used a scripted item whose script is `script`, where
    /// they stand.
    pub(super) async fn use_scripted_item(&mut self, script: &str) -> Result<(), RuntimeError> {
        let game = self.in_game()?;
        match scripts::item(script, &game.player, game.position) {
            Some(effects) => {
                self.apply_effects(&effects).await?;
            }
            None => debug!(script, "No native item script"),
        }
        // The client waits for this before it uses another item.
        let mut enable = stat::build_stat_update(true, &[])?;
        self.send(&mut enable).await
    }
}
