//! The v83 quest flow. The client renders the conversation from `Say.img`
//! and sends `QUEST_ACTION`; the server checks the request against the
//! catalog and the player's state before anything is persisted.

use crate::load;
use game::{InventoryChange, InventoryError, Player, QuestState, QuestStatus, SkillRecord};
pub use game_data::quests::{
    QuestCatalog, QuestItem, QuestPrerequisite, QuestSkill, QuestTemplate, StartRequirements,
};
use rand::Rng;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

pub fn catalog() -> Result<&'static QuestCatalog, &'static str> {
    static CATALOG: OnceLock<Result<QuestCatalog, String>> = OnceLock::new();
    load::once(&CATALOG, || {
        QuestCatalog::load_from_nx(load::asset_path(
            "RUSTMS_QUEST_NX_PATH",
            "assets/game-data/Quest.nx",
        ))
    })
}

/// A parsed `QUEST_ACTION`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestRequest {
    pub quest_id: i16,
    pub kind: QuestActionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestActionKind {
    Start {
        npc_id: i32,
    },
    /// `choice` is the picked reward's place among those offered, or -1.
    Complete {
        npc_id: i32,
        choice: i32,
    },
    Forfeit,
    ScriptStart {
        npc_id: i32,
    },
    ScriptEnd {
        npc_id: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestRejection {
    WrongNpc,
    WrongState,
    Job,
    Level,
    Prerequisite(i16),
    /// Missing (or, for a count of 0, still holding) a required item.
    Items(i32),
    /// Fewer kills of this mob than required.
    Mobs(i32),
    /// Lacking, or for a "must not have" entry having, this skill.
    Skill(i32),
    /// Less fame than the quest asks.
    Fame,
    /// Not wearing this required item, or (for `equipSelectNeed`, with 0)
    /// none of the listed ones.
    Worn(i32),
    /// Not on one of the quest's maps.
    Map,
    /// Before the quest's start date.
    Date,
    /// Fewer mesos than the quest takes.
    Mesos,
    /// Fewer maps visited than an explorer medal needs.
    Explored,
    /// The quest's own record short of its count (`end_record`).
    Record,
    /// The quest's info string doesn't read what completion needs, or a
    /// record doesn't read what the start needs (`StartRequirements::record`).
    Info,
    /// No pet out, or the first pet out isn't one the quest lists.
    Pet,
    /// Without an item's effect the quest needs (`buff`), or with one it
    /// rules out (`exceptbuff`).
    Buff,
    /// The Monster Book holds too few or too many cards (`mbmin`, `mbmax`),
    /// or the wrong count of one it lists (`mbcard`).
    MonsterBook,
    /// The mount's level is out of the quest's range
    /// (`tamingmoblevelmin`).
    MountLevel,
    /// Too few quests completed (`questComplete`).
    CompletedQuests,
    /// The item rewards don't fit.
    InventoryFull,
    /// A one-of-a-kind reward the player already has.
    OnlyOne,
    /// The picked reward isn't one of those offered.
    Choice,
}

/// Whether a quest completed `at` may be started again by `now`. Without an
/// interval, v83 restarts a `dayByDay` quest once the UTC date of `now`
/// passes the completion's, as it reads the FILETIMEs (FUN_00721163).
fn repeat_due(start: &StartRequirements, at: SystemTime, now: SystemTime) -> bool {
    const DAY: u64 = 24 * 60 * 60;
    let day = |time: SystemTime| {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() / DAY)
    };
    match start.repeat_interval {
        Some(minutes) => at + Duration::from_secs(u64::from(minutes) * 60) <= now,
        None => start.daily && day(at) < day(now),
    }
}

/// The day of the week of `now`, 0 (Sunday) to 6, in UTC. v83 tests
/// `dayOfWeek` against the PC's own weekday (GetLocalTime), which the server
/// can't know; it keeps to UTC, as for the quest dates.
fn weekday(now: SystemTime) -> u32 {
    let days = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / (24 * 60 * 60));
    // 1 January 1970 was a Thursday.
    ((days + 4) % 7) as u32
}

