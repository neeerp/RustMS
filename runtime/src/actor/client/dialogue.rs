use super::ClientActor;
use crate::{
    db::spawn_db,
    error::RuntimeError,
    message::{FieldKey, FieldMessage},
};
use net::dialogue::Conversation;
use net::packet::build::world::{npc, quest};
use net::quests::{ContactInteraction, ContactOffer, QuestEffect, QuizConversation};
use std::collections::BTreeMap;

pub(super) enum ActiveConversation {
    Normal(Conversation),
    Quiz(QuizConversation),
    Contact(ContactOffer),
}

impl ClientActor {
    pub(super) async fn start_npc_conversation(
        &mut self,
        npc_id: i32,
        field: FieldKey,
    ) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        let stats = &game.player.stats;
        let (map_id, job, level) = (stats.map_id, stats.job, i16::from(stats.level));
        if field.map_id != map_id || field.channel_id != self.channel_id {
            return Ok(());
        }
        // Quest states come from the in-memory log, which every transition updates.
        let mut states: BTreeMap<_, _> = game
            .player
            .quests
            .started()
            .map(|q| (q.quest_id, 1))
            .chain(game.player.quests.completed().map(|(id, _)| (id, 2)))
            .collect();
        self.conversation = None;
        let contacts =
            net::quests::contact_data().map_err(|e| RuntimeError::Handler(e.to_string()))?;
        if npc_id == 12101
            || contacts
                .quests()
                .any(|q| q.start_npc == npc_id || q.finish_npc == npc_id)
        {
            if let Some((template, interaction)) =
                net::quests::contact_eligible(contacts, npc_id, job, level, &states)
            {
                match interaction {
                    ContactInteraction::Complete => {
                        self.apply_quest_effect(
                            template.id,
                            template.required_job,
                            template.prerequisite,
                            None,
                            Some(template.experience),
                        )
                        .await?;
                        states.insert(template.id, 2);
                        if self.send_contact_pages(npc_id, &template.finished).await? {
                            return Ok(());
                        }
                        // Nina/Sen have no completion speech: their Act/nextQuest
                        // continues the conversation with the next existing offer.
                        // Only a supported, eligible quest may be offered here.
                        if let Some(next) = template.next_quest.and_then(|id| contacts.quest(id)) {
                            if net::quests::contact_eligible(contacts, npc_id, job, level, &states)
                                .is_some_and(|(q, role)| {
                                    q.id == next.id && role == ContactInteraction::Offer
                                })
                            {
                                let c = ContactOffer::new(next);
                                let mut p = npc::build_quest_prompt(&c.prompt())
                                    .map_err(|e| RuntimeError::Handler(e.to_string()))?;
                                self.writer.send_packet(&mut p).await?;
                                self.conversation = Some(ActiveConversation::Contact(c));
                                return Ok(());
                            }
                        }
                    }
                    ContactInteraction::Offer => {
                        let c = ContactOffer::new(template);
                        let mut p = npc::build_quest_prompt(&c.prompt())
                            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
                        self.writer.send_packet(&mut p).await?;
                        self.conversation = Some(ActiveConversation::Contact(c));
                        return Ok(());
                    }
                    ContactInteraction::Reminder => {
                        if self.send_contact_pages(npc_id, &template.reminder).await? {
                            return Ok(());
                        }
                    }
                }
            }
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
                    let reward = if effect == QuestEffect::Complete {
                        Some(q.experience)
                    } else {
                        None
                    };
                    self.apply_quest_effect(q.id, q.required_job, q.prerequisite, None, reward)
                        .await?;
                }
                npc::build_quest_prompt(&prompt)
            }
            ActiveConversation::Contact(c) => {
                let Some((prompt, effect)) = c.respond(message_type, response) else {
                    return Ok(());
                };
                if effect == Some(QuestEffect::Start) {
                    let q = &c.quest;
                    self.apply_quest_effect(
                        q.id,
                        q.required_job,
                        q.prerequisite,
                        Some(q.max_level),
                        None,
                    )
                    .await?;
                }
                npc::build_quest_prompt(&prompt)
            }
        }
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.writer.send_packet(&mut packet).await?;
        self.conversation = Some(conversation);
        Ok(())
    }

    async fn send_contact_pages(
        &mut self,
        npc_id: i32,
        pages: &[String],
    ) -> Result<bool, RuntimeError> {
        let Some(c) = Conversation::from_pages(npc_id, pages) else {
            return Ok(false);
        };
        let mut p = npc::build_npc_dialogue(&c.current_page())
            .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.writer.send_packet(&mut p).await?;
        self.conversation = Some(ActiveConversation::Normal(c));
        Ok(true)
    }

    async fn apply_quest_effect(
        &mut self,
        id: i16,
        job: i16,
        prerequisite: Option<i16>,
        max_level: Option<i16>,
        reward: Option<i32>,
    ) -> Result<(), RuntimeError> {
        // The transition locks and updates the character row, so flush any
        // unsaved in-memory changes first; then adopt what it committed.
        self.flush().await?;
        let cid = self.client_id;
        let (character, progress) =
            spawn_db(move || db::quest::transition(cid, id, job, prerequisite, max_level, reward))
                .await?;
        let player = &mut self.in_game_mut()?.player;
        let previous_level = player.stats.level;
        player.stats.level = u8::try_from(character.level).unwrap_or(previous_level);
        player.stats.exp = character.exp;
        player.stats.ap = character.ap;
        player.quests.upsert(db::player::quest_record(&progress));
        let level_changed = player.stats.level != previous_level;
        let mut packets = vec![quest::build_quest_progress(&progress)
            .map_err(|e| RuntimeError::Handler(e.to_string()))?];
        if let Some(gain) = reward {
            packets.push(
                quest::build_reward_stats(&self.in_game()?.player.stats)
                    .map_err(|e| RuntimeError::Handler(e.to_string()))?,
            );
            packets.push(
                quest::build_gain_exp(gain).map_err(|e| RuntimeError::Handler(e.to_string()))?,
            );
        }
        for packet in &mut packets {
            self.writer.send_packet(packet).await?;
        }
        if level_changed {
            // Players joining the map later should see the new level.
            let game = self.in_game()?;
            let avatar = game.player.field_avatar(0, 0, 0);
            let client_id = self.client_id;
            self.send_to_field(FieldMessage::UpdateAvatar { client_id, avatar })
                .await?;
        }
        Ok(())
    }
}
