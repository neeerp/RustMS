use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::FieldMessage};
use game::{Player, QuestRecord, QuestState, QuestStatus};
use net::npc_scripts::NpcScript;
use net::packet::build::world::effect::{self, CharEffect};
use net::packet::build::world::{drop, inventory, quest, stat};
use net::quests::{self, QuestActionKind, QuestItem, QuestRejection, QuestRequest, QuestTemplate};
use net::FieldRequest;
use packet::Packet;
use tracing::{debug, warn};

/// A quest change to commit together with the player.
#[derive(Clone, Copy)]
pub(super) enum QuestChange {
    Start(&'static QuestTemplate),
    Complete(&'static QuestTemplate),
}

impl ClientActor {
    /// Check a `QUEST_ACTION` against the catalog and the player, then ask the
    /// field whether the NPC is near before anything is persisted.
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
        self.pending_quest = Some((self.npc_request_id, request));
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
            QuestActionKind::Complete { npc_id, .. } => self.complete_quest(template, npc_id).await,
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
        let player = &self.in_game()?.player;
        match script.and_then(|script| NpcScript::for_quest(script, player)) {
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
        if !self.quest_items(template.id, &template.start_items, &mut next, &mut packets)? {
            return self.send_all(&mut packets).await;
        }
        if self
            .commit_player(next, Some(QuestChange::Start(template)), &mut packets)
            .await?
        {
            packets.push(quest::build_quest_action_result(template.id, npc_id, 0)?);
        }
        self.send_all(&mut packets).await
    }

    async fn complete_quest(
        &mut self,
        template: &'static QuestTemplate,
        npc_id: i32,
    ) -> Result<(), RuntimeError> {
        if quests::check_complete(template, npc_id, &self.in_game()?.player).is_err() {
            return Ok(());
        }
        let mut next = self.in_game()?.player.clone();
        let mut packets = Vec::new();
        if !self.quest_items(template.id, &template.end_rewards, &mut next, &mut packets)? {
            return self.send_all(&mut packets).await;
        }
        if template.money != 0 {
            next.meso = next.meso.saturating_add(template.money).max(0);
            packets.push(stat::build_stat_update(
                false,
                &[stat::StatUpdate::Meso(next.meso)],
            )?);
            packets.push(drop::build_meso_gain_in_chat(template.money)?);
        }
        if template.exp > 0 {
            gain_exp(&mut next, template.exp, &mut packets)?;
        }
        if !self
            .commit_player(next, Some(QuestChange::Complete(template)), &mut packets)
            .await?
        {
            return Ok(());
        }
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
        self.send_all(&mut packets).await?;
        let character_id = self.in_game()?.player.id();
        let shown = effect::build_foreign_effect(character_id, CharEffect::QuestClear)?;
        self.send_to_field(FieldMessage::FromClient {
            from: self.client_id,
            request: FieldRequest::Broadcast(shown),
        })
        .await
    }

    /// Apply a quest step's item actions to `next`. `false`, with the
    /// "inventory full" notice queued, if the gifts don't fit.
    fn quest_items(
        &self,
        quest_id: i16,
        items: &[QuestItem],
        next: &mut Player,
        packets: &mut Vec<Packet>,
    ) -> Result<bool, RuntimeError> {
        let picked = quests::pick_items(items, next, &mut rand::thread_rng());
        let changes = match quests::apply_items(next, &picked) {
            Ok(changes) => changes,
            Err(rejection) => {
                debug!(?rejection, "Quest items refused");
                if rejection == QuestRejection::InventoryFull {
                    packets.push(quest::build_quest_items_full(quest_id)?);
                }
                return Ok(false);
            }
        };
        if !changes.is_empty() {
            packets.push(inventory::build_inventory_operation(true, &changes)?);
        }
        for (item_id, count) in picked {
            packets.push(inventory::build_item_gain_in_chat(item_id, count)?);
        }
        Ok(true)
    }

    /// Count a kill toward started quests and show the new progress.
    pub(super) async fn count_quest_kill(&mut self, mob_id: i32) -> Result<(), RuntimeError> {
        let Ok(catalog) = quests::catalog() else {
            return Ok(());
        };
        let player = &mut self.in_game_mut()?.player;
        let changed = quests::count_kill(|id| catalog.quest(id), player, mob_id);
        if changed.is_empty() {
            return Ok(());
        }
        player.mark_dirty();
        let mut packets = Vec::new();
        for quest_id in changed {
            let status = player.quests.status(quest_id);
            packets.push(quest::build_quest_progress(quest_id, status)?);
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
            // Hunting quests start with every kill count at 0 ("000").
            next.quests.upsert(QuestRecord {
                quest_id: template.id,
                status: QuestStatus::Started {
                    kills: vec![0; template.end_mobs.len()],
                },
            });
        }
        if let Some(change) = change {
            let snapshot = next.clone();
            let progress = match change {
                QuestChange::Start(template) => {
                    let prerequisites: Vec<_> = template
                        .start
                        .quests
                        .iter()
                        .map(|p| (p.quest_id, p.state))
                        .collect();
                    spawn_db(move || db::quest::start(&snapshot, template.id, &prerequisites)).await
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
                announce.push(effect::build_foreign_effect(client_id, CharEffect::LevelUp)?);
            }
            if advanced {
                announce.push(effect::build_foreign_effect(client_id, CharEffect::JobChange)?);
            }
            self.send_to_field(FieldMessage::UpdateAvatar {
                client_id,
                avatar,
                announce,
            })
            .await?;
        }
        Ok(true)
    }

    async fn forfeit_quest(&mut self, quest_id: i16) -> Result<(), RuntimeError> {
        if self.in_game()?.player.quests.state(quest_id) != QuestState::Started {
            return Ok(());
        }
        let cid = self.client_id;
        match spawn_db(move || db::quest::forfeit(cid, quest_id)).await {
            Ok(()) => {}
            Err(RuntimeError::Database(db::Error::NotFound)) => return Ok(()),
            Err(error) => return Err(error),
        }
        self.in_game_mut()?.player.quests.remove(quest_id);
        let mut packets = vec![quest::build_quest_progress(quest_id, None)?];
        self.send_all(&mut packets).await
    }

    pub(super) async fn send_all(&mut self, packets: &mut [Packet]) -> Result<(), RuntimeError> {
        for packet in packets {
            self.send(packet).await?;
        }
        Ok(())
    }
}

/// Add quest or script EXP to `next`, with the stat update and the EXP
/// message in the chat log.
pub(super) fn gain_exp(
    next: &mut Player,
    amount: i32,
    packets: &mut Vec<Packet>,
) -> Result<(), RuntimeError> {
    let levelled = next.gain_exp(amount) > 0;
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
    let levelled = next.gain_exp(amount.saturating_add(party_bonus)) > 0;
    packets.push(stat::build_exp_stats(&next.stats, levelled)?);
    packets.push(quest::build_gain_exp(amount, party_bonus)?);
    Ok(())
}
