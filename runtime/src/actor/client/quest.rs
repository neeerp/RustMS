use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{DirectoryMessage, FieldMessage},
};
use game::{Player, QuestRecord, QuestState, QuestStatus};
use game_data::server_content::keys;
use net::npc_scripts::{NpcScript, FAMILY_QUEST_SCRIPTS};
use net::packet::build::world::effect::{self, CharEffect};
use net::packet::build::world::messaging::{build_broadcast, Broadcast};
use net::packet::build::world::{drop, inventory, pet, quest, skill, stat};
use net::packet::handle::world::{
    apply_item_effect, item_effect_packets, tame_first_pet, train_first_pet, ItemEffect,
};
use net::quests::{self, QuestActionKind, QuestItem, QuestRejection, QuestRequest, QuestTemplate};
use net::skills;
use net::FieldRequest;
use packet::Packet;
use std::time::{Instant, SystemTime};
use tracing::{debug, warn};

/// A quest change to commit together with the player.
#[derive(Clone, Copy)]
pub(super) enum QuestChange {
    Start(&'static QuestTemplate),
    Complete(&'static QuestTemplate),
    /// Started and completed at once, by a start script.
    Finish(&'static QuestTemplate),
}

impl ClientActor {
    /// Check a `QUEST_ACTION` against the catalog and the player, then ask the
    /// field whether the NPC is near before anything is persisted, unless the
    /// quest alert offers the step.
    pub(super) async fn request_quest_action(
        &mut self,
        request: QuestRequest,
    ) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        let catalog = quests::catalog().map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let Some(template) = catalog.quest(request.quest_id) else {
            debug!(
                quest_id = request.quest_id,
                "Quest action for an unsupported quest"
            );
            return Ok(());
        };
        // Scripted steps must go through their script (actions 4/5), and
        // only scripted steps may.
        let (npc_id, check) = match request.kind {
            QuestActionKind::Start { npc_id } if template.start_script.is_none() => {
                (npc_id, quests::check_start(template, npc_id, player))
            }
            QuestActionKind::ScriptStart { npc_id } if template.start_script.is_some() => {
                (npc_id, quests::check_start(template, npc_id, player))
            }
            QuestActionKind::Complete { npc_id, .. } if template.end_script.is_none() => {
                (npc_id, quests::check_complete(template, npc_id, player))
            }
            QuestActionKind::ScriptEnd { npc_id } if template.end_script.is_some() => {
                (npc_id, quests::check_complete(template, npc_id, player))
            }
            QuestActionKind::Forfeit => return self.forfeit_quest(request.quest_id).await,
            _ => {
                debug!(quest_id = request.quest_id, ?request.kind, "Wrong quest action kind");
                return Ok(());
            }
        };
        if let Err(rejection) = check {
            debug!(
                quest_id = request.quest_id,
                ?rejection,
                "Quest action rejected"
            );
            if rejection == QuestRejection::Mesos {
                let mut packets = vec![quest::build_quest_short_of_mesos()?];
                return self.send_all(&mut packets).await;
            }
            return Ok(());
        }
        self.npc_request_id = self.npc_request_id.wrapping_add(1);
        // The quest alert starts autoStart quests and completes autoComplete
        // and autoPreComplete ones wherever the player is.
        let anywhere = match request.kind {
            QuestActionKind::Start { .. } | QuestActionKind::ScriptStart { .. } => {
                template.auto_start
            }
            _ => template.auto_complete || template.auto_pre_complete,
        };
        self.pending_quest = Some((self.npc_request_id, request));
        if anywhere {
            return self.quest_npc_checked(self.npc_request_id, true).await;
        }
        self.send_to_field(FieldMessage::QuestNpc {
            from: self.client_id,
            npc_id,
            request_id: self.npc_request_id,
        })
        .await
    }

    /// The field answered; run the pending action if it is still current.
    pub(super) async fn quest_npc_checked(
        &mut self,
        request_id: u64,
        nearby: bool,
    ) -> Result<(), RuntimeError> {
        let Some((pending_id, request)) = self.pending_quest else {
            return Ok(());
        };
        if pending_id != request_id {
            return Ok(());
        }
        self.pending_quest = None;
        if !nearby {
            debug!(quest_id = request.quest_id, "Quest NPC is not nearby");
            return Ok(());
        }
        let catalog = quests::catalog().map_err(|e| RuntimeError::Handler(e.to_string()))?;
        let Some(template) = catalog.quest(request.quest_id) else {
            return Ok(());
        };
        match request.kind {
            QuestActionKind::Start { npc_id } => self.start_quest(template, npc_id).await,
            QuestActionKind::Complete { npc_id, choice } => {
                self.complete_quest(template, npc_id, choice).await
            }
            QuestActionKind::ScriptStart { .. } => {
                self.run_quest_script(template.start_script.as_deref())
                    .await
            }
            QuestActionKind::ScriptEnd { .. } => {
                self.run_quest_script(template.end_script.as_deref()).await
            }
            QuestActionKind::Forfeit => Ok(()),
        }
    }

    async fn run_quest_script(&mut self, script: Option<&str>) -> Result<(), RuntimeError> {
        let Some(script) = script else {
            return Ok(());
        };
        // Captain Al's and Dalair's Outstanding Citizen read the Family
        // window's numbers.
        let cid = self.client_id;
        let family = if FAMILY_QUEST_SCRIPTS.contains(&script) {
            Some(spawn_db(move || db::family::info(cid)).await?)
        } else {
            None
        };
        let player = &self.in_game()?.player;
        let script = match family {
            Some(family) => NpcScript::for_family_quest(script, player, &family),
            None => NpcScript::for_quest(script, player),
        };
        match script {
            Some(script) => self.start_script(script).await,
            None => Ok(()),
        }
    }

    async fn start_quest(
        &mut self,
        template: &'static QuestTemplate,
        npc_id: i32,
    ) -> Result<(), RuntimeError> {
        // Recheck: the player may have changed while the field answered.
        if quests::check_start(template, npc_id, &self.in_game()?.player).is_err() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if !self.quest_items(
            template.id,
            &template.start_items,
            -1,
            &mut next,
            &mut packets,
        )? {
            return self.send_all(&mut packets).await;
        }
        pay_mesos(&mut next, template.start_money, &mut packets)?;
        if template.start_exp > 0 {
            gain_exp(&mut next, template.start_exp, &mut packets)?;
        }
        let buff = buff_item(&mut next, template.start_buff_item);
        if !self
            .commit_player(next, Some(QuestChange::Start(template)), &mut packets)
            .await?
        {
            return Ok(());
        }
        let seen = self.show_buff_item(buff, &mut packets)?;
        packets.push(quest::build_quest_action_result(template.id, npc_id, 0)?);
        self.send_all(&mut packets).await?;
        self.show_seen(seen).await
    }

    async fn complete_quest(
        &mut self,
        template: &'static QuestTemplate,
        npc_id: i32,
        choice: i32,
    ) -> Result<(), RuntimeError> {
        if quests::check_complete(template, npc_id, &self.in_game()?.player).is_err() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if !self.quest_items(
            template.id,
            &template.end_rewards,
            choice,
            &mut next,
            &mut packets,
        )? {
            return self.send_all(&mut packets).await;
        }
        pay_mesos(&mut next, template.money, &mut packets)?;
        if template.fame != 0 {
            next.stats.fame = next.stats.fame.saturating_add(template.fame);
            packets.push(stat::build_stat_update(
                false,
                &[stat::StatUpdate::Fame(next.stats.fame)],
            )?);
            packets.push(quest::build_fame_gain_in_chat(template.fame)?);
        }
        if template.exp > 0 {
            gain_exp(&mut next, template.exp, &mut packets)?;
        }
        // With no pet out, the closeness is lost, as in Cosmic.
        let mut tamed = false;
        if template.pet_closeness != 0 {
            if let Some((changes, levelled)) = tame_first_pet(&mut next, template.pet_closeness) {
                if !changes.is_empty() {
                    packets.push(inventory::build_inventory_operation(true, &changes)?);
                }
                if levelled {
                    packets.push(pet::build_pet_level_up(None, 0)?);
                    tamed = true;
                }
            }
        }
        let trained = train_first_pet(&mut next, template.pet_skill, template.pet_speed);
        if !trained.is_empty() {
            packets.push(inventory::build_inventory_operation(true, &trained)?);
        }
        let taught = quests::apply_skills(&template.end_skills, &mut next);
        let buff = buff_item(&mut next, template.end_buff_item);
        if !self
            .commit_player(next, Some(QuestChange::Complete(template)), &mut packets)
            .await?
        {
            return Ok(());
        }
        let seen = self.show_buff_item(buff, &mut packets)?;
        // Only continue into a quest the client can actually be offered.
        let follow_up = template
            .next_quest
            .filter(|id| quests::catalog().is_ok_and(|c| c.quest(*id).is_some()))
            .unwrap_or(0);
        packets.push(quest::build_quest_action_result(
            template.id,
            npc_id,
            follow_up,
        )?);
        packets.push(effect::build_own_effect(CharEffect::QuestClear)?);
        if !taught.is_empty() {
            packets.push(skill::build_update_skills(&taught)?);
        }
        self.send_all(&mut packets).await?;
        self.show_seen(seen).await?;
        let character_id = self.in_game()?.player.id();
        if tamed {
            let shown = pet::build_pet_level_up(Some(character_id), 0)?;
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request: FieldRequest::Broadcast(shown),
            })
            .await?;
        }
        let shown = effect::build_foreign_effect(character_id, CharEffect::QuestClear)?;
        self.send_to_field(FieldMessage::FromClient {
            from: self.client_id,
            request: FieldRequest::Broadcast(shown),
        })
        .await?;
        match template.end_npc_act.as_deref() {
            Some(action) => {
                self.send_to_field(FieldMessage::FromClient {
                    from: self.client_id,
                    request: FieldRequest::NpcSpecialAction { npc_id, action },
                })
                .await
            }
            None => Ok(()),
        }
    }

