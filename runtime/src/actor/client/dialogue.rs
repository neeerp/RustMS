use super::ClientActor;
use crate::{error::RuntimeError, message::FieldKey};
use net::dialogue::Conversation;
use net::packet::build::world::npc;

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
}
