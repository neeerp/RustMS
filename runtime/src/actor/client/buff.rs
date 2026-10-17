use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::BuffStat;
use net::packet::build::world::buff;
use net::packet::build::world::stat::{self, StatUpdate};
use packet::Packet;

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
        self.send_all(&mut packets).await?;
        self.hide_buffs(&tick.expired).await
    }

    /// Show the others on the map a change to the buffs they see, and keep
    /// the player's field avatar up to date for those who arrive later.
    pub(super) async fn show_buffs(&mut self, announce: Packet) -> Result<(), RuntimeError> {
        let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
        self.send_to_field(FieldMessage::UpdateAvatar {
            client_id: self.client_id,
            avatar,
            announce: Some(announce),
        })
        .await
    }

    /// Tell the others which of the buffs they saw ended.
    async fn hide_buffs(&mut self, ended: &[BuffStat]) -> Result<(), RuntimeError> {
        let unseen: Vec<_> = ended.iter().copied().filter(|stat| stat.shown()).collect();
        if unseen.is_empty() {
            return Ok(());
        }
        let id = self.in_game()?.player.id();
        let announce = buff::build_cancel_foreign_buff(id, &unseen)?;
        self.show_buffs(announce).await
    }

    /// End every buff now, e.g. on death. Buffs don't follow the player to
    /// another channel either.
    pub(super) async fn cancel_buffs(&mut self) -> Result<(), RuntimeError> {
        let ended = self.in_game_mut()?.player.buffs.clear();
        if ended.is_empty() {
            return Ok(());
        }
        let mut packet = buff::build_cancel_buff(&ended)?;
        self.send(&mut packet).await?;
        self.hide_buffs(&ended).await
    }
}