    /// Queue what a quest's `buffItemID` did for the player; return what the
    /// others on the map see.
    fn show_buff_item(
        &mut self,
        buff: Option<(i32, ItemEffect)>,
        packets: &mut Vec<Packet>,
    ) -> Result<Vec<Packet>, RuntimeError> {
        let Some((item_id, effect)) = buff else {
            return Ok(Vec::new());
        };
        let (own, seen) =
            item_effect_packets(&mut self.in_game_mut()?.player, item_id, effect, false)?;
        packets.extend(own);
        Ok(seen)
    }

    async fn show_seen(&mut self, seen: Vec<Packet>) -> Result<(), RuntimeError> {
        for packet in seen {
            self.show_buffs(packet).await?;
        }
        Ok(())
    }

    /// Apply a quest step's item actions, with the reward picked as
    /// `choice`, to `next`. `false` if the pick isn't offered or, with the
    /// "inventory full" notice queued, if the gifts don't fit.
    fn quest_items(
        &self,
        quest_id: i16,
        items: &[QuestItem],
        choice: i32,
        next: &mut Player,
        packets: &mut Vec<Packet>,
    ) -> Result<bool, RuntimeError> {
        let picked = match quests::pick_items(items, next, choice, &mut rand::thread_rng()) {
            Ok(picked) => picked,
            Err(rejection) => {
                debug!(?rejection, "Quest reward refused");
                return Ok(false);
            }
        };
        let changes = match quests::apply_items(next, &picked, SystemTime::now()) {
            Ok(changes) => changes,
            Err(rejection) => {
                debug!(?rejection, "Quest items refused");
                match rejection {
                    QuestRejection::InventoryFull => {
                        packets.push(quest::build_quest_items_full(quest_id)?)
                    }
                    QuestRejection::OnlyOne => packets.push(quest::build_quest_only_one()?),
                    _ => {}
                }
                return Ok(false);
            }
        };
        if !changes.is_empty() {
            packets.push(inventory::build_inventory_operation(true, &changes)?);
        }
        for (item_id, count, _) in picked {
            packets.push(inventory::build_item_gain_in_chat(item_id, count)?);
        }
        Ok(true)
    }

