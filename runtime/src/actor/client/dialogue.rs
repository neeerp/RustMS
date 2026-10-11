use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError, message::FieldKey};
use net::dialogue::Conversation;
use net::packet::build::world::{npc, quest};
use net::quests::{QuestEffect, QuizConversation};
use std::collections::BTreeMap;

pub(super) enum ActiveConversation {
    Normal(Conversation),
    Quiz(QuizConversation),
}

impl ClientActor {
    pub(super) async fn start_npc_conversation(
        &mut self,
        npc_id: i32,
        field: FieldKey,
    ) -> Result<(), RuntimeError> {
        let chr = self.session.get_character()?;
        let (map_id, job) = {
            let chr = chr.lock().unwrap();
            (chr.character.map_id, chr.character.job)
        };
        let channel_id = self
            .session
            .session
            .as_ref()
            .and_then(|s| s.selected_channel_id)
            .unwrap_or(0) as u8;
        if field.map_id != map_id || field.channel_id != channel_id || self.client_id == 0 {
            return Ok(());
        }
        self.conversation = None;
        if npc_id == 12101 {
            let cid = self.client_id;
            let progress = spawn_db(move || db::quest::get_quests(cid)).await?;
            let states: BTreeMap<_, _> = progress
                .into_iter()
                .map(|q| (q.quest_id, q.status))
                .collect();
            let data = net::quests::data().map_err(|e| RuntimeError::Handler(e.to_string()))?;
            if let Some(template) = net::quests::eligible(data, npc_id, job, &states) {
                let c = QuizConversation::new(template, states.get(&template.id) == Some(&1));
                let mut packet = npc::build_quest_prompt(&c.prompt())
                    .map_err(|e| RuntimeError::Handler(e.to_string()))?;
                self.writer.send_packet(&mut packet).await?;
                self.conversation = Some(ActiveConversation::Quiz(c));
                return Ok(());
            }
        }
        let data =
            net::get_npc_dialogue_data().map_err(|e| RuntimeError::Handler(e.to_string()))?;
        if let Some(c) = data.dialogue(npc_id).and_then(Conversation::start) {
            let mut packet = npc::build_npc_dialogue(&c.current_page())
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.writer.send_packet(&mut packet).await?;
            self.conversation = Some(ActiveConversation::Normal(c));
        }
        Ok(())
    }

    pub(super) async fn respond_to_npc(
        &mut self,
        message_type: u8,
        response: i8,
        selection: Option<i32>,
    ) -> Result<(), RuntimeError> {
        let Some(mut conversation) = self.conversation.take() else {
            return Ok(());
        };
        let mut packet = match &mut conversation {
            ActiveConversation::Normal(c) => {
                let Some(page) = c.respond(message_type, response) else {
                    return Ok(());
                };
                npc::build_npc_dialogue(&page)
            }
            ActiveConversation::Quiz(c) => {
                let Some((prompt, effect)) = c.respond(message_type, response, selection) else {
                    return Ok(());
                };
                if let Some(effect) = effect {
                    let q = c.quest.clone();
                    let cid = self.client_id;
                    let reward = if effect == QuestEffect::Complete {
                        Some(q.experience)
                    } else {
                        None
                    };
                    let (character, progress) = spawn_db(move || {
                        db::quest::transition(cid, q.id, q.required_job, q.prerequisite, reward)
                    })
                    .await?;
                    let mut packets = vec![quest::build_quest_progress(&progress)
                        .map_err(|e| RuntimeError::Handler(e.to_string()))?];
                    if let Some(gain) = reward {
                        packets.push(
                            quest::build_reward_stats(&character)
                                .map_err(|e| RuntimeError::Handler(e.to_string()))?,
                        );
                        packets.push(
                            quest::build_gain_exp(gain)
                                .map_err(|e| RuntimeError::Handler(e.to_string()))?,
                        );
                    }
                    // Keep the session snapshot current so subsequent map/keybind
                    // handlers cannot save the old EXP over the committed reward.
                    self.session.get_character()?.lock().unwrap().character = character;
                    for packet in &mut packets {
                        self.writer.send_packet(packet).await?;
                    }
                }
                npc::build_quest_prompt(&prompt)
            }
        }
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.writer.send_packet(&mut packet).await?;
        self.conversation = Some(conversation);
        Ok(())
    }
}
