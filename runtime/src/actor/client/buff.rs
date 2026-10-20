use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::FieldMessage;
use game::buff::{Buff, Buffs};
use game::inventory::equip_slot;
use game::{BuffStat, InventoryType};
use net::packet::build::world::buff;
use net::packet::build::world::messaging::build_item_line;
use net::packet::build::world::stat::{self, StatUpdate};
use net::packet::build::world::summon::Gone;
use net::packet::handle::world::{absorbed, catch_disease, countered, party_skill_reached};
use net::summons::Support;
use net::{summons, FieldRequest, PartySkill, SpawnPosition};
use packet::Packet;

/// What a player is told when their mount tires them off it.
const MOUNT_TIRED: &str =
    "Your mount grew tired! Treat it some revitalizer before riding it again!";

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
    /// map, out of the Cash Shop, and not under Holy Shield, which keeps off
    /// every disease a mob's skill brings.
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
        if game.player.stats.map_id != map_id || game.player.buffs.has(BuffStat::HolyShield) {
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

    /// The player's attack took HP and MP from mobs, if they are still on
    /// its map and out of the Cash Shop.
    pub(super) async fn absorbed(
        &mut self,
        hp: i32,
        mp: i32,
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
        let result =
            absorbed(&mut game.player, hp, mp).map_err(|e| RuntimeError::Handler(e.to_string()))?;
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

    /// Heal from Recovery, hurt from Poison and Dragon Blood, tire a ridden
    /// mount, and end the buffs and cooldowns that ran out; a mount tired
    /// out ends the ride.
    pub(super) async fn update_buffs(&mut self) -> Result<(), RuntimeError> {
        let player = &mut self.in_game_mut()?.player;
        let mut tick = player.buffs.advance(std::time::Instant::now());
        let mut packets = Vec::new();
        let stats = &player.stats;
        // Resistance takes its share off Poison, which never takes the last
        // HP, and Achilles takes its share off, as a v83 client works it out
        // (0x94b075, 0x94b113).
        let poison = net::combat::poison_resisted(player, tick.poison);
        let poison = poison.min(i32::from(stats.hp) - 1).max(0);
        let poison = net::combat::taken(player, poison, false, false).hp;
        let mut change = tick.heal - poison;
        // Dragon Blood drinks its HP while the player has more, and ends
        // rather than take their last (its description; the client leaves
        // it to the server).
        let bled = tick.blood > 0 && i32::from(stats.hp) + change > tick.blood;
        if bled {
            change -= tick.blood;
        } else if tick.blood > 0 {
            tick.expired
                .extend(player.buffs.cancel(net::skills::DRAGON_BLOOD));
        }
        let mut tired = false;
        for _ in 0..tick.tire {
            tired |= player.mount.tire();
        }
        if tired {
            if let Some(ride) = player.ride_skill() {
                tick.expired.extend(player.buffs.cancel(ride));
            }
        }
        let stats = &player.stats;
        let below_max = i32::from(stats.hp) < player.effective_max_hp();
        if stats.hp > 0 && (change < 0 || (change > 0 && below_max)) {
            let hp = player.add_hp(change);
            packets.push(stat::build_stat_update(false, &[StatUpdate::Hp(hp)])?);
        }
        if bled {
            packets.push(buff::build_own_special(net::skills::DRAGON_BLOOD)?);
        }
        if !tick.expired.is_empty() {
            packets.push(buff::build_cancel_buff(&tick.expired)?);
            if player.fit_hp_mp() {
                packets.push(stat::build_hp_mp(&player.stats)?);
            }
        }
        for skill_id in tick.cooled {
            packets.push(buff::build_cooldown(skill_id, 0)?);
        }
        let id = player.id();
        // The map sees the mount tire (Cosmic's runTirednessSchedule).
        let tiring = if tick.tire > 0 {
            let info = buff::build_taming_mob_info(id, &player.mount, false)?;
            packets.push(info.clone());
            Some(info)
        } else {
            None
        };
        if tired {
            packets.push(build_item_line(MOUNT_TIRED, 0)?);
        }
        self.send_all(&mut packets).await?;
        if let Some(info) = tiring {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Broadcast(info),
            })
            .await?;
        }
        if bled {
            let shown = buff::build_foreign_special(id, net::skills::DRAGON_BLOOD)?;
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Broadcast(shown),
            })
            .await?;
        }
        self.hide_buffs(&tick.expired).await
    }

    /// Show the others on the map a change to the buffs they see, and keep
    /// the player's field avatar up to date for those who arrive later.
    pub(super) async fn show_buffs(&mut self, announce: Packet) -> Result<(), RuntimeError> {
        let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
        self.send_to_field(FieldMessage::UpdateAvatar {
            client_id: self.client_id,
            avatar,
            announce: vec![announce],
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
        self.send_cancelled(&ended).await?;
        self.hide_buffs(&ended).await
    }

    /// Tell the player their buffs among `ended` ended, and bring HP and MP
    /// down to the max they left.
    async fn send_cancelled(&mut self, ended: &[BuffStat]) -> Result<(), RuntimeError> {
        let mut packets = vec![buff::build_cancel_buff(ended)?];
        let player = &mut self.in_game_mut()?.player;
        if player.fit_hp_mp() {
            packets.push(stat::build_hp_mp(&player.stats)?);
        }
        self.send_all(&mut packets).await
    }

    /// End one buff, e.g. a Puppet's when a mob broke it.
    async fn end_buff(&mut self, skill_id: i32) -> Result<(), RuntimeError> {
        let ended = self.in_game_mut()?.player.buffs.cancel(skill_id);
        if ended.is_empty() {
            return Ok(());
        }
        self.send_cancelled(&ended).await?;
        self.hide_buffs(&ended).await
    }

    /// Get the player off their mount before they enter a map that keeps
    /// mounts off (Cosmic's `MapleMap.addPlayer`).
    pub(super) async fn keep_mounts_off(&mut self, map_id: i32) -> Result<(), RuntimeError> {
        let Some(skill_id) = self.in_game()?.player.ride_skill() else {
            return Ok(());
        };
        let field_limit = net::get_game_data()
            .ok()
            .and_then(|data| data.field(map_id))
            .map_or(0, |field| field.field_limit);
        if field_limit & game_data::FIELD_LIMIT_MOUNTS == 0 {
            return Ok(());
        }
        self.end_buff(skill_id).await
    }

    /// A beacon's mark goes with the map: the next map's mobs reuse its
    /// object ids (Cosmic's PlayerMapTransitionHandler).
    pub(super) async fn drop_beacon(&mut self) -> Result<(), RuntimeError> {
        if self.in_game_mut()?.player.buffs.beacon.take().is_none() {
            return Ok(());
        }
        self.send(&mut buff::build_cancel_buff(&[BuffStat::GuidedBullet])?)
            .await
    }

    /// The player's own client learns their mount's state when they come
    /// in wearing one (Cosmic's PlayerLoggedinHandler).
    pub(super) async fn show_mount(&mut self) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if player
            .inventory
            .get(InventoryType::Equipped, equip_slot::MOUNT)
            .is_none()
        {
            return Ok(());
        }
        let mut packet = buff::build_taming_mob_info(player.id(), &player.mount, false)?;
        self.send(&mut packet).await
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