/// Whether `player` may start `quest` with `npc_id`.
pub fn check_start(
    quest: &QuestTemplate,
    npc_id: i32,
    player: &Player,
) -> Result<(), QuestRejection> {
    let start = &quest.start;
    if start.npc != npc_id {
        return Err(QuestRejection::WrongNpc);
    }
    match player.quests.status(quest.id) {
        None => {}
        // A repeatable quest starts again `interval` minutes after it was
        // last completed, and a daily one on a later day (v83 FUN_00721163).
        Some(QuestStatus::Completed { at }) if repeat_due(start, *at, SystemTime::now()) => {}
        Some(_) => return Err(QuestRejection::WrongState),
    }
    if !start.jobs.is_empty() && !start.jobs.contains(&player.stats.job) {
        return Err(QuestRejection::Job);
    }
    let level = i16::from(player.stats.level);
    if start.min_level.is_some_and(|min| level < min)
        || start.max_level.is_some_and(|max| level > max)
    {
        return Err(QuestRejection::Level);
    }
    if let Some(unmet) = start
        .quests
        .iter()
        .find(|p| player.quests.state(p.quest_id) != p.state)
    {
        return Err(QuestRejection::Prerequisite(unmet.quest_id));
    }
    if let Some((record, checks)) = &start.record {
        if !game_data::quests::info_met(player.quests.info(*record), checks) {
            return Err(QuestRejection::Info);
        }
    }
    // v83 compares fame unsigned, so negative fame passes.
    if start.min_fame != 0 && (player.stats.fame as u16) < (start.min_fame as u16) {
        return Err(QuestRejection::Fame);
    }
    for &(item_id, count) in &start.items {
        let held = held_for_start(player, item_id);
        let met = match count {
            1.. => held >= count,
            0 => held == 0,
            _ => held <= count.saturating_neg(),
        };
        if !met {
            return Err(QuestRejection::Items(item_id));
        }
    }
    if let Some(&(skill_id, _)) = start
        .skills
        .iter()
        .find(|&&(id, acquire)| has_skill(player, id) != acquire)
    {
        return Err(QuestRejection::Skill(skill_id));
    }
    let worn = |item_id: i32| player.inventory.equipped_ids().any(|(_, id)| id == item_id);
    if let Some(&item_id) = start.worn_all.iter().find(|&&id| !worn(id)) {
        return Err(QuestRejection::Worn(item_id));
    }
    if !start.worn_any.is_empty() && !start.worn_any.iter().any(|&id| worn(id)) {
        return Err(QuestRejection::Worn(0));
    }
    if !start.maps.is_empty() && !start.maps.contains(&player.stats.map_id) {
        return Err(QuestRejection::Map);
    }
    let now = SystemTime::now();
    if start.not_before.is_some_and(|at| now < at) || start.not_after.is_some_and(|at| now > at) {
        return Err(QuestRejection::Date);
    }
    if start
        .weekdays
        .is_some_and(|days| days & 1 << weekday(now) == 0)
    {
        return Err(QuestRejection::Date);
    }
    if !pet_met(
        &start.pets,
        start.pet_closeness,
        start.pet_skill_limit,
        player,
    ) {
        return Err(QuestRejection::Pet);
    }
    if start
        .mount_level
        .is_some_and(|(min, max)| !(min..=max).contains(&player.mount.level))
    {
        return Err(QuestRejection::MountLevel);
    }
    if !buffs_met(start.buff, start.except_buff, player) {
        return Err(QuestRejection::Buff);
    }
    // v83's client doesn't check; it has the server's refusal text.
    if quest.start_money < 0 && player.meso < quest.start_money.saturating_neg() {
        return Err(QuestRejection::Mesos);
    }
    Ok(())
}

/// Whether `player` meets a `pet` requirement listing `pets`, with its
/// `closeness` range (0 to 30000 when unset), and the `petSkill` bits
/// `skill_limit` the pet must lack: v83's checks (FUN_00721163,
/// FUN_00721d2c) look only at the first pet out, and at its closeness only
/// when the quest lists pets.
fn pet_met(pets: &[i32], closeness: Option<(i32, i32)>, skill_limit: u16, player: &Player) -> bool {
    if pets.is_empty() && skill_limit == 0 {
        return true;
    }
    let Some((item_id, pet)) = player
        .summoned_pet(0)
        .and_then(|item| Some((item.item_id, item.pet.as_ref()?)))
    else {
        return false;
    };
    let (min, max) = closeness.unwrap_or((0, 30_000));
    let listed = pets.is_empty()
        || (pets.contains(&item_id) && (min..=max).contains(&i32::from(pet.closeness)));
    listed && pet.skill & skill_limit == 0
}

/// Whether `player` has the effect of item `buff`, if set, and not that of
/// `except`: a buff from using the item, or from a quest's `buffItemID`,
/// which runs under the item's negated id. v83's checks test the same.
fn buffs_met(buff: Option<i32>, except: Option<i32>, player: &Player) -> bool {
    let now = Instant::now();
    let on = |item_id: i32| {
        player
            .buffs
            .active()
            .iter()
            .any(|b| b.skill_id == -item_id && b.expires > now)
    };
    buff.is_none_or(on) && !except.is_some_and(on)
}

/// Whether `player`'s Monster Book meets `quest`'s completion bounds: the
/// distinct cards it holds against `mbmin`/`mbmax`, and each listed card's
/// count against its own. v83's completion check (FUN_00721d2c) tests the
/// same, after the mesos.
fn book_met(quest: &QuestTemplate, player: &Player) -> bool {
    let within = |n: i32, min: Option<i32>, max: Option<i32>| {
        min.is_none_or(|min| n >= min) && max.is_none_or(|max| n <= max)
    };
    let book = &player.monster_book;
    let total = i32::try_from(book.cards().count()).unwrap_or(i32::MAX);
    within(total, quest.end_book_min, quest.end_book_max)
        && quest
            .end_cards
            .iter()
            .all(|c| within(i32::from(book.level(c.card)), c.min, c.max))
}

/// How many of `item_id` v83's start check sees `player` hold: those in the
/// bag, plus one if a copy is worn.
fn held_for_start(player: &Player, item_id: i32) -> i32 {
    let worn = player.inventory.equipped_ids().any(|(_, id)| id == item_id);
    player.inventory.count(item_id) + i32::from(worn)
}

/// Whether v83's start check counts `player` as having `skill_id`: a skill
/// they've learned, or any skill that isn't hidden from the skill window.
/// It looks at neither the level nor the job.
fn has_skill(player: &Player, skill_id: i32) -> bool {
    crate::skills::info(skill_id)
        .is_some_and(|info| !info.invisible || player.skills.get(skill_id).is_some())
}

