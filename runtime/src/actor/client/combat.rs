use super::quest::gain_exp_with_bonus;
use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::stat;
use net::packet::handle::world::{death_penalty, warp};

impl ClientActor {
    /// This player helped kill a mob, or shares in a party member's kill:
    /// credit their share of its EXP times the world's and their cards'
    /// rates and Holy Symbol's, half of it while cursed (Cosmic). v83 has no
    /// line for a rate, so the gain shows as plain EXP. A party bonus means
    /// the kill was shared.
    pub(super) async fn mob_killed(
        &mut self,
        _mob_id: i32,
        exp: i32,
        party_bonus: i32,
    ) -> Result<(), RuntimeError> {
        let mut next = self.in_game()?.player.clone();
        let rate = net::rates::exp(&next, std::time::SystemTime::now())
            * net::rates::holy_symbol(&next, party_bonus > 0);
        let scale = |exp: i32| (f64::from(exp) * rate).round().min(f64::from(i32::MAX)) as i32;
        let (exp, party_bonus) = (scale(exp), scale(party_bonus));
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

    /// EXP from outside a kill or quest, such as gacha EXP: the stat update
    /// shows it, with no message.
    pub(super) async fn gained_exp(&mut self, amount: i32) -> Result<(), RuntimeError> {
        let mut next = self.in_game()?.player.clone();
        let levelled = next.gain_exp(amount, net::skills::growth_bonus(&next, true)) > 0;
        let mut packets = vec![stat::build_exp_stats(&next.stats, levelled)?];
        self.commit_player(next, None, &mut packets).await?;
        self.send_all(&mut packets).await
    }

    /// HP reached 0: buffs end and the death penalty applies. The spawn
    /// point nearest is kept for a Wheel of Destiny revival.
    pub(super) async fn died(&mut self) -> Result<(), RuntimeError> {
        self.cancel_buffs().await?;
        self.remember_spawn_point();
        let player = &mut self.in_game_mut()?.player;
        let mut packets = death_penalty(player)?;
        self.send_all(&mut packets).await
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
