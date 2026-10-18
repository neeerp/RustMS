use super::quest::{gain_exp, QuestChange};
use super::ClientActor;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use net::conti::{self, State};
use net::dialogue::Prompt;
use net::npc_scripts::{self, NpcScript, Step};
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{drop, effect, guild, inventory, npc, stat};
use net::quests::{self, QuestTemplate};
use net::scripts::Effect;
use tracing::{debug, warn};

/// The NPC that offers veterans a tutorial skip.
const MAPLE_ADMINISTRATOR: i32 = 2007;

/// A script conversation and the prompt it is waiting on.
pub(super) struct ActiveScript {
    script: NpcScript,
    prompt: Prompt,
}

/// What became of a step's effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Applied {
    Yes,
    /// The items it gives don't fit; nothing was applied.
    NoRoom,
    /// The database refused the quest change, or the player no longer has
    /// what it takes; nothing was applied.
    Refused,
}

impl ClientActor {
    pub(super) async fn start_script(&mut self, script: NpcScript) -> Result<(), RuntimeError> {
        self.conversation = None;
        let step = script.begin();
        self.run_step(script, step).await
    }

    /// Answer the running script, if there is one. Returns whether a script
    /// handled the answer.
    pub(super) async fn respond_to_script(
        &mut self,
        message_type: u8,
        response: i8,
        selection: Option<i32>,
    ) -> Result<bool, RuntimeError> {
        let Some(ActiveScript { mut script, prompt }) = self.script.take() else {
            return Ok(false);
        };
        let Some(input) = npc_scripts::input(&prompt, message_type, response, selection) else {
            // Closed, or an answer that doesn't fit the prompt: end it.
            return Ok(true);
        };
        let Some(step) = script.respond(input) else {
            debug!(?input, "Script refused the answer");
            return Ok(true);
        };
        self.run_step(script, step).await?;
        Ok(true)
    }

    /// Apply a step's effects, then show its prompt.
    async fn run_step(&mut self, mut script: NpcScript, step: Step) -> Result<(), RuntimeError> {
        let step = match self.apply_effects(&step.effects).await? {
            Applied::Yes => step,
            Applied::NoRoom => match script.inventory_full() {
                Some(step) => step,
                None => return Ok(()),
            },
            Applied::Refused => return Ok(()),
        };
        if let Some(prompt) = step.prompt {
            let mut packet = npc::build_prompt(&prompt)?;
            self.send(&mut packet).await?;
            self.script = Some(ActiveScript { script, prompt });
        }
        Ok(())
    }

    /// Apply script effects. Changes to the player (with any quest change)
    /// commit together or not at all; screen effects follow them.
    pub(super) async fn apply_effects(
        &mut self,
        effects: &[Effect],
    ) -> Result<Applied, RuntimeError> {
        let mut next = self.in_game()?.player.clone();
        let mut changed = false;
        let mut change = None;
        let mut packets = Vec::new();
        let mut afterwards = Vec::new();
        let mut offer_skip = false;
        let mut warp = None;
        let mut name_guild = false;
        for effect in effects {
            match effect {
                Effect::FieldEffect(path) => afterwards.push(effect::build_field_effect(path)?),
                Effect::AvatarEffect(path) => afterwards.push(effect::build_avatar_effect(path)?),
                Effect::Hint {
                    text,
                    width,
                    height,
                } => afterwards.push(effect::build_hint(text, *width, *height)?),
                Effect::OfferTutorialSkip => offer_skip = true,
                Effect::OpenGuildNamePrompt => {
                    name_guild = true;
                    afterwards.push(guild::build_name_prompt()?);
                }
                Effect::SetHp(hp) => {
                    next.stats.hp = (*hp).clamp(0, next.stats.max_hp);
                    changed = true;
                    packets.push(stat::build_stat_update(
                        false,
                        &[StatUpdate::Hp(next.stats.hp)],
                    )?);
                }
                Effect::GiveItem { item_id, quantity } => {
                    let changes = match net::items::give(&mut next.inventory, *item_id, *quantity) {
                        Ok(changes) => changes,
                        Err(error) => {
                            warn!(item_id, %error, "Script item not given; nothing applied");
                            return Ok(Applied::NoRoom);
                        }
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(
                        *item_id,
                        i32::from(*quantity),
                    )?);
                }
                Effect::TakeItem { item_id, quantity } => {
                    let Ok(changes) = next.inventory.remove(*item_id, *quantity) else {
                        warn!(item_id, "Script item no longer held; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(
                        *item_id,
                        -i32::from(*quantity),
                    )?);
                }
                Effect::SpendMeso(amount) => {
                    if next.meso < *amount {
                        warn!(amount, "Script mesos no longer held; nothing applied");
                        return Ok(Applied::Refused);
                    }
                    next.meso -= amount;
                    changed = true;
                    packets.push(stat::build_stat_update(
                        false,
                        &[StatUpdate::Meso(next.meso)],
                    )?);
                    packets.push(drop::build_meso_gain_in_chat(-amount)?);
                }
                Effect::GainExp(amount) => {
                    gain_exp(&mut next, *amount, &mut packets)?;
                    changed = true;
                }
                Effect::StartQuest(id) => change = Some(QuestChange::Start(template(*id)?)),
                Effect::CompleteQuest(id) => change = Some(QuestChange::Complete(template(*id)?)),
                Effect::Warp(map_id) => warp = Some(*map_id),
                Effect::RequireBoarding(platform) => {
                    let boarding = conti::departing_from(*platform)
                        .is_some_and(|route| route.timetable.state(conti::now()) == State::Wait);
                    if !boarding {
                        debug!(platform, "Boarding closed; nothing applied");
                        return Ok(Applied::Refused);
                    }
                }
            }
        }
        if (changed || change.is_some()) && !self.commit_player(next, change, &mut packets).await? {
            return Ok(Applied::Refused);
        }
        packets.append(&mut afterwards);
        self.naming_guild |= name_guild;
        self.send_all(&mut packets).await?;

        if offer_skip {
            let account_id = self.in_game()?.player.account_id;
            let veteran =
                spawn_db(move || db::character::account_has_level(account_id, 30)).await?;
            let player = &self.in_game()?.player;
            if let Some(script) = veteran
                .then(|| NpcScript::for_npc(MAPLE_ADMINISTRATOR, player))
                .flatten()
            {
                Box::pin(self.start_script(script)).await?;
            }
        }
        if let Some(map_id) = warp {
            let channel_id = self.channel_id;
            let actions = net::packet::handle::world::warp(
                &mut self.in_game_mut()?.player,
                map_id,
                "",
                channel_id,
            )?;
            for action in actions {
                Box::pin(self.apply(action)).await?;
            }
        }
        Ok(Applied::Yes)
    }
}

fn template(quest_id: i16) -> Result<&'static QuestTemplate, RuntimeError> {
    quests::catalog()
        .map_err(|e| RuntimeError::Handler(e.to_string()))?
        .quest(quest_id)
        .ok_or_else(|| RuntimeError::Handler(format!("script names unknown quest {quest_id}")))
}