/// Whether `player` may complete `quest` with `npc_id`. The start
/// requirements, including the level range, only apply to starting.
pub fn check_complete(
    quest: &QuestTemplate,
    npc_id: i32,
    player: &Player,
) -> Result<(), QuestRejection> {
    if quest.end_npc != npc_id {
        return Err(QuestRejection::WrongNpc);
    }
    if player.quests.state(quest.id) != QuestState::Started {
        return Err(QuestRejection::WrongState);
    }
    for &(item_id, count) in &quest.end_items {
        let held = player.inventory.count(item_id);
        let met = if count > 0 { held >= count } else { held == 0 };
        if !met {
            return Err(QuestRejection::Items(item_id));
        }
    }
    let kills = player.quests.kills(quest.id);
    for (index, &(mob_id, count)) in quest.end_mobs.iter().enumerate() {
        if kills.get(index).copied().unwrap_or(0) < count {
            return Err(QuestRejection::Mobs(mob_id));
        }
    }
    if let Some(unmet) = quest
        .end_quests
        .iter()
        .find(|p| player.quests.state(p.quest_id) != p.state)
    {
        return Err(QuestRejection::Prerequisite(unmet.quest_id));
    }
    if !quest.end_info.is_empty()
        && !game_data::quests::info_met(
            player
                .quests
                .info(quest.end_info_record.unwrap_or(quest.id)),
            &quest.end_info,
        )
    {
        return Err(QuestRejection::Info);
    }
    if player.meso < quest.end_meso
        || (quest.money < 0 && player.meso < quest.money.saturating_neg())
    {
        return Err(QuestRejection::Mesos);
    }
    if !book_met(quest, player) {
        return Err(QuestRejection::MonsterBook);
    }
    if quest.end_completed_quests.is_some_and(|least| {
        player.quests.completed().count() < usize::try_from(least).unwrap_or(0)
    }) {
        return Err(QuestRejection::CompletedQuests);
    }
    if quest
        .end_min_level
        .is_some_and(|min| i16::from(player.stats.level) < min)
    {
        return Err(QuestRejection::Level);
    }
    if !pet_met(
        &quest.end_pets,
        quest.end_pet_closeness,
        quest.end_pet_skill_limit,
        player,
    ) {
        return Err(QuestRejection::Pet);
    }
    if !buffs_met(quest.end_buff, quest.end_except_buff, player) {
        return Err(QuestRejection::Buff);
    }
    // At least the count, as v83's own check (cond 1) has it.
    if let Some(explored) = quest.explored {
        if (player.quests.explored(explored.record) as u64) < u64::from(explored.needed) {
            return Err(QuestRejection::Explored);
        }
    }
    if let Some(needed) = quest.end_record {
        if player.quests.kills(quest.id).first().copied().unwrap_or(0) < needed {
            return Err(QuestRejection::Record);
        }
    }
    Ok(())
}

/// Count a kill of `mob_id` toward every started quest that needs it.
/// Returns the quests whose progress changed.
pub fn count_kill<'a>(
    quest: impl Fn(i16) -> Option<&'a QuestTemplate>,
    player: &mut Player,
    mob_id: i32,
) -> Vec<i16> {
    let started: Vec<i16> = player.quests.started().map(|r| r.quest_id).collect();
    let mut changed = Vec::new();
    for quest_id in started {
        let Some(quest) = quest(quest_id) else {
            continue;
        };
        if !quest
            .end_mobs
            .iter()
            .any(|&(required, _)| required == mob_id)
        {
            continue;
        }
        let Some(kills) = player.quests.kills_mut(quest_id) else {
            continue;
        };
        kills.resize(quest.end_mobs.len(), 0);
        let mut counted = false;
        for (index, &(required, count)) in quest.end_mobs.iter().enumerate() {
            // Kills past the required count are not recorded.
            if required == mob_id && kills[index] < count {
                kills[index] += 1;
                counted = true;
            }
        }
        if counted {
            changed.push(quest_id);
        }
    }
    changed
}

/// The item changes a quest step makes for `player`, as `(item id, count,
/// minutes a gift lasts)`:
/// every entry for their job and gender without a `prop`, plus one pick
/// among those with a positive one, weighted by it, plus the `choice`th of
/// those with -1 that give something, if there are any (v83 numbers the
/// choices it shows the same way, FUN_0071808a). An entry without a count
/// takes all the player holds, and is left out if that's none.
pub fn pick_items(
    items: &[QuestItem],
    player: &Player,
    choice: i32,
    rng: &mut impl Rng,
) -> Result<Vec<(i32, i32, Option<i32>)>, QuestRejection> {
    let eligible: Vec<&QuestItem> = items
        .iter()
        .filter(|item| item.applies_to(player.stats.job, player.stats.gender))
        .collect();
    let choices: Vec<&&QuestItem> = eligible
        .iter()
        .filter(|item| item.prop.is_some_and(|p| p < 0) && item.count > 0)
        .collect();
    let mut picked: Vec<(i32, i32, Option<i32>)> = eligible
        .iter()
        .filter(|item| item.prop.is_none())
        .map(|item| match item.count {
            0 => (item.item_id, -player.inventory.count(item.item_id), None),
            count => (item.item_id, count, item.period),
        })
        .filter(|&(_, count, _)| count != 0)
        .collect();
    let weighted: Vec<&&QuestItem> = eligible
        .iter()
        .filter(|item| item.prop.is_some_and(|w| w > 0))
        .collect();
    let total: i32 = weighted.iter().filter_map(|item| item.prop).sum();
    if total > 0 {
        let mut roll = rng.gen_range(0, total);
        for item in weighted {
            let weight = item.prop.unwrap_or(0);
            if roll < weight {
                picked.push((item.item_id, item.count, item.period));
                break;
            }
            roll -= weight;
        }
    }
    if !choices.is_empty() {
        let chosen = usize::try_from(choice)
            .ok()
            .and_then(|index| choices.get(index))
            .ok_or(QuestRejection::Choice)?;
        picked.push((chosen.item_id, chosen.count, chosen.period));
    }
    Ok(picked)
}

