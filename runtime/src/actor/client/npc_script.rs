use super::quest::{gain_exp, QuestChange};
use super::ClientActor;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::message::{DirectoryMessage, FieldMessage, SquadAnswer};
use game::pet::PetChange;
use game::quest::{QuestRecord, QuestStatus};
use game::{InventoryType, QuestState, SkillRecord};
use net::conti::{self, State};
use net::dialogue::Prompt;
use net::events::EventKind;
use net::guild_quest;
use net::handler::BroadcastScope;
use net::npc_scripts::{self, NpcScript, Step};
use net::packet::build::world::effect::CharEffect;
use net::packet::build::world::messaging::{
    build_broadcast, build_gachapon_message, build_item_line, Broadcast,
};
use net::packet::build::world::pet::PetGone;
use net::packet::build::world::stat::StatUpdate;
use net::packet::build::world::{
    drop, effect, guild, inventory, marriage, npc, quest, skill, stat,
};
use net::quests::{self, QuestTemplate};
use net::scripts::Effect;
use net::squads::SquadCommand;
use net::{FieldRequest, WorldAction};
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;
use tracing::{debug, warn};

/// The NPC that offers veterans a tutorial skip.
const MAPLE_ADMINISTRATOR: i32 = 2007;
/// The guide's balloon: its width and how long it stays, in ms, and a
/// hint picture's (OdinMS-family servers' numbers; the client reads them
/// from the packet).
const GUIDE_WIDTH: i32 = 200;
const GUIDE_SAYS_FOR: i32 = 4000;
const GUIDE_HINT_FOR: i32 = 7000;

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
        text: Option<String>,
    ) -> Result<bool, RuntimeError> {
        let Some(ActiveScript { mut script, prompt }) = self.script.take() else {
            return Ok(false);
        };
        let Some(input) = npc_scripts::input(&prompt, message_type, response, selection, text)
        else {
            // Closed, or an answer that doesn't fit the prompt: end it.
            return Ok(true);
        };
        let Some(step) = script.respond(input.clone()) else {
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
            Applied::Refused => match script.refused() {
                Some(step) => step,
                None => return Ok(()),
            },
        };
        if let Some(prompt) = step.prompt {
            let mut packet = npc::build_prompt(&prompt)?;
            self.send(&mut packet).await?;
            self.script = Some(ActiveScript { script, prompt });
        }
        Ok(())
    }

    /// Whether anyone is on `map_id` in the player's channel; a lost reply
    /// counts as someone.
    async fn map_occupied(&self, map_id: i32) -> Result<bool, RuntimeError> {
        let (reply, occupied) = oneshot::channel();
        self.send_to_directory(DirectoryMessage::MapOccupied {
            channel_id: self.channel_id,
            map_id,
            reply,
        })
        .await?;
        Ok(occupied.await.unwrap_or(true))
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
        let mut open_npc = None;
        let mut thought = None;
        let mut to_field = Vec::new();
        let mut warp = None;
        let mut name_guild = false;
        let mut grow_guild = None;
        let mut disband_guild = false;
        let mut guild_points = 0;
        let mut alliance = None;
        let mut carnival = None;
        let mut gm_event = None;
        let mut left_gm_event = None;
        let mut wedding = None;
        let mut restyled = false;
        let mut command = None;
        let mut gachapon_win = None;
        let mut evolved = Vec::new();
        let mut item_buffs = Vec::new();
        for effect in effects {
            match effect {
                Effect::FieldEffect(path) => afterwards.push(effect::build_field_effect(path)?),
                Effect::Sound(path) => afterwards.push(effect::build_field_sound(path)?),
                Effect::SoundToMap(path) => {
                    let sound = effect::build_field_sound(path)?;
                    to_field.push(FieldRequest::BroadcastAll(sound.clone()));
                    afterwards.push(sound);
                }
                Effect::PortalSound => {
                    afterwards.push(effect::build_own_effect(CharEffect::PortalSound)?)
                }
                Effect::ApplyItemBuff(_) | Effect::CancelItemBuff(_) => item_buffs.push(effect),
                Effect::SaveLocation(kind, location) => {
                    match location {
                        Some(map_id) => next.saved_locations.insert(*kind, *map_id),
                        None => next.saved_locations.remove(kind),
                    };
                    changed = true;
                }
                Effect::DojoPoints(points) => {
                    let Some(total) = next.dojo.points.checked_add(*points).filter(|t| *t >= 0)
                    else {
                        warn!(points, "Dojo points no longer held; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    next.dojo.points = total;
                    changed = true;
                }
                Effect::DojoStage(stage) => {
                    next.dojo.stage = *stage;
                    changed = true;
                }
                Effect::DojoMedal { stage, kills } => {
                    next.dojo.vanquisher_stage = *stage;
                    next.dojo.vanquisher_kills = *kills;
                    changed = true;
                }
                Effect::DojoTrialDone => {
                    next.dojo.tutorial_done = true;
                    changed = true;
                }
                Effect::AriantPoints(points) => {
                    let Some(total) =
                        (next.ariant_points.checked_add(*points)).filter(|total| *total >= 0)
                    else {
                        warn!(
                            points,
                            "Battle Arena points no longer held; nothing applied"
                        );
                        return Ok(Applied::Refused);
                    };
                    next.ariant_points = total;
                    changed = true;
                }
                Effect::AriantPrizeTaken => {
                    next.ariant_tier = next.ariant_tier.map(|_| 0);
                    changed = true;
                }
                Effect::AvatarEffect(path) => afterwards.push(effect::build_avatar_effect(path)?),
                Effect::ScenePart(path) => afterwards.push(effect::build_scene_part(path)?),
                Effect::Scene { parts, exit } => {
                    afterwards.push(effect::build_lock_ui(true)?);
                    for part in parts {
                        afterwards.push(effect::build_scene_part(part)?);
                    }
                    next.scene_exit = Some(*exit);
                    changed = true;
                }
                Effect::Pink(text) => afterwards.push(build_broadcast(Broadcast::Pink, text)?),
                Effect::InfoText(text) => afterwards.push(quest::build_info_text(text)?),
                Effect::Hint {
                    text,
                    width,
                    height,
                } => afterwards.push(effect::build_hint(text, *width, *height)?),
                Effect::SpawnGuide(text) => {
                    afterwards.push(effect::build_spawn_guide(true)?);
                    if !text.is_empty() {
                        afterwards.push(effect::build_guide_says(
                            text,
                            GUIDE_WIDTH,
                            GUIDE_SAYS_FOR,
                        )?);
                    }
                }
                Effect::GuideSays(text) => {
                    afterwards.push(effect::build_guide_says(text, GUIDE_WIDTH, GUIDE_SAYS_FOR)?)
                }
                Effect::RemoveGuide => afterwards.push(effect::build_spawn_guide(false)?),
                Effect::GuideHint(hint) => {
                    afterwards.push(effect::build_guide_hint(*hint, GUIDE_HINT_FOR)?)
                }
                Effect::OfferTutorialSkip => offer_skip = true,
                Effect::OpenNpc(npc_id) => open_npc = Some(*npc_id),
                Effect::Think(text) => thought = Some(text.clone()),
                Effect::BringOutMob { mob_id, at } => to_field.push(FieldRequest::BringOutMob {
                    mob_id: *mob_id,
                    at: *at,
                }),
                Effect::BringOutNpc { npc_id, at } => to_field.push(FieldRequest::BringOutNpc {
                    npc_id: *npc_id,
                    at: *at,
                }),
                Effect::BringOutMobs { mob_id, at, each } => {
                    to_field.push(FieldRequest::BringOutMobs {
                        mob_id: *mob_id,
                        at: at
                            .iter()
                            .flat_map(|&point| std::iter::repeat_n(point, usize::from(*each)))
                            .collect(),
                    })
                }
                Effect::SetReactorState { name, state } => {
                    to_field.push(FieldRequest::SetReactorState {
                        name,
                        state: *state,
                    })
                }
                Effect::OpenReactor(name) => to_field.push(FieldRequest::OpenReactor { name }),
                Effect::NpcSpecialAction { npc_id, action } => {
                    to_field.push(FieldRequest::NpcSpecialAction {
                        npc_id: *npc_id,
                        action,
                    })
                }
                Effect::ClearMobs => to_field.push(FieldRequest::ClearMobs),
                Effect::BringOutSnowman => to_field.push(FieldRequest::BringOutSnowman),
                Effect::BringOutScrooge => to_field.push(FieldRequest::BringOutScrooge),
                Effect::OpenGuildNamePrompt => {
                    name_guild = true;
                    afterwards.push(guild::build_name_prompt()?);
                }
                Effect::OpenEmblemMaker => afterwards.push(guild::build_emblem_maker()?),
                Effect::OpenWishlist => afterwards.push(marriage::build_wishlist_input()?),
                Effect::GrowGuild { from } => grow_guild = Some(*from),
                Effect::DisbandGuild => disband_guild = true,
                Effect::GainGuildPoints(amount) => guild_points += amount,
                Effect::CreateAlliance { .. }
                | Effect::GrowAlliance { .. }
                | Effect::DisbandAlliance => alliance = Some(effect.clone()),
                Effect::CarnivalRoom(_) | Effect::CarnivalAnswer { .. } => {
                    carnival = Some(effect.clone())
                }
                Effect::JoinGmEvent { npc } => gm_event = Some(*npc),
                Effect::LeaveGmEvent { map_id } => left_gm_event = Some(*map_id),
                Effect::BookWedding { .. }
                | Effect::ExtraInvitations { .. }
                | Effect::StartWedding { .. }
                | Effect::Vows
                | Effect::EndWedding
                | Effect::EnterWedding { .. }
                | Effect::LeaveWedding
                | Effect::OpenRegistry { .. }
                | Effect::OpenGifts => wedding = Some(effect.clone()),
                Effect::SetHp(hp) => {
                    next.stats.hp = (*hp).clamp(0, next.effective_max_hp() as i16);
                    changed = true;
                    packets.push(stat::build_stat_update(
                        false,
                        &[StatUpdate::Hp(next.stats.hp)],
                    )?);
                }
                Effect::GiveItem { item_id, quantity } => {
                    if net::items::refuses_only(&next.inventory, *item_id, (*quantity).into()) {
                        warn!(
                            item_id,
                            "Script item is one of a kind and held; nothing applied"
                        );
                        return Ok(Applied::NoRoom);
                    }
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
                Effect::GiveVariedItem(item_id) => {
                    if net::items::refuses_only(&next.inventory, *item_id, 1) {
                        warn!(
                            item_id,
                            "Script item is one of a kind and held; nothing applied"
                        );
                        return Ok(Applied::NoRoom);
                    }
                    let changes = match net::items::give_dropped(
                        &mut next.inventory,
                        *item_id,
                        1,
                        &mut rand::thread_rng(),
                    ) {
                        Ok(changes) => changes,
                        Err(error) => {
                            warn!(item_id, %error, "Script item not given; nothing applied");
                            return Ok(Applied::NoRoom);
                        }
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(*item_id, 1)?);
                }
                Effect::GivePet { item_id, hours } => {
                    let until = SystemTime::now() + Duration::from_secs(hours * 3600);
                    let Some(pet) = net::items::new_pet(*item_id, until) else {
                        warn!(item_id, "Script pet unknown; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    if next.inventory.clone().add(pet.clone(), 1).is_err() {
                        return Ok(Applied::NoRoom);
                    }
                    let serial = spawn_db(db::cash_shop::new_serial).await?;
                    let pet = game::Item {
                        cash_serial: Some(serial),
                        ..pet
                    };
                    let Ok(changes) = next.inventory.add(pet, 1) else {
                        return Ok(Applied::NoRoom);
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(*item_id, 1)?);
                }
                Effect::TakeWorn(slot) => {
                    let Ok(changes) = next.inventory.take(InventoryType::Equipped, *slot, 1) else {
                        continue;
                    };
                    changed = true;
                    restyled = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
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
                Effect::Gachapon {
                    ticket,
                    item_id,
                    quantity,
                    town,
                    announced,
                } => {
                    let Ok(mut changes) = next.inventory.remove(*ticket, 1) else {
                        warn!("Gachapon Ticket no longer held; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    if net::items::refuses_only(&next.inventory, *item_id, (*quantity).into()) {
                        return Ok(Applied::NoRoom);
                    }
                    let Ok(given) = net::items::give(&mut next.inventory, *item_id, *quantity)
                    else {
                        debug!(item_id, "Gachapon prize doesn't fit; nothing applied");
                        return Ok(Applied::NoRoom);
                    };
                    // The world sees the prize as given, an equip with its
                    // stats.
                    let prize = given.iter().find_map(|change| match change {
                        game::InventoryChange::Add { item, .. } if item.item_id == *item_id => {
                            Some(item.clone())
                        }
                        _ => None,
                    });
                    changes.extend(given);
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    packets.push(inventory::build_item_gain_in_chat(*ticket, -1)?);
                    packets.push(inventory::build_item_gain_in_chat(
                        *item_id,
                        i32::from(*quantity),
                    )?);
                    if *announced {
                        gachapon_win = prize
                            .or_else(|| net::items::create(*item_id, *quantity))
                            .map(|prize| (*town, prize));
                    }
                }
                Effect::RevivePet { serial, days } => {
                    let until = SystemTime::now() + Duration::from_secs(*days as u64 * 86_400);
                    let Some(changes) =
                        net::packet::handle::world::revive_pet(&mut next, *serial, until)
                    else {
                        warn!(serial, "Script pet no longer a doll; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                }
                Effect::EvolvePet { serial, into, days } => {
                    // Only the pet that leads, as the quest checked.
                    if next.pets.first() != Some(serial) {
                        warn!(serial, "Script pet no longer leads; nothing applied");
                        return Ok(Applied::Refused);
                    }
                    let until = SystemTime::now() + Duration::from_secs(*days as u64 * 86_400);
                    let Some(changes) =
                        net::packet::handle::world::evolve_pet(&mut next, *serial, *into, until)
                    else {
                        warn!(serial, "Script pet no longer held; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    changed = true;
                    packets.push(inventory::build_inventory_operation(false, &changes)?);
                    if let Some(index) = next.pets.iter().position(|out| out == serial) {
                        let index = index as u8;
                        evolved.push(PetChange::Dismissed { index });
                        evolved.push(PetChange::Summoned {
                            index,
                            serial: *serial,
                        });
                    }
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
                Effect::GainMeso(amount) => {
                    let before = next.meso;
                    next.meso = next.meso.saturating_add((*amount).max(0));
                    changed = true;
                    packets.push(stat::build_stat_update(
                        false,
                        &[StatUpdate::Meso(next.meso)],
                    )?);
                    packets.push(drop::build_meso_gain_in_chat(next.meso - before)?);
                }
                Effect::GainExp(amount) => {
                    gain_exp(&mut next, *amount, &mut packets)?;
                    changed = true;
                }
                Effect::GainFame(amount) => {
                    next.stats.fame = next.stats.fame.saturating_add(*amount);
                    changed = true;
                    packets.push(stat::build_stat_update(
                        false,
                        &[StatUpdate::Fame(next.stats.fame)],
                    )?);
                    packets.push(quest::build_fame_gain_in_chat(*amount)?);
                }
                Effect::TakeFirstJob(job)
                | Effect::TakeSecondJob(job)
                | Effect::TakeThirdJob(job)
                | Effect::TakeFourthJob(job) => {
                    let qualifies = match effect {
                        Effect::TakeFirstJob(_) => next.can_take_first_job(*job),
                        Effect::TakeSecondJob(_) => next.can_take_second_job(*job),
                        Effect::TakeThirdJob(_) => next.can_take_third_job(*job),
                        _ => next.can_take_fourth_job(*job),
                    };
                    if !qualifies {
                        warn!(job, "No longer qualifies for the job; nothing applied");
                        return Ok(Applied::Refused);
                    }
                    match effect {
                        Effect::TakeFirstJob(_) => next.take_first_job(*job),
                        Effect::TakeSecondJob(_) => next.take_second_job(*job),
                        Effect::TakeThirdJob(_) => next.take_third_job(*job),
                        _ => next.take_fourth_job(*job),
                    }
                    changed = true;
                    let stats = &next.stats;
                    let mut updates = vec![
                        StatUpdate::Job(stats.job),
                        StatUpdate::MaxHp(stats.max_hp),
                        StatUpdate::MaxMp(stats.max_mp),
                        StatUpdate::Sp(stats.sp),
                    ];
                    if matches!(effect, Effect::TakeThirdJob(_) | Effect::TakeFourthJob(_)) {
                        updates.push(StatUpdate::Ap(stats.ap));
                    }
                    packets.push(stat::build_stat_update(false, &updates)?);
                    if matches!(effect, Effect::TakeFourthJob(_)) {
                        let taught: Vec<_> = net::skills::fourth_job_skills(*job)
                            .into_iter()
                            .map(|mut record| {
                                if let Some(known) = next.skills.get(record.skill_id) {
                                    record.level = known.level;
                                    record.master_level =
                                        record.master_level.max(known.master_level);
                                }
                                next.skills.set(record.clone());
                                record
                            })
                            .collect();
                        packets.push(skill::build_update_skills(&taught)?);
                    }
                }
                Effect::TeachSkill {
                    skill_id,
                    level,
                    master_level,
                } => {
                    let skill = quests::QuestSkill {
                        skill_id: *skill_id,
                        level: *level,
                        master_level: *master_level,
                        jobs: vec![next.stats.job],
                    };
                    let taught = quests::apply_skills(&[skill], &mut next);
                    if !taught.is_empty() {
                        changed = true;
                        packets.push(skill::build_update_skills(&taught)?);
                    }
                }
                Effect::ForgetSkill(skill_id) => {
                    if next.skills.get(*skill_id).is_none() {
                        continue;
                    }
                    next.skills.remove(*skill_id);
                    changed = true;
                    // Level 0 takes it out of the skill window, as a GM's
                    // `!skill <id> 0` does.
                    packets.push(skill::build_update_skills(&[SkillRecord {
                        skill_id: *skill_id,
                        level: 0,
                        master_level: 0,
                        expiration: None,
                    }])?);
                }
                Effect::Mark(marker) => {
                    next.quests.mark(*marker);
                    changed = true;
                }
                Effect::QuestRecord { quest_id, count } => {
                    let Some(kills) = next.quests.kills_mut(*quest_id) else {
                        continue;
                    };
                    *kills = vec![*count];
                    changed = true;
                    let status = next.quests.status(*quest_id);
                    packets.push(quest::build_quest_progress(*quest_id, status)?);
                }
                Effect::Unmark(marker) => {
                    next.quests.remove(*marker);
                    changed = true;
                }
                Effect::RequireEmptyMap(map_id) => {
                    if self.map_occupied(*map_id).await? {
                        debug!(map_id, "Map occupied; nothing applied");
                        return Ok(Applied::Refused);
                    }
                }
                Effect::WarpToFreeCopy {
                    maps,
                    beside,
                    portal,
                } => {
                    let mut free = None;
                    for &map_id in maps.iter() {
                        if !self.map_occupied(map_id).await?
                            && !self.map_occupied(map_id + beside).await?
                        {
                            free = Some(map_id);
                            break;
                        }
                    }
                    let Some(map_id) = free else {
                        debug!(?maps, "Every copy occupied; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    warp = Some((map_id, Some(*portal)));
                }
                Effect::RequireNoMobs => {
                    let (reply, count) = oneshot::channel();
                    self.send_to_field(FieldMessage::MobCount { reply }).await?;
                    if count.await.unwrap_or(1) > 0 {
                        return Ok(Applied::Refused);
                    }
                }
                Effect::AddSlotRow(kind) => {
                    let limit = next
                        .inventory
                        .slot_limit(*kind)
                        .saturating_add(4)
                        .min(net::cash_shop::MAX_BAG_SLOTS);
                    next.inventory_mut().set_slot_limit(*kind, limit);
                    changed = true;
                    packets.push(inventory::build_inventory_grow(*kind, limit)?);
                }
                Effect::SetStyle(style) => {
                    let stats = &mut next.stats;
                    let update = match style / 10_000 {
                        2 => {
                            stats.face = *style;
                            StatUpdate::Face(*style)
                        }
                        3 => {
                            stats.hair = *style;
                            StatUpdate::Hair(*style)
                        }
                        _ => {
                            stats.skin = *style as u8;
                            StatUpdate::Skin(stats.skin)
                        }
                    };
                    changed = true;
                    restyled = true;
                    packets.push(stat::build_stat_update(false, &[update])?);
                }
                Effect::StartQuest(id) => change = Some(QuestChange::Start(template(*id)?)),
                Effect::SetQuestChar {
                    quest: id,
                    slot,
                    from,
                    to,
                } => {
                    if !next
                        .quests
                        .set_info_char(*id, usize::from(*slot), *from, *to)
                    {
                        return Ok(Applied::Refused);
                    }
                    changed = true;
                    packets.push(quest::build_quest_progress(*id, next.quests.status(*id))?);
                }
                Effect::SetQuestInfo { quest: id, value } => {
                    if next.quests.set_info(*id, value) {
                        changed = true;
                        packets.push(quest::build_quest_progress(*id, next.quests.status(*id))?);
                    } else if let Some(QuestChange::Start(t)) =
                        change.filter(|_| next.quests.state(*id) != QuestState::Started)
                    {
                        if t.id != *id {
                            continue;
                        }
                        // Started in this batch: the quest starts with it,
                        // and its start sends the progress.
                        next.quests.upsert(QuestRecord {
                            quest_id: t.id,
                            status: QuestStatus::Started {
                                kills: vec![0; t.end_mobs.len()],
                                info: Some(value.clone()),
                            },
                        });
                    }
                }
                Effect::SetRecord { record, value } => {
                    if next.quests.set_record(*record, value) {
                        changed = true;
                        packets.push(quest::build_quest_record(*record, value)?);
                    }
                }
                Effect::SetQuestEx {
                    quest: id,
                    key,
                    value,
                } => {
                    if next.quest_ex.get(*id, key) != Some(value.as_str()) {
                        next.quest_ex.set(*id, key, value);
                        changed = true;
                        let data = next.quest_ex.raw(*id).unwrap_or_default();
                        packets.push(quest::build_quest_ex_record(*id, data)?);
                    }
                }
                Effect::Explore { medal, map_id } => {
                    let title = template(*medal)?;
                    let Some(explored) = title.explored else {
                        continue;
                    };
                    let new = next.quests.explore(explored.record, *map_id);
                    // Starting it sends the medal's record; so does each
                    // visit after. A forfeited medal starts again on a map
                    // it has already counted.
                    match next.quests.state(*medal) {
                        QuestState::NotStarted => change = Some(QuestChange::Start(title)),
                        _ if !new => continue,
                        _ => packets.push(quest::build_quest_progress(
                            *medal,
                            next.quests.status(*medal),
                        )?),
                    }
                    changed = true;
                    let count = next.quests.explored(explored.record);
                    packets.push(quest::build_quest_record(
                        explored.record,
                        &count.to_string(),
                    )?);
                    if new {
                        afterwards.extend(net::medals::messages(title, count)?);
                    }
                }
                Effect::CompleteQuest(id) => change = Some(QuestChange::Complete(template(*id)?)),
                Effect::FinishQuest(id) => change = Some(QuestChange::Finish(template(*id)?)),
                Effect::Warp(map_id) => warp = Some((*map_id, None)),
                Effect::WarpAfter { map_id, after } => {
                    self.warp_after(next.stats.map_id, *map_id, *after)
                }
                Effect::WarpToPortal(map_id, portal) => warp = Some((*map_id, Some(*portal))),
                Effect::EnterDungeon {
                    first,
                    count,
                    portal,
                } => {
                    let (reply, entered) = oneshot::channel();
                    self.send_to_directory(DirectoryMessage::EnterDungeon {
                        from: self.client_id,
                        first: *first,
                        count: *count,
                        portal: *portal,
                        reply,
                    })
                    .await?;
                    match entered.await {
                        Ok(Ok(copy)) => warp = Some((copy, Some(*portal))),
                        Ok(Err(refusal)) => {
                            let mut line = build_broadcast(Broadcast::Pink, &refusal.text())?;
                            self.send(&mut line).await?;
                            return Ok(Applied::Refused);
                        }
                        Err(_) => return Ok(Applied::Refused),
                    }
                }
                Effect::StartEvent(kind) => {
                    // Scripts end on this effect: once the directory reserves
                    // the event, the party goes in.
                    let (reply, start) = oneshot::channel();
                    self.send_to_directory(DirectoryMessage::StartEvent {
                        from: self.client_id,
                        kind: *kind,
                        reply,
                    })
                    .await?;
                    let Some(start) = start.await.ok().flatten() else {
                        debug!(?kind, "Party may not go in; nothing applied");
                        return Ok(Applied::Refused);
                    };
                    crate::actor::event::spawn(
                        start,
                        self.services.fields.clone(),
                        self.services.directory.clone(),
                    );
                }
                Effect::EventCommand(told) => command = Some(*told),
                Effect::Squad { kind, command } => {
                    // Not back into one while in an event.
                    if *command == SquadCommand::Rejoin && self.event.is_some() {
                        return Ok(Applied::Refused);
                    }
                    let (reply, answer) = oneshot::channel();
                    self.send_to_directory(DirectoryMessage::Squad {
                        from: self.client_id,
                        kind: *kind,
                        command: *command,
                        reply,
                    })
                    .await?;
                    match answer.await.unwrap_or(SquadAnswer::Refused) {
                        SquadAnswer::Refused => {
                            debug!(?kind, ?command, "Squad refused; nothing applied");
                            return Ok(Applied::Refused);
                        }
                        SquadAnswer::Done => {}
                        SquadAnswer::Start(start) => crate::actor::event::spawn(
                            start,
                            self.services.fields.clone(),
                            self.services.directory.clone(),
                        ),
                        SquadAnswer::Rejoin(ticket) => {
                            warp = Some((kind.event().entry, None));
                            self.event = Some(ticket);
                        }
                    }
                }
                Effect::GuildQuest(command) => {
                    // Not into the guild's run while in another event.
                    if *command == guild_quest::Command::Join && self.event.is_some() {
                        return Ok(Applied::Refused);
                    }
                    let (reply, answer) = oneshot::channel();
                    self.send_to_directory(DirectoryMessage::GuildQuest {
                        from: self.client_id,
                        command: *command,
                        reply,
                    })
                    .await?;
                    match answer.await.unwrap_or(SquadAnswer::Refused) {
                        SquadAnswer::Refused | SquadAnswer::Start(_) => {
                            debug!(?command, "Guild Quest refused; nothing applied");
                            return Ok(Applied::Refused);
                        }
                        SquadAnswer::Done => {}
                        SquadAnswer::Rejoin(ticket) => {
                            warp = Some((EventKind::GuildQuest.event().entry, None));
                            self.event = Some(ticket);
                        }
                    }
                }
                Effect::RequireStage { stage, refused } => {
                    let map_id = next.stats.map_id;
                    let cleared = self
                        .event
                        .as_ref()
                        .is_some_and(|ticket| ticket.holds(map_id) && ticket.cleared(*stage));
                    if !cleared {
                        if let Some(refused) = refused {
                            let text = net::content::text().get(refused, &[]);
                            self.send(&mut build_item_line(&text, 0)?).await?;
                        }
                        return Ok(Applied::Refused);
                    }
                }
                Effect::ShutByReactor { name, state } => {
                    let states = self.reactor_states(&[*name]).await?;
                    if states.first().copied().flatten() == Some(*state) {
                        debug!(name, state, "Shut by a reactor; nothing applied");
                        return Ok(Applied::Refused);
                    }
                }
                Effect::RequireReactorsSet(names) => {
                    let states = self.reactor_states(names).await?;
                    let set = states.len() == names.len()
                        && states
                            .iter()
                            .all(|state| state.is_some_and(|state| state > 0));
                    if !set {
                        debug!(?names, ?states, "Reactors not set; nothing applied");
                        return Ok(Applied::Refused);
                    }
                }
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
        if let Some(command) = command {
            // Only once the items it follows have changed hands.
            self.tell_event(command).await;
        }
        self.naming_guild |= name_guild;
        self.send_all(&mut packets).await?;
        if let Some((town, prize)) = gachapon_win {
            let name = self.in_game()?.player.name().to_owned();
            let packet = build_gachapon_message(&name, self.channel_id, town, &prize)?;
            Box::pin(self.apply(WorldAction::Broadcast {
                scope: BroadcastScope::World,
                packet,
            }))
            .await?;
        }
        if !evolved.is_empty() {
            Box::pin(self.pets_changed(evolved, PetGone::Dismissed)).await?;
        }
        if restyled {
            let announce = inventory::build_char_look(&self.in_game()?.player)?;
            Box::pin(self.apply(WorldAction::LookChanged(announce))).await?;
        }

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
        if let Some(npc_id) = open_npc {
            Box::pin(self.open_npc(npc_id)).await?;
        }
        if let Some(text) = thought {
            if self.script.is_none() && self.conversation.is_none() {
                Box::pin(self.start_script(NpcScript::thought(text))).await?;
            }
        }
        for request in to_field {
            self.send_to_field(FieldMessage::FromClient {
                from: self.client_id,
                request,
            })
            .await?;
        }
        // These change the player themselves, so they wait for the commit.
        if let Some(from) = grow_guild {
            self.grow_guild(from).await?;
        }
        if guild_points != 0 {
            self.gain_guild_points(guild_points).await?;
        }
        if disband_guild {
            self.disband_guild().await?;
        }
        // These say how it went in a script of their own.
        match alliance {
            Some(Effect::CreateAlliance { name }) => Box::pin(self.create_alliance(name)).await?,
            Some(Effect::GrowAlliance { from }) => Box::pin(self.grow_alliance(from)).await?,
            Some(Effect::DisbandAlliance) => Box::pin(self.disband_alliance()).await?,
            _ => {}
        }
        match carnival {
            Some(Effect::CarnivalRoom(room)) => Box::pin(self.pick_carnival_room(room)).await?,
            Some(Effect::CarnivalAnswer { challenger, accept }) => {
                self.answer_carnival(challenger, accept).await?
            }
            _ => {}
        }
        if let Some(npc) = gm_event {
            Box::pin(self.join_gm_event(npc)).await?;
        }
        if let Some(map_id) = left_gm_event {
            self.send_to_directory(DirectoryMessage::LeaveGmEvent {
                channel_id: self.channel_id,
                map_id,
            })
            .await?;
        }
        match wedding {
            Some(Effect::BookWedding { venue, premium }) => {
                Box::pin(self.reserve_wedding(venue, premium)).await?
            }
            Some(Effect::ExtraInvitations { venue }) => {
                Box::pin(self.extra_invitations(venue)).await?
            }
            Some(Effect::StartWedding { venue }) => Box::pin(self.start_wedding(venue)).await?,
            Some(Effect::Vows) => Box::pin(self.take_vows()).await?,
            Some(Effect::EndWedding) => self.end_wedding().await,
            Some(Effect::EnterWedding { venue }) => Box::pin(self.enter_wedding(venue)).await?,
            Some(Effect::LeaveWedding) => Box::pin(self.leave_wedding()).await?,
            Some(Effect::OpenRegistry { bride }) => Box::pin(self.open_registry(bride)).await?,
            Some(Effect::OpenGifts) => Box::pin(self.open_gifts()).await?,
            _ => {}
        }
        // On or off in the map being left, so its players see it.
        for effect in item_buffs {
            match *effect {
                Effect::ApplyItemBuff(item_id) => self.take_item_buff(item_id).await?,
                Effect::CancelItemBuff(item_id) => self.end_buff(-item_id).await?,
                _ => {}
            }
        }
        if let Some((map_id, portal)) = warp {
            let channel_id = self.channel_id;
            let player = &mut self.in_game_mut()?.player;
            let actions = match portal {
                Some(portal) => net::packet::handle::world::warp_to_spawn_point(
                    player, map_id, portal, channel_id,
                )?,
                None => net::packet::handle::world::warp(player, map_id, "", channel_id)?,
            };
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
