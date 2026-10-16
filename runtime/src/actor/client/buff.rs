use super::ClientActor;
use crate::error::RuntimeError;
use net::packet::build::world::buff;
use net::packet::build::world::stat::{self, StatUpdate};

impl ClientActor {
    /// When the player's buffs next need `update_buffs`.
    pub(super) fn buffs_due(&self) -> Option<tokio::time::Instant> {
        let deadline = self.game.as_ref()?.player.buffs.next_deadline()?;
        Some(tokio::time::Instant::from_std(deadline))
    }

    /// Heal from Recovery and end the buffs and cooldowns that ran out.
    pub(super) async fn update_buffs(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let tick = player.buffs.advance(std::time::Instant::now());
        let mut packets = Vec::new();
        let stats = &player.stats;
        if tick.heal > 0 && stats.hp > 0 && stats.hp < stats.max_hp {
            let hp = player.add_hp(tick.heal);
            packets.push(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?);
        }
        if !tick.expired.is_empty() {
            packets.push(buff::build_cancel_buff(&tick.expired)?);
        }
        for skill_id in tick.cooled {
            packets.push(buff::build_cooldown(skill_id, 0)?);
        }
        self.send_all(&mut packets).await
    }

    /// End every buff now, e.g. on death. Buffs don't follow the player to
    /// another channel either.
    pub(super) async fn cancel_buffs(&mut self) -> Result<(), RuntimeError> {
        let ended = self.in_game_mut()?.player.buffs.clear();
        if ended.is_empty() {
            return Ok(());
        }
        let mut packet = buff::build_cancel_buff(&ended)?;
        self.send(&mut packet).await
    }
}