/// Take, then give, `items` (negative counts take), a gift with minutes
/// lasting that long from `now`. Takes are capped at what the player holds.
/// Fails without a partial result if a gift doesn't fit; the caller applies
/// this to a copy of the player.
pub fn apply_items(
    player: &mut Player,
    items: &[(i32, i32, Option<i32>)],
    now: SystemTime,
) -> Result<Vec<InventoryChange>, QuestRejection> {
    let mut changes = Vec::new();
    for &(item_id, count, _) in items.iter().filter(|(_, count, _)| *count < 0) {
        let held = player.inventory.count(item_id);
        let taken = i16::try_from(held.min(-count)).unwrap_or(i16::MAX);
        if taken > 0 {
            changes.extend(
                player
                    .inventory
                    .remove(item_id, taken)
                    .map_err(|_| QuestRejection::Items(item_id))?,
            );
        }
    }
    for &(item_id, count, minutes) in items.iter().filter(|(_, count, _)| *count > 0) {
        let quantity = i16::try_from(count).unwrap_or(i16::MAX);
        if crate::items::refuses_only(&player.inventory, item_id, count) {
            return Err(QuestRejection::OnlyOne);
        }
        let until = minutes.map(|m| now + Duration::from_secs(60 * u64::try_from(m).unwrap_or(0)));
        let given = crate::items::give_until(&mut player.inventory, item_id, quantity, until);
        changes.extend(given.map_err(|error| match error {
            InventoryError::UnknownItem => QuestRejection::Items(item_id),
            _ => QuestRejection::InventoryFull,
        })?);
    }
    Ok(changes)
}

