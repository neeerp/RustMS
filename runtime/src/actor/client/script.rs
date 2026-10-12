use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::{effect, stat};
use net::scripts::{self, ScriptEffect};
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
        match scripts::map_enter(&script) {
            Some(effects) => self.send_script_effects(&effects).await,
            None => {
                debug!(map_id, script, "No native map entry script");
                Ok(())
            }
        }
    }

    /// The client touched a script portal on its map.
    pub(super) async fn touch_script_portal(&mut self, portal: &str) -> Result<(), RuntimeError> {
        let map_id = self.in_game()?.player.stats.map_id;
        let script = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
            .and_then(|field| field.portal_by_name(portal))
            .and_then(|portal| portal.script.clone());
        if let Some(script) = script.filter(|s| !self.blocked_scripts.contains(s)) {
            match scripts::portal(&script, &self.in_game()?.player) {
                Some(outcome) => {
                    if outcome.block {
                        self.blocked_scripts.insert(script);
                    }
                    self.send_script_effects(&outcome.effects).await?;
                }
                None => debug!(map_id, portal, script, "No native portal script"),
            }
        }
        // The client waits for this before it accepts the next portal.
        let mut enable = stat::build_stat_update(true, &[])?;
        self.send(&mut enable).await
    }

    async fn send_script_effects(&mut self, effects: &[ScriptEffect]) -> Result<(), RuntimeError> {
        for effect in effects {
            let mut packet = match effect {
                ScriptEffect::FieldEffect(path) => effect::build_field_effect(path),
                ScriptEffect::AvatarEffect(path) => effect::build_avatar_effect(path),
                ScriptEffect::Hint {
                    text,
                    width,
                    height,
                } => effect::build_hint(text, *width, *height),
            }?;
            self.send(&mut packet).await?;
        }
        Ok(())
    }
}
