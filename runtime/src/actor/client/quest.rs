use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::FieldMessage};
use game::QuestState;
use net::packet::build::world::quest;
use net::quests::{self, QuestActionKind, QuestRequest, QuestTemplate};
use packet::Packet;
use tracing::{debug, warn};

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
        let (npc_id, check) = match request.kind {
            QuestActionKind::Start { npc_id } => {
                (npc_id, quests::check_start(template, npc_id, player))
            }
            QuestActionKind::Complete { npc_id, .. } => {
                (npc_id, quests::check_complete(template, npc_id, player))
            }
            QuestActionKind::Forfeit => return self.forfeit_quest(request.quest_id).await,
            // No supported quest is scripted; scripted quests are refused by the catalog.
            QuestActionKind::ScriptStart { .. } | QuestActionKind::ScriptEnd { .. } => {
                return Ok(())
            }
        };
        if let Err(rejection) = check {
            debug!(
                quest_id = request.quest_id,
                ?rejection,
                "Quest action rejected"
            );
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
            _ => Ok(()),
        }
    }

    async fn start_quest(
        &mut self,
        template: &QuestTemplate,
        npc_id: i32,
    ) -> Result<(), RuntimeError> {
        // Recheck: the player may have changed while the field answered.
        if quests::check_start(template, npc_id, &self.in_game()?.player).is_err() {
            return Ok(());
        }
        let (cid, qid) = (self.client_id, template.id);
        let prerequisites: Vec<_> = template
            .start
            .quests
            .iter()
            .map(|p| (p.quest_id, p.state))
            .collect();
        let progress = match spawn_db(move || db::quest::start(cid, qid, &prerequisites)).await {
            Ok(progress) => progress,
            Err(RuntimeError::Database(db::Error::NotFound)) => {
                warn!(cid, qid, "Quest start refused by the database");
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let record = db::player::quest_record(&progress);
        let mut packets = vec![
            quest::build_quest_progress(qid, Some(&record.status))?,
            quest::build_quest_action_result(qid, npc_id, 0)?,
        ];
        self.in_game_mut()?.player.quests.upsert(record);
        self.send_all(&mut packets).await
    }

    async fn complete_quest(
        &mut self,
        template: &QuestTemplate,
        npc_id: i32,
    ) -> Result<(), RuntimeError> {
        if quests::check_complete(template, npc_id, &self.in_game()?.player).is_err() {
            return Ok(());
        }
        // The transaction locks and updates the character row, so flush any
        // unsaved in-memory changes first; then adopt what it committed.
        self.flush().await?;
        let (cid, qid, exp) = (self.client_id, template.id, template.exp);
        let (character, progress) = match spawn_db(move || db::quest::complete(cid, qid, exp)).await
        {
            Ok(result) => result,
            Err(RuntimeError::Database(db::Error::NotFound)) => {
                warn!(cid, qid, "Quest completion refused by the database");
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let player = &mut self.in_game_mut()?.player;
        let previous_level = player.stats.level;
        player.stats.level = u8::try_from(character.level).unwrap_or(previous_level);
        player.stats.exp = character.exp;
        player.stats.ap = character.ap;
        let record = db::player::quest_record(&progress);
        let mut packets = vec![quest::build_quest_progress(qid, Some(&record.status))?];
        player.quests.upsert(record);
        let level_changed = player.stats.level != previous_level;

        // Only continue into a quest the client can actually be offered.
        let next = template
            .next_quest
            .filter(|id| quests::catalog().is_ok_and(|c| c.quest(*id).is_some()))
            .unwrap_or(0);
        if exp > 0 {
            packets.push(quest::build_reward_stats(&self.in_game()?.player.stats)?);
            packets.push(quest::build_gain_exp(exp)?);
        }
        packets.push(quest::build_quest_action_result(qid, npc_id, next)?);
        self.send_all(&mut packets).await?;

        if level_changed {
            // Players joining the map later should see the new level.
            let avatar = self.in_game()?.player.field_avatar(0, 0, 0);
            let client_id = self.client_id;
            self.send_to_field(FieldMessage::UpdateAvatar { client_id, avatar })
                .await?;
        }
        Ok(())
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

    async fn send_all(&mut self, packets: &mut [Packet]) -> Result<(), RuntimeError> {
        for packet in packets {
            self.send(packet).await?;
        }
        Ok(())
    }
}