/// Teach `player` the skills for their exact job, raising the level and the
/// master level to the entry's where they are lower (as Cosmic's
/// SkillAction does). The records that changed, for UPDATE_SKILLS.
pub fn apply_skills(skills: &[QuestSkill], player: &mut Player) -> Vec<SkillRecord> {
    let mut taught = Vec::new();
    for skill in skills
        .iter()
        .filter(|skill| skill.jobs.contains(&player.stats.job))
    {
        let known = player.skills.get(skill.skill_id).cloned();
        let record = SkillRecord {
            skill_id: skill.skill_id,
            level: skill.level.max(known.as_ref().map_or(0, |k| k.level)),
            master_level: skill
                .master_level
                .max(known.as_ref().map_or(0, |k| k.master_level)),
            expiration: known.as_ref().and_then(|k| k.expiration),
        };
        if known.as_ref() != Some(&record) {
            player.skills.set(record.clone());
            taught.push(record);
        }
    }
    taught
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::QuestRecord;

    fn quest() -> QuestTemplate {
        QuestTemplate {
            id: 1033,
            name: "What Sen Wants".into(),
            start: StartRequirements {
                npc: 2001,
                jobs: vec![0],
                min_level: None,
                max_level: Some(10),
                quests: vec![QuestPrerequisite {
                    quest_id: 1032,
                    state: QuestState::Completed,
                }],
                ..StartRequirements::default()
            },
            end_npc: 2102,
            end_items: vec![],
            end_mobs: vec![],
            end_quests: vec![],
            end_meso: 0,
            end_min_level: None,
            end_pets: vec![],
            end_pet_closeness: None,
            end_pet_skill_limit: 0,
            end_buff: None,
            end_except_buff: None,
            end_book_min: None,
            end_book_max: None,
            end_cards: vec![],
            end_completed_quests: None,
            start_items: vec![],
            end_rewards: vec![],
            start_exp: 0,
            start_money: 0,
            start_buff_item: None,
            end_buff_item: None,
            end_skills: vec![],
            start_script: None,
            end_script: None,
            exp: 7,
            money: 0,
            fame: 0,
            pet_closeness: 0,
            pet_skill: 0,
            pet_speed: false,
            next_quest: None,
            auto_start: false,
            auto_complete: false,
            auto_pre_complete: false,
            explored: None,
            end_record: None,
            start_info: None,
            end_info: Vec::new(),
            end_info_record: None,
            time_limit: None,
        }
    }

    fn player(level: u8, job: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        player.stats.job = job;
        player
    }

    fn set(player: &mut Player, quest_id: i16, state: QuestState) {
        let status = match state {
            QuestState::NotStarted => return player.quests.remove(quest_id),
            QuestState::Started => QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
            QuestState::Completed => QuestStatus::Completed {
                at: SystemTime::UNIX_EPOCH,
            },
        };
        player.quests.upsert(QuestRecord { quest_id, status });
    }

    #[test]
    fn start_checks_npc_state_job_level_and_prerequisites() {
        let q = quest();
        let mut p = player(1, 0);
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Prerequisite(1032))
        );
        set(&mut p, 1032, QuestState::Completed);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        assert_eq!(check_start(&q, 2102, &p), Err(QuestRejection::WrongNpc));
        assert_eq!(
            check_start(&q, 2001, &player(11, 0)),
            Err(QuestRejection::Level)
        );
        let mut warrior = player(1, 100);
        set(&mut warrior, 1032, QuestState::Completed);
        assert_eq!(check_start(&q, 2001, &warrior), Err(QuestRejection::Job));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
    }

    #[test]
    fn complete_needs_a_started_quest_at_its_end_npc_and_ignores_level() {
        let q = quest();
        let mut p = player(30, 0);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::WrongState)
        );
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2001, &p), Err(QuestRejection::WrongNpc));
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        set(&mut p, 1033, QuestState::Completed);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::WrongState)
        );
    }

    #[test]
    fn a_quest_that_takes_mesos_needs_them_held() {
        let mut q = quest();
        q.money = -200;
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        p.meso = 199;
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Mesos));
        p.meso = 200;
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        q.money = 200;
        p.meso = 0;
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        q.end_meso = 1;
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Mesos));
    }

    #[test]
    fn an_explorer_title_needs_its_places_seen() {
        let mut q = quest();
        q.explored = Some(game_data::quests::Explored {
            record: 27010,
            needed: 2,
        });
        let mut p = player(30, 0);
        set(&mut p, 1033, QuestState::Started);
        p.quests.explore(27010, 100000000);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Explored));
        p.quests.explore(27010, 101000000);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn an_own_record_quest_needs_its_count() {
        let mut q = quest();
        q.end_record = Some(1);
        let mut p = player(30, 0);
        set(&mut p, q.id, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Record));
        *p.quests.kills_mut(q.id).unwrap() = vec![1];
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn a_pet_requirement_looks_at_the_first_pet_out() {
        let mut q = quest();
        q.start.max_level = None;
        q.start.quests.clear();
        q.start.pets = vec![5000000, 5000001];
        q.end_pets = q.start.pets.clone();
        let mut p = player(30, 0);
        let pet = |serial, item_id| game::Item {
            cash_serial: Some(serial),
            pet: Some(game::PetData::new("Pet")),
            ..game::Item::stack(item_id, 1)
        };
        p.inventory
            .insert(game::InventoryType::Cash, 1, pet(7, 5000000));
        p.inventory
            .insert(game::InventoryType::Cash, 2, pet(8, 5000066));
        // Carried but not out.
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Pet));
        // Out, but not first.
        p.pets = vec![8, 7];
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Pet));
        p.pets = vec![7, 8];
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        p.pets = vec![8];
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Pet));
        p.pets.clear();
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Pet));
    }

    #[test]
    fn a_closeness_requirement_looks_at_the_first_pet_out() {
        let mut q = quest();
        q.start.max_level = None;
        q.start.quests.clear();
        q.start.pets = vec![5000029];
        q.start.pet_closeness = Some((1642, 30_000));
        q.end_pets = vec![5000029];
        q.end_pet_closeness = Some((1642, 30_000));
        let mut p = player(30, 0);
        let mut baby = game::Item::stack(5000029, 1);
        baby.cash_serial = Some(7);
        baby.pet = Some(game::PetData::new("Baby Dragon"));
        baby.pet.as_mut().unwrap().closeness = 1641;
        p.inventory.insert(game::InventoryType::Cash, 1, baby);
        p.pets = vec![7];
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Pet));
        let mut closer = p
            .inventory
            .get(game::InventoryType::Cash, 1)
            .unwrap()
            .clone();
        closer.pet.as_mut().unwrap().closeness = 1642;
        p.inventory.insert(game::InventoryType::Cash, 1, closer);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        p.pets.clear();
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Pet));
        // Without a pet list, v83 doesn't read the range.
        q.end_pets.clear();
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn a_pet_skill_quest_needs_a_pet_without_the_skill() {
        let mut q = quest();
        q.start.max_level = None;
        q.start.quests.clear();
        q.start.pet_skill_limit = game_data::quests::PET_AUTO_SPEAKING;
        q.end_pet_skill_limit = game_data::quests::PET_AUTO_SPEAKING;
        let mut p = player(30, 0);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Pet));
        let mut robo = game::Item::stack(5000048, 1);
        robo.cash_serial = Some(7);
        robo.pet = Some(game::PetData::new("Robo"));
        robo.pet.as_mut().unwrap().skill = game_data::quests::PET_RECALL;
        p.inventory
            .insert(game::InventoryType::Cash, 1, robo.clone());
        p.pets = vec![7];
        // Recall is another skill; any pet will do without a pet list.
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        robo.pet.as_mut().unwrap().skill |= game_data::quests::PET_AUTO_SPEAKING;
        p.inventory.insert(game::InventoryType::Cash, 1, robo);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Pet));
    }

    #[test]
    fn a_mount_level_requirement_bounds_the_mount() {
        let mut q = quest();
        q.start.max_level = None;
        q.start.quests.clear();
        q.start.mount_level = Some((3, 30));
        let mut p = player(30, 0);
        p.mount.level = 2;
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::MountLevel));
        p.mount.level = 3;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
    }

    #[test]
    fn a_scent_needs_its_own_buff_and_not_the_last_one() {
        let mut q = quest();
        q.start.max_level = None;
        q.start.quests.clear();
        q.start.except_buff = Some(2022631);
        q.end_buff = Some(2022632);
        let mut p = player(30, 0);
        let scent = |item_id: i32| {
            game::Buff::new(
                -item_id,
                game::BuffStat::Speed,
                -5,
                Instant::now(),
                Duration::from_secs(40),
            )
        };
        p.buffs.give(scent(2022631));
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Buff));
        p.buffs.cancel(-2022631);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Buff));
        p.buffs.give(scent(2022632));
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        // Another item's effect on the same stat isn't this one.
        p.buffs.give(scent(2022633));
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Buff));
    }

    #[test]
    fn complete_needs_enough_quests_completed() {
        let mut q = quest();
        q.end_completed_quests = Some(2);
        let mut p = player(30, 0);
        set(&mut p, 1033, QuestState::Started);
        set(&mut p, 1000, QuestState::Completed);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::CompletedQuests)
        );
        set(&mut p, 1001, QuestState::Completed);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn complete_needs_the_monster_book_within_its_bounds() {
        let mut q = quest();
        q.end_book_min = Some(2);
        q.end_cards = vec![game_data::quests::BookCard {
            card: 2380000,
            min: Some(2),
            max: None,
        }];
        let mut p = player(30, 0);
        set(&mut p, 1033, QuestState::Started);
        p.monster_book.add(2380000);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::MonsterBook)
        );
        // Two distinct cards, but only one of the listed card.
        p.monster_book.add(2380001);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::MonsterBook)
        );
        p.monster_book.add(2380000);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        q.end_book_max = Some(1);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::MonsterBook)
        );
    }

    #[test]
    fn complete_needs_the_info_string_to_read_its_value() {
        let mut q = quest();
        q.end_info = vec![game_data::quests::InfoCheck {
            cond: 0,
            value: "3333".into(),
        }];
        let mut p = player(30, 0);
        let mut started = |info: Option<&str>| {
            p.quests.upsert(QuestRecord {
                quest_id: q.id,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: info.map(String::from),
                },
            });
            check_complete(&q, 2102, &p)
        };
        assert_eq!(started(Some("3323")), Err(QuestRejection::Info));
        assert_eq!(started(None), Err(QuestRejection::Info));
        assert_eq!(started(Some("3333")), Ok(()));
    }

    #[test]
    fn complete_reads_another_record_when_the_data_names_one() {
        let mut q = quest();
        q.end_info = vec![game_data::quests::InfoCheck {
            cond: 0,
            value: "q3".into(),
        }];
        q.end_info_record = Some(6401);
        let mut p = player(120, 0);
        set(&mut p, q.id, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Info));
        assert!(p.quests.set_record(6401, "q3"));
        assert!(!p.quests.set_record(6401, "q3"));
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn start_reads_a_record_when_the_data_names_one() {
        let mut q = quest();
        q.start.record = Some((
            2311,
            vec![game_data::quests::InfoCheck {
                cond: 1,
                value: "1".into(),
            }],
        ));
        let npc = q.start.npc;
        let mut p = player(10, 0);
        set(&mut p, 1032, QuestState::Completed);
        assert_eq!(check_start(&q, npc, &p), Err(QuestRejection::Info));
        p.quests.set_record(2311, "0");
        assert_eq!(check_start(&q, npc, &p), Err(QuestRejection::Info));
        p.quests.set_record(2311, "1");
        assert_eq!(check_start(&q, npc, &p), Ok(()));
    }

    #[test]
    fn complete_checks_the_end_level() {
        let mut q = quest();
        q.end_min_level = Some(200);
        let mut p = player(199, 0);
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(check_complete(&q, 2102, &p), Err(QuestRejection::Level));
        p.stats.level = 200;
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn completion_checks_item_counts_and_zero_means_none_left() {
        let mut q = quest();
        q.end_items = vec![(4031792, 2), (2010007, 0)];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Items(4031792))
        );
        p.inventory.add(game::Item::stack(4031792, 2), 100).unwrap();
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
        p.inventory.add(game::Item::stack(2010007, 1), 100).unwrap();
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Items(2010007))
        );
    }

    #[test]
    fn completion_needs_the_listed_quests_in_their_states() {
        let mut q = quest();
        q.end_quests = vec![
            QuestPrerequisite {
                quest_id: 1041,
                state: QuestState::Completed,
            },
            QuestPrerequisite {
                quest_id: 1042,
                state: QuestState::Completed,
            },
        ];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        set(&mut p, 1041, QuestState::Completed);
        set(&mut p, 1042, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Prerequisite(1042))
        );
        set(&mut p, 1042, QuestState::Completed);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn mutually_exclusive_quests_require_the_others_unstarted() {
        let mut q = quest();
        q.start.quests = vec![QuestPrerequisite {
            quest_id: 2078,
            state: QuestState::NotStarted,
        }];
        let mut p = player(1, 0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, 2078, QuestState::Started);
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Prerequisite(2078))
        );
    }

    #[test]
    fn start_items_count_worn_copies_and_bound_by_sign() {
        let mut q = quest();
        q.start.quests.clear();
        q.start.items = vec![(1002067, 2), (2010007, 0), (4000381, -1)];
        let mut p = player(1, 0);
        p.inventory.insert(
            game::InventoryType::Equipped,
            game::inventory::equip_slot::CAP,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(1002067))
        );
        p.inventory.insert(
            game::InventoryType::Equip,
            1,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.inventory.add(game::Item::stack(4000381, 1), 100).unwrap();
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.inventory.add(game::Item::stack(4000381, 1), 100).unwrap();
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(4000381))
        );
        q.start.items.truncate(2);
        p.inventory.add(game::Item::stack(2010007, 1), 100).unwrap();
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Items(2010007))
        );
    }

    #[test]
    fn repeatable_quests_restart_once_their_interval_has_passed() {
        let mut q = quest();
        q.start.quests.clear();
        let mut p = player(1, 0);
        let id = q.id;
        let completed = move |p: &mut Player, ago: u64| {
            p.quests.upsert(QuestRecord {
                quest_id: id,
                status: QuestStatus::Completed {
                    at: SystemTime::now() - Duration::from_secs(ago),
                },
            })
        };
        completed(&mut p, 0);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
        q.start.repeat_interval = Some(0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        q.start.repeat_interval = Some(60);
        completed(&mut p, 59 * 60);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
        completed(&mut p, 61 * 60);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        set(&mut p, q.id, QuestState::Started);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::WrongState));
    }

    #[test]
    fn daily_quests_restart_on_a_later_utc_day() {
        let day = Duration::from_secs(24 * 60 * 60);
        let midnight = SystemTime::UNIX_EPOCH + day * 20_000;
        let at = |secs: u64| midnight + Duration::from_secs(secs);
        let mut start = StartRequirements::default();
        assert!(!repeat_due(&start, at(0), at(0) + day * 3));
        start.daily = true;
        assert!(!repeat_due(&start, at(1), at(86_399)), "the same day");
        assert!(repeat_due(&start, at(86_399), at(86_400)), "the next");
        // An interval, even one under a day, wins as in v83 (none has both).
        start.repeat_interval = Some(1);
        assert!(repeat_due(&start, at(0), at(60)));
    }

    #[test]
    fn start_fame_is_compared_unsigned() {
        let mut q = quest();
        q.start.quests.clear();
        q.start.min_fame = 1;
        let mut p = player(1, 0);
        p.stats.fame = 0;
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Fame));
        p.stats.fame = 1;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.stats.fame = -3;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
    }

    #[test]
    fn start_keeps_to_the_date_window() {
        let mut q = quest();
        q.start.quests.clear();
        let p = player(1, 0);
        q.start.not_before = Some(SystemTime::now() + Duration::from_secs(3600));
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Date));
        q.start.not_before = Some(SystemTime::now() - Duration::from_secs(3600));
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        q.start.not_after = Some(SystemTime::now() - Duration::from_secs(1));
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Date));
        q.start.not_after = Some(SystemTime::now() + Duration::from_secs(3600));
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
    }

    #[test]
    fn start_keeps_to_its_weekdays() {
        // 2026-10-25 was a Sunday, and 2000-01-01 a Saturday.
        let day = |days: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(days * 86_400 + 3_600);
        assert_eq!(weekday(day(20_751)), 0);
        assert_eq!(weekday(day(20_752)), 1);
        assert_eq!(weekday(day(10_957)), 6);

        let mut q = quest();
        q.start.quests.clear();
        let p = player(1, 0);
        let today = 1 << weekday(SystemTime::now());
        q.start.weekdays = Some(0x7f & !today);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Date));
        q.start.weekdays = Some(today);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        q.start.weekdays = Some(0);
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Date));
    }

    #[test]
    fn a_quest_that_takes_mesos_to_start_needs_them_held() {
        let mut q = quest();
        q.start.quests.clear();
        q.start_money = -1000;
        let mut p = player(1, 0);
        p.meso = 999;
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Mesos));
        p.meso = 1000;
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
    }

    #[test]
    fn start_checks_worn_items_and_the_map() {
        let mut q = quest();
        q.start.quests.clear();
        q.start.worn_all = vec![1002067];
        q.start.worn_any = vec![1102000, 1102135];
        q.start.maps = vec![261000000];
        let mut p = player(1, 0);
        p.stats.map_id = 261000000;
        // Holding a copy in the bag isn't wearing it.
        p.inventory.insert(
            game::InventoryType::Equip,
            1,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Worn(1002067))
        );
        p.inventory.insert(
            game::InventoryType::Equipped,
            game::inventory::equip_slot::CAP,
            game::Item::equipment(1002067, game::EquipStats::default()),
        );
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Worn(0)));
        p.inventory.insert(
            game::InventoryType::Equipped,
            game::inventory::equip_slot::CAPE,
            game::Item::equipment(1102135, game::EquipStats::default()),
        );
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.stats.map_id = 261000001;
        assert_eq!(check_start(&q, 2001, &p), Err(QuestRejection::Map));
    }

    #[test]
    fn start_skills_count_any_visible_skill_and_learned_hidden_ones() {
        if crate::skills::data().is_err() {
            return;
        }
        let mut q = quest();
        q.start.quests.clear();
        // Power Strike (visible) needed; Maple Warrior (hidden) must be lacked.
        q.start.skills = vec![(1001004, true), (1121000, false)];
        let mut p = player(1, 0);
        assert_eq!(check_start(&q, 2001, &p), Ok(()));
        p.skills.set(SkillRecord {
            skill_id: 1121000,
            level: 0,
            master_level: 10,
            expiration: None,
        });
        assert_eq!(
            check_start(&q, 2001, &p),
            Err(QuestRejection::Skill(1121000))
        );
    }

    #[test]
    fn kills_count_toward_started_quests_up_to_the_required_number() {
        let mut q = quest();
        q.end_mobs = vec![(100100, 2), (100101, 1)];
        let mut p = player(1, 0);
        set(&mut p, 1033, QuestState::Started);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Mobs(100100))
        );
        let catalog = |id| (id == 1033).then_some(&q);
        assert_eq!(count_kill(catalog, &mut p, 100100), [1033]);
        assert_eq!(count_kill(catalog, &mut p, 100100), [1033]);
        assert!(count_kill(catalog, &mut p, 100100).is_empty());
        assert!(count_kill(catalog, &mut p, 9300018).is_empty());
        assert_eq!(p.quests.kills(1033), [2, 0]);
        assert_eq!(
            check_complete(&q, 2102, &p),
            Err(QuestRejection::Mobs(100101))
        );
        assert_eq!(count_kill(catalog, &mut p, 100101), [1033]);
        assert_eq!(check_complete(&q, 2102, &p), Ok(()));
    }

    #[test]
    fn rewards_filter_by_job_and_gender_and_pick_one_weighted_entry() {
        use rand::SeedableRng;
        let item = |item_id, job, gender, prop| QuestItem {
            item_id,
            count: 1,
            job,
            gender,
            prop,
            period: None,
        };
        let items = [
            item(1002053, 1, 2, Some(1)),
            item(1002008, 1, 2, Some(1)),
            item(1302000, 2, 2, None),
            item(1041002, 0, 1, None),
            QuestItem {
                count: -1,
                ..item(4031802, 0, 2, None)
            },
            // Without a count: all held, or nothing.
            QuestItem {
                count: 0,
                ..item(4031792, 0, 2, None)
            },
            QuestItem {
                count: 0,
                ..item(4031856, 0, 2, None)
            },
        ];
        let mut beginner = player(1, 0);
        beginner.stats.gender = 0;
        beginner
            .inventory
            .add(game::Item::stack(4031792, 17), 100)
            .unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut hats = std::collections::BTreeSet::new();
        for _ in 0..50 {
            let picked = pick_items(&items, &beginner, -1, &mut rng).unwrap();
            // The shellpiece and every held 4031792 are always taken;
            // exactly one hat; no sword (warriors only) and no skirt
            // (female only).
            assert_eq!(picked.len(), 3);
            assert!(picked.contains(&(4031802, -1, None)));
            assert!(picked.contains(&(4031792, -17, None)));
            hats.insert(picked.iter().find(|(_, c, _)| *c > 0).unwrap().0);
        }
        assert_eq!(hats.len(), 2);
    }

    #[test]
    fn the_picked_reward_counts_only_choices_offered() {
        let choice = |item_id, count, job| QuestItem {
            item_id,
            count,
            job,
            gender: 2,
            prop: Some(-1),
            period: None,
        };
        let items = [
            QuestItem {
                prop: None,
                ..choice(4031223, -1, 0)
            },
            choice(1072133, 1, 0),
            // Not offered: nothing given, or for warriors only.
            choice(1072122, 0, 0),
            choice(1072303, 1, 2),
            choice(1072129, 1, 0),
        ];
        let beginner = player(37, 0);
        let mut rng = rand::thread_rng();
        let mut pick = |choice| pick_items(&items, &beginner, choice, &mut rng);
        assert_eq!(pick(1), Ok(vec![(4031223, -1, None), (1072129, 1, None)]));
        assert_eq!(pick(0), Ok(vec![(4031223, -1, None), (1072133, 1, None)]));
        assert_eq!(pick(2), Err(QuestRejection::Choice));
        assert_eq!(pick(-1), Err(QuestRejection::Choice));
        let warrior = player(37, 100);
        assert_eq!(
            pick_items(&items, &warrior, 1, &mut rand::thread_rng()),
            Ok(vec![(4031223, -1, None), (1072303, 1, None)])
        );
    }

    #[test]
    fn a_gift_with_minutes_comes_dated_and_on_its_own() {
        if crate::items::data().is_err() {
            return;
        }
        let mut p = player(70, 0);
        let now = SystemTime::now();
        let gifts = [(4031281, 1, Some(60)), (4031281, 1, None)];
        assert!(apply_items(&mut p, &gifts, now).is_ok());
        let dated: Vec<_> = p
            .inventory
            .items(game::InventoryType::Etc)
            .map(|(_, item)| item.expiration)
            .collect();
        assert_eq!(dated, [Some(now + Duration::from_secs(3600)), None]);
    }

    #[test]
    fn skill_rewards_match_the_exact_job_and_never_lower_a_skill() {
        let ambush = |skill_id, job| QuestSkill {
            skill_id,
            level: 0,
            master_level: 10,
            jobs: vec![job, 0],
        };
        let skills = [ambush(4221004, 422), ambush(4121004, 412)];
        let mut p = player(130, 412);
        let taught = apply_skills(&skills, &mut p);
        let expected = SkillRecord {
            skill_id: 4121004,
            level: 0,
            master_level: 10,
            expiration: None,
        };
        assert_eq!(taught, [expected.clone()]);
        assert_eq!(p.skills.get(4121004), Some(&expected));
        assert!(p.skills.get(4221004).is_none());
        // A known skill keeps its higher level and master level.
        p.skills.set(SkillRecord {
            level: 12,
            master_level: 20,
            ..expected
        });
        assert!(apply_skills(&skills, &mut p).is_empty());
        assert_eq!(p.skills.level(4121004), 12);
        // Job 410 is not 412, though both are thieves.
        assert!(apply_skills(&skills, &mut player(70, 410)).is_empty());
    }
}
