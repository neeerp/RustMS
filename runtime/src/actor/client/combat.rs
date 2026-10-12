use super::quest::gain_exp;
use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::stat::{self, StatUpdate};

impl ClientActor {
    /// This player helped kill a mob: credit their share of its EXP.
    pub(super) async fn mob_killed(&mut self, _mob_id: i32, exp: i32) -> Result<(), RuntimeError> {
        if exp <= 0 {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        gain_exp(&mut next, exp, &mut packets)?;
        self.commit_player(next, None, &mut packets).await?;
        self.send_all(&mut packets).await
    }

    /// HP reached 0. Beginners lose nothing; others lose 10% of the EXP
    /// their level needs (v83's field death penalty).
    pub(super) async fn died(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        if player.stats.job % 1000 == 0 {
            return Ok(());
        }
        let Some(needed) = game::exp::to_next_level(player.stats.level) else {
            return Ok(());
        };
        let loss = (needed / 10) as i32;
        player.stats.exp = (player.stats.exp - loss).max(0);
        player.mark_dirty();
        let exp = player.stats.exp;
        let mut packet = stat::build_stat_update(false, &[StatUpdate::Exp(exp)])?;
        self.send(&mut packet).await
    }
}
