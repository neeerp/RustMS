use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::buff::{Buff, Buffs};
use game::BuffStat;
use net::packet::build::world::buff;
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::build::world::summon::Gone;
use net::packet::handle::world::{catch_disease, countered, party_skill_reached};
use net::summons::Support;
use net::{summons, FieldRequest, PartySkill, SpawnPosition};
use packet::Packet;

impl ClientActor {
    /// A party member's skill reached the player: if they are still on its
    /// map, out of the Cash Shop, take its buffs or HP.
    pub(super) async fn party_skill_reached(
        &mut self,
        used: PartySkill,
        origin: (i16, i16),
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id {
            return Ok(());
        }
        let at = game.position;
        let result = party_skill_reached(
            &mut game.player,
            &used,
            origin,
            at,
            std::time::Instant::now(),
        )
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// A mob's skill gave the player a disease, if they are still on its
    /// map and out of the Cash Shop.
    pub(super) async fn diseased(
        &mut self,
        disease: game::Disease,
        map_id: i32,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id {
            return Ok(());
        }
        let result = catch_disease(&mut game.player, disease, std::time::Instant::now())
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// Mobs the player attacked countered them, if they are still on its
    /// map and out of the Cash Shop.
    pub(super) async fn countered(&mut self, damage: i32, map_id: i32) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id {
            return Ok(());
        }
        let result = countered(&mut game.player, damage)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }

    /// When the player's buffs next need `update_buffs`.
    pub(super) fn buffs_due(&self) -> Option<tokio::time::Instant> {
        let deadline = self.game.as_ref()?.player.buffs.next_deadline()?;
        Some(tokio::time::Instant::from_std(deadline))
    }

    /// Heal from Recovery, hurt from Poison, and end the buffs and
    /// cooldowns that ran out.
    pub(super) async fn update_buffs(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let tick = player.buffs.advance(std::time::Instant::now());
        let mut packets = Vec::new();
        let stats = &player.stats;
        // Poison never takes the last HP.
        let poison = tick.poison.min(i32::from(stats.hp) - 1).max(0);
        let change = tick.heal - poison;
        if stats.hp > 0 && (change < 0 || (change > 0 && stats.hp < stats.max_hp)) {
            let hp = player.add_hp(change);
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
        self.end_buffs(Buffs::clear).await
    }

    /// A mob's Dispel ends the player's skills' buffs, if they are alive,
    /// still on its map and out of the Cash Shop. Diseases stay.
    pub(super) async fn dispelled(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        if game.player.stats.map_id != map_id || game.player.stats.hp <= 0 {
            return Ok(());
        }
        self.end_buffs(Buffs::dispel).await
    }

    /// End the buffs `end` ends, the summons with theirs.
    async fn end_buffs(
        &mut self,
        end: fn(&mut Buffs) -> Vec<BuffStat>,
    ) -> Result<(), RuntimeError> {
        let buffs = &mut self.in_game_mut()?.player.buffs;
        let summoned = buffs
            .active()
            .iter()
            .any(|buff| summons::kind(buff.skill_id).is_some());
        let ended = end(buffs);
        if ended.is_empty() {
            return Ok(());
        }
        if summoned {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Unsummon {
                    skill_id: None,
                    why: Gone::Dismissed,
                },
            })
            .await?;
        }
        let mut packet = buff::build_cancel_buff(&ended)?;
        self.send(&mut packet).await?;
        self.hide_buffs(&ended).await
    }

    /// End one buff, e.g. a Puppet's when a mob broke it.
    async fn end_buff(&mut self, skill_id: i32) -> Result<(), RuntimeError> {
        let ended = self.in_game_mut()?.player.buffs.cancel(skill_id);
        if ended.is_empty() {
            return Ok(());
        }
        let mut packet = buff::build_cancel_buff(&ended)?;
        self.send(&mut packet).await?;
        self.hide_buffs(&ended).await
    }

    pub(super) async fn summon_broken(&mut self, skill_id: i32) -> Result<(), RuntimeError> {
        self.end_buff(skill_id).await
    }

    /// The player's Beholder heals them, or buffs one stat, while they
    /// live.
    pub(super) async fn beholder_support(&mut self, support: Support) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        if player.stats.hp <= 0 {
            return Ok(());
        }
        let mut packet = match support {
            Support::Heal { hp, .. } => {
                let hp = player.add_hp(hp);
                stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?
            }
            Support::Hex {
                stat, value, lasts, ..
            } => {
                let now = std::time::Instant::now();
                let skill_id = support.skill_id();
                player
                    .buffs
                    .give(Buff::new(skill_id, stat, value, now, lasts));
                buff::build_give_buff(skill_id, lasts.as_millis() as i32, &[(stat, value)])?
            }
        };
        self.send(&mut packet).await
    }

    /// On a new map the summons that follow the player come along, at
    /// `spawn`; those that stay where they were put are gone.
    pub(super) async fn bring_summons(&mut self, spawn: SpawnPosition) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let now = std::time::Instant::now();
        let summoned: Vec<_> = player
            .buffs
            .active()
            .iter()
            .filter(|buff| summons::kind(buff.skill_id).is_some())
            .map(|buff| {
                let level = player.skills.level(buff.skill_id);
                (
                    buff.skill_id,
                    level,
                    buff.expires.saturating_duration_since(now),
                )
            })
            .collect();
        for (skill_id, level, lasts) in summoned {
            if summons::stationary(skill_id) {
                self.end_buff(skill_id).await?;
                continue;
            }
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Summon {
                    skill_id,
                    level,
                    at: (spawn.x, spawn.y),
                    left: false,
                    lasts,
                    appear: false,
                },
            })
            .await?;
        }
        Ok(())
    }
}