    /// Count a kill toward started quests, and set the records it sets,
    /// and show the new progress.
    pub(super) async fn count_quest_kill(&mut self, mob_id: i32) -> Result<(), RuntimeError> {
        let Ok(catalog) = quests::catalog() else {
            return Ok(());
        };
        let player = &mut self.in_game_mut()?.player;
        let changed = quests::count_kill(|id| catalog.quest(id), player, mob_id);
        let records = quests::kill_records(player, mob_id);
        if changed.is_empty() && records.is_empty() {
            return Ok(());
        }
        player.mark_dirty();
        let mut packets = Vec::new();
        for quest_id in changed {
            let status = player.quests.status(quest_id);
            packets.push(quest::build_quest_progress(quest_id, status)?);
        }
        for (record, value) in records {
            packets.push(quest::build_quest_record(record, value)?);
        }
        self.send_all(&mut packets).await
    }

    /// Adopt `next` as the player. With a quest change, the change and the
    /// whole player commit together first; if the database refuses (e.g. a
    /// replayed completion), nothing is adopted and `false` is returned. The
    /// quest record update goes first in `packets`.
    pub(super) async fn commit_player(
        &mut self,
        mut next: Player,
        change: Option<QuestChange>,
        packets: &mut Vec<Packet>,
    ) -> Result<bool, RuntimeError> {
        let previous_level = self.in_game()?.player.stats.level;
        let previous_job = self.in_game()?.player.stats.job;
        if let Some(QuestChange::Start(template)) = change {
            // Hunting quests start with every kill count at 0 ("000"), info
            // quests with their first info string, unless a script set the
            // string along with the start (`Effect::SetQuestInfo`).
            let preset = next.quests.state(template.id) == QuestState::Started
                && self.in_game()?.player.quests.state(template.id) != QuestState::Started;
            if !preset {
                next.quests.upsert(QuestRecord {
                    quest_id: template.id,
                    status: QuestStatus::Started {
                        kills: vec![0; template.end_mobs.len()],
                        info: template.start_info.clone(),
                    },
                });
            }
        }
        if let Some(change) = change {
            let snapshot = next.clone();
            let progress = match change {
                QuestChange::Start(template) | QuestChange::Finish(template) => {
                    let prerequisites: Vec<_> = template
                        .start
                        .quests
                        .iter()
                        .map(|p| (p.quest_id, p.state))
                        .collect();
                    let repeatable = template.start.repeat_interval.is_some();
                    let begin = match change {
                        QuestChange::Finish(_) => db::quest::finish,
                        _ => db::quest::start,
                    };
                    spawn_db(move || begin(&snapshot, template.id, &prerequisites, repeatable))
                        .await
                }
                QuestChange::Complete(template) => {
                    spawn_db(move || db::quest::complete(&snapshot, template.id)).await
                }
            };
            let progress = match progress {
                Ok(progress) => progress,
                Err(RuntimeError::Database(db::Error::NotFound)) => {
                    warn!(self.client_id, "Quest change refused by the database");
                    return Ok(false);
                }
                Err(error) => return Err(error),
            };
            let record = db::player::quest_record(&progress);
            packets.insert(
                0,
                quest::build_quest_progress(record.quest_id, Some(&record.status))?,
            );
            next.quests.upsert(record);
            next.mark_saved();
            match change {
                QuestChange::Start(template) => self.start_quest_clock(template.id, packets)?,
                QuestChange::Complete(template) | QuestChange::Finish(template) => {
                    self.end_quest_clock(template.id, packets)?
                }
            }
        } else {
            next.mark_dirty();
        }
        let levelled = next.stats.level > previous_level;
        let advanced = next.stats.job != previous_job;
        self.in_game_mut()?.player = next;
        if levelled || advanced {
            // Others see the level-up or job-change effect; the field avatar
            // shows the level and job. The player's own client plays them
            // from the stat change.
            let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
            let client_id = self.client_id;
            let mut announce = Vec::new();
            if levelled {
                announce.push(effect::build_foreign_effect(
                    client_id,
                    CharEffect::LevelUp,
                )?);
            }
            if advanced {
                announce.push(effect::build_foreign_effect(
                    client_id,
                    CharEffect::JobChange,
                )?);
            }
            self.send_to_field(FieldMessage::UpdateAvatar {
                client_id,
                avatar,
                announce,
            })
            .await?;
        }
        if levelled {
            self.announce_level_200(previous_level).await?;
            let levels = self.in_game()?.player.stats.level - previous_level;
            self.family_earn(game::family::REP_PER_LEVEL * i32::from(levels))
                .await?;
        }
        Ok(true)
    }

