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
        if field != game.field.key {
            return Ok(());
        }
        self.talk_to(npc_id).await
    }

    /// Open `npc_id`'s conversation without a click, as a portal does, but
    /// not over one the player is in already.
    pub(super) async fn open_npc(&mut self, npc_id: i32) -> Result<(), RuntimeError> {
        if self.script.is_some() || self.conversation.is_some() {
            return Ok(());
        }
        self.talk_to(npc_id).await
    }

    /// Whatever `npc_id` has to say: its script, its shop or storage, or its
    /// speech.
    async fn talk_to(&mut self, npc_id: i32) -> Result<(), RuntimeError> {
        self.conversation = None;
        if npc_id == net::npc_scripts::HERACLE || npc_id == net::npc_scripts::LEA {
            return self.talk_about_guild(npc_id).await;
        }
        if npc_id == net::npc_scripts::LENARIO {
            return self.talk_to_lenario().await;
        }
        if npc_id == net::npc_scripts::LAKELIS {
            return self.talk_about_event(net::events::EventKind::Kpq).await;
        }
        if npc_id == net::npc_scripts::RED_SIGN {
            return self.talk_about_event(net::events::EventKind::Lpq).await;
        }
        if npc_id == net::npc_scripts::CLOTO && self.talk_to_cloto().await? {
            return Ok(());
        }
        if self.talk_to_balloon(npc_id).await?
            || self.talk_to_wedding_assistant(npc_id).await?
            || self.talk_to_wedding_staff(npc_id).await?
        {
            return Ok(());
        }
        let player = &self.in_game()?.player;
        if npc_id == net::npc_scripts::TORY && player.stats.map_id == net::events::HPQ.lobby {
            return self.talk_about_event(net::events::EventKind::Hpq).await;
        }
        if npc_id == net::npc_scripts::WONKY && player.stats.map_id == net::events::OPQ.lobby {
            return self.talk_about_event(net::events::EventKind::Opq).await;
        }
        if npc_id == net::npc_scripts::ADOBIS && player.stats.map_id == net::events::ZPQ.lobby {
            return self.talk_about_event(net::events::EventKind::Zpq).await;
        }
        if npc_id == net::npc_scripts::HTPQ_KEEPER && player.stats.map_id == net::events::HTPQ.lobby
        {
            return self.talk_about_event(net::events::EventKind::Htpq).await;
        }
        if npc_id == net::npc_scripts::DELLI && player.stats.map_id == net::events::DELLI.lobby {
            return self.talk_about_event(net::events::EventKind::Delli).await;
        }
        // Tylus is the warriors' third-job chief too: his party's menu is
        // only for one on "Protecting Tylus".
        if npc_id == net::npc_scripts::TYLUS
            && player.stats.map_id == net::events::TYLUS.lobby
            && player.quests.state(net::events::TYLUS_QUEST) == game::QuestState::Started
        {
            return self.talk_about_event(net::events::EventKind::Tylus).await;
        }
        if npc_id == net::npc_scripts::ELLIN && player.stats.map_id == net::events::ELLIN.lobby {
            return self.talk_about_event(net::events::EventKind::Ellin).await;
        }
        if npc_id == net::npc_scripts::ROLLY && player.stats.map_id == net::events::LUDIMAZE.lobby {
            return self
                .talk_about_event(net::events::EventKind::LudiMaze)
                .await;
        }
        // The NPCs who gather squads, each on its event's lobby.
        for kind in net::events::EventKind::ALL {
            let recruiter = net::npc_scripts::recruiter(kind).map(|recruiter| recruiter.npc);
            if recruiter == Some(npc_id) && player.stats.map_id == kind.event().lobby {
                return self.talk_about_event(kind).await;
            }
        }
        if npc_id == net::npc_scripts::AURA {
            if let Some(script) = self.zpq_aura()? {
                return self.start_script(script).await;
            }
        }
        if npc_id == net::npc_scripts::EAK {
            if let Some(script) = self.opq_eak().await? {
                return self.start_script(script).await;
            }
        }
        let player = &self.in_game()?.player;
        if let Some(ticket) = self.event.as_ref().filter(|t| t.holds(player.stats.map_id)) {
            let (leader, won) = (ticket.leader(), ticket.won());
            let script = net::npc_scripts::NpcScript::in_event(npc_id, player, leader, won);
            if let Some(script) = script {
                return self.start_script(script).await;
            }
        }
        if let Some(script) = net::npc_scripts::NpcScript::for_npc(npc_id, player) {
            return self.start_script(script).await;
        }
        if self.open_shop(npc_id).await?
            || self.open_storage(npc_id).await?
            || self.open_duey(npc_id).await?
            || self.open_store_bank(npc_id).await?
            || self.open_rps(npc_id).await?
        {
            return Ok(());
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

    /// Natural recovery reported by the client. Like v83, HP heals only
    /// once the player has stood still for its interval (5 or 10 seconds),
    /// MP every 10 seconds whatever they do. Each allows a second of slack.
    pub(super) async fn natural_heal(&mut self, hp: i16, mp: i16) -> Result<(), RuntimeError> {
        let now = tokio::time::Instant::now();
        let hp_interval = {
            let player = &self.in_game()?.player;
            std::time::Duration::from_millis(net::skills::natural_recovery(player).hp_interval)
        };
        let recent = |at: Option<tokio::time::Instant>, interval: std::time::Duration| {
            at.is_some_and(|at| now.duration_since(at) + NATURAL_HEAL_SLACK < interval)
        };
        let hp = if recent(self.last_moved, hp_interval) || recent(self.last_heal.0, hp_interval) {
            0
        } else {
            hp
        };
        let mp = if recent(self.last_heal.1, MP_HEAL_INTERVAL) {
            0
        } else {
            mp
        };
        if hp <= 0 && mp <= 0 {
            return Ok(());
        }
        let player = &mut self.in_game_mut()?.player;
        if player.stats.hp <= 0 {
            return Ok(());
        }
        player.add_hp(i32::from(hp));
        player.add_mp(i32::from(mp));
        let (hp_now, mp_now) = (player.stats.hp, player.stats.mp);
        if hp > 0 {
            self.last_heal.0 = Some(now);
        }
        if mp > 0 {
            self.last_heal.1 = Some(now);
        }
        let mut packet = net::packet::build::world::stat::build_stat_update(
            false,
            &[StatUpdate::Hp(hp_now), StatUpdate::Mp(mp_now)],
        )
        .map_err(|e| RuntimeError::Handler(e.to_string()))?;
        self.send(&mut packet).await
    }
}

/// The v83 client reports MP recovery every 10 seconds.
const MP_HEAL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

/// How early a heal may arrive, for network jitter.
const NATURAL_HEAL_SLACK: std::time::Duration = std::time::Duration::from_secs(1);
