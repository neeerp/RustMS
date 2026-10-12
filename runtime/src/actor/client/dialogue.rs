use super::ClientActor;
use crate::{error::RuntimeError, message::FieldKey};
use net::dialogue::Conversation;
use net::packet::build::world::npc;
use net::packet::build::world::stat::StatUpdate;

impl ClientActor {
    /// Start an NPC's normal speech once its field confirmed the NPC. Quests
    /// are not conversations here: the client renders them itself.
    pub(super) async fn start_npc_conversation(
        &mut self,
        npc_id: i32,
        field: FieldKey,
    ) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };
        if field.map_id != game.player.stats.map_id || field.channel_id != self.channel_id {
            return Ok(());
        }
        self.conversation = None;
        if let Some(script) = net::npc_scripts::NpcScript::for_npc(npc_id) {
            return self.start_script(script).await;
        }
        let data =
            net::get_npc_dialogue_data().map_err(|e| RuntimeError::Handler(e.to_string()))?;
        if let Some(c) = data.dialogue(npc_id).and_then(Conversation::start) {
            let mut packet = npc::build_npc_dialogue(&c.current_page())
                .map_err(|e| RuntimeError::Handler(e.to_string()))?;
            self.send(&mut packet).await?;
            self.conversation = Some(c);
        }
        Ok(())
    }

    pub(super) async fn respond_to_npc(
        &mut self,
        message_type: u8,
        response: i8,
    ) -> Result<(), RuntimeError> {
        let Some(mut conversation) = self.conversation.take() else {
            return Ok(());
        };
        let Some(page) = conversation.respond(message_type, response) else {
            return Ok(());
        };
        let mut packet =
            npc::build_npc_dialogue(&page).map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut packet).await?;
        self.conversation = Some(conversation);
        Ok(())
    }

    /// Natural recovery reported by the client, at most every 9 seconds.
    pub(super) async fn natural_heal(&mut self, hp: i16, mp: i16) -> Result<(), RuntimeError> {
        let now = tokio::time::Instant::now();
        if self
            .last_heal
            .is_some_and(|last| now.duration_since(last) < NATURAL_HEAL_INTERVAL)
        {
            return Ok(());
        }
        let player = &mut self.in_game_mut()?.player;
        if player.stats.hp <= 0 {
            return Ok(());
        }
        player.add_hp(i32::from(hp));
        player.add_mp(i32::from(mp));
        let (hp, mp) = (player.stats.hp, player.stats.mp);
        self.last_heal = Some(now);
        let mut packet = net::packet::build::world::stat::build_stat_update(
            false,
            &[StatUpdate::Hp(hp), StatUpdate::Mp(mp)],
        )
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut packet).await
    }
}

/// The v83 client reports natural recovery every 10 seconds.
const NATURAL_HEAL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(9);