    /// Everyone in the world hears of a player reaching level 200, but not
    /// of a GM (Cosmic; OrionAlpha tells the channel).
    async fn announce_level_200(&mut self, previous_level: u8) -> Result<(), RuntimeError> {
        let player = &self.in_game()?.player;
        if previous_level >= 200 || player.stats.level < 200 || player.gm_level > 0 {
            return Ok(());
        }
        let text = net::content::text().get(keys::LEVEL_200, &[("name", &player.stats.name)]);
        self.send_to_directory(DirectoryMessage::Broadcast {
            from: self.client_id,
            scope: net::BroadcastScope::World,
            packet: build_broadcast(Broadcast::Notice, &text)?,
        })
        .await
    }

    async fn forfeit_quest(&mut self, quest_id: i16) -> Result<(), RuntimeError> {
        let mut packets = self.drop_quest(quest_id).await?;
        self.end_quest_clock(quest_id, &mut packets)?;
        self.send_all(&mut packets).await
    }

    /// Forfeit `quest_id` if started; the packets that show it.
    pub(super) async fn drop_quest(&mut self, quest_id: i16) -> Result<Vec<Packet>, RuntimeError> {
        let dropped = forfeit(&mut self.in_game_mut()?.player, quest_id).await?;
        dropped
            .into_iter()
            .map(|id| Ok(quest::build_quest_progress(id, None)?))
            .collect()
    }

