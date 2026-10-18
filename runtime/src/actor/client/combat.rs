use super::quest::gain_exp_with_bonus;
use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::handle::world::warp;

impl ClientActor {
    /// This player helped kill a mob, or shares in a party member's kill:
    /// credit their share of its EXP, half of it while cursed (Cosmic).
    pub(super) async fn mob_killed(
        &mut self,
        _mob_id: i32,
        exp: i32,
        party_bonus: i32,
    ) -> Result<(), RuntimeError> {
        let mut next = self.in_game()?.player.clone();
        let (exp, party_bonus) = if next.buffs.has(game::BuffStat::Curse) {
            (exp / 2, party_bonus / 2)
        } else {
            (exp, party_bonus)
        };
        if exp <= 0 {
            return Ok(());
        }
        let mut packets = Vec::new();
        gain_exp_with_bonus(&mut next, exp, party_bonus, &mut packets)?;
        self.commit_player(next, None, &mut packets).await?;
        self.send_all(&mut packets).await
    }

    /// HP reached 0: buffs end. Beginners lose nothing; others lose 10% of the EXP
    /// their level needs (v83's field death penalty).
    pub(super) async fn died(&mut self) -> Result<(), RuntimeError> {
        self.cancel_buffs().await?;
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

impl ClientActor {
    /// A mob's Banish sends the player to `to`, telling them why in pink
    /// (Cosmic's changeMapBanish), if they are alive, still on its map and
    /// out of the Cash Shop.
    pub(super) async fn banished(
        &mut self,
        to: game_data::mobs::Banish,
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let channel_id = self.channel_id;
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id || game.player.stats.hp <= 0 {
            return Ok(());
        }
        let actions = warp(&mut game.player, to.map_id, &to.portal, channel_id)?;
        if !to.message.is_empty() {
            let mut told = build_broadcast(Broadcast::Pink, &to.message)?;
            self.send(&mut told).await?;
        }
        for action in actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}