    pub(super) async fn send_all(&mut self, packets: &mut [Packet]) -> Result<(), RuntimeError> {
        for packet in packets {
            self.send(packet).await?;
        }
        Ok(())
    }
}

/// Forfeit `player`'s `quest_id` if started, and the records it reads, or
/// a new start would find them set. The quests that went, the quest first.
pub(super) async fn forfeit(player: &mut Player, quest_id: i16) -> Result<Vec<i16>, RuntimeError> {
    if player.quests.state(quest_id) != QuestState::Started {
        return Ok(Vec::new());
    }
    let cid = player.id();
    let records: Vec<i16> = game_data::quests::RECORD_QUESTS
        .iter()
        .filter(|&&(quest, _)| quest == quest_id)
        .map(|&(_, record)| record)
        .collect();
    let deleted = records.clone();
    match spawn_db(move || db::quest::forfeit(cid, quest_id, &deleted)).await {
        // Already gone from the database: it goes from the player too.
        Ok(()) | Err(RuntimeError::Database(db::Error::NotFound)) => {}
        Err(error) => return Err(error),
    }
    let dropped: Vec<i16> = std::iter::once(quest_id).chain(records).collect();
    for &id in &dropped {
        player.quests.remove(id);
    }
    Ok(dropped)
}

/// Give `next` the effect of a quest step's `buffItemID`, if any.
fn buff_item(next: &mut Player, item_id: Option<i32>) -> Option<(i32, ItemEffect)> {
    let item_id = item_id?;
    let effect = apply_item_effect(next, item_id, Instant::now());
    if effect.is_none() {
        warn!(item_id, "Quest buff item has no supported effect");
    }
    Some((item_id, effect?))
}

/// Pay a quest step's `money` to `next`, or take it if negative, with the
/// stat update and the message in the chat log.
pub(super) fn pay_mesos(
    next: &mut Player,
    money: i32,
    packets: &mut Vec<Packet>,
) -> Result<(), RuntimeError> {
    if money == 0 {
        return Ok(());
    }
    next.meso = next.meso.saturating_add(money).max(0);
    packets.push(stat::build_stat_update(
        false,
        &[stat::StatUpdate::Meso(next.meso)],
    )?);
    packets.push(drop::build_meso_gain_in_chat(money)?);
    Ok(())
}

/// Add quest or script EXP to `next`, with the stat update and the EXP
/// message in the chat log.
pub(super) fn gain_exp(
    next: &mut Player,
    amount: i32,
    packets: &mut Vec<Packet>,
) -> Result<(), RuntimeError> {
    let bonus = skills::growth_bonus(next, true);
    let levelled = next.gain_exp(amount, bonus, skills::growth_roll) > 0;
    packets.push(stat::build_exp_stats(&next.stats, levelled)?);
    packets.push(quest::build_gain_exp_in_chat(amount)?);
    Ok(())
}

/// Add kill EXP to `next`, with the stat update and the EXP message on the
/// right; the party bonus shows on its own line.
pub(super) fn gain_exp_with_bonus(
    next: &mut Player,
    amount: i32,
    party_bonus: i32,
    packets: &mut Vec<Packet>,
) -> Result<(), RuntimeError> {
    let bonus = skills::growth_bonus(next, true);
    let levelled = next.gain_exp(
        amount.saturating_add(party_bonus),
        bonus,
        skills::growth_roll,
    ) > 0;
    packets.push(stat::build_exp_stats(&next.stats, levelled)?);
    packets.push(quest::build_gain_exp(amount, party_bonus)?);
    Ok(())
}
