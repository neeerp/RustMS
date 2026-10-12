//! Quest definitions from `Quest.nx`, classified against what the server can
//! check and grant.
//!
//! In v83 the client renders quest conversations (`Say.img`) itself and only
//! asks the server to start or complete a quest, so the server reads the
//! requirements (`Check.img`) and actions (`Act.img`). A quest is supported
//! only if every requirement and action it declares is one the server
//! implements; anything else would be silently skipped, so such quests are
//! refused instead. HeavenClient offers only the supported quests, which it
//! reads from `supported_quests.txt`.

use crate::{GameDataError, NxDataFile};
use game::QuestState;
use std::collections::BTreeMap;
use std::path::Path;

/// Requirements to start a quest (`Check.img/<id>/0`) that the server checks.
pub const START_REQUIREMENTS: &[&str] = &["npc", "job", "lvmin", "lvmax", "quest", "startscript"];
/// Requirements to complete a quest (`Check.img/<id>/1`) that the server checks.
pub const END_REQUIREMENTS: &[&str] = &["npc", "item", "endscript"];
/// Quest scripts (`startscript`/`endscript`) the server implements. A quest
/// naming any other script is unsupported.
pub const SCRIPTS: &[&str] = &["q1021s", "q1021e"];
/// Actions on starting a quest (`Act.img/<id>/0`) that the server applies.
pub const START_ACTIONS: &[&str] = &[];
/// Actions on completing a quest (`Act.img/<id>/1`) that the server applies.
pub const END_ACTIONS: &[&str] = &["exp", "nextQuest"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestPrerequisite {
    pub quest_id: i16,
    pub state: QuestState,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartRequirements {
    pub npc: i32,
    /// Allowed job ids; empty means any job.
    pub jobs: Vec<i16>,
    pub min_level: Option<i16>,
    pub max_level: Option<i16>,
    pub quests: Vec<QuestPrerequisite>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestTemplate {
    pub id: i16,
    pub name: String,
    pub start: StartRequirements,
    pub end_npc: i32,
    /// Items needed to complete, as `(item id, count)`. A count of 0 means
    /// the player must have none left (e.g. Roger's apples, all eaten).
    pub end_items: Vec<(i32, i32)>,
    /// Scripts that run the start or completion instead of the quest data.
    pub start_script: Option<String>,
    pub end_script: Option<String>,
    pub exp: i32,
    pub next_quest: Option<i16>,
}

/// The keys a quest declares in each `Check`/`Act` node, used to decide
/// whether the server supports it.
#[derive(Clone, Debug, Default)]
pub struct QuestKeys {
    pub start_check: Option<Vec<String>>,
    pub end_check: Option<Vec<String>>,
    pub start_act: Vec<String>,
    pub end_act: Vec<String>,
}

/// `Ok` if every declared requirement and action is supported, otherwise the
/// first unsupported key.
pub fn classify(keys: &QuestKeys) -> Result<(), String> {
    let start = keys.start_check.as_deref().ok_or("no start requirements")?;
    let end = keys
        .end_check
        .as_deref()
        .ok_or("no completion requirements")?;
    if !start.iter().any(|k| k == "npc") {
        return Err("no start NPC".into());
    }
    if !end.iter().any(|k| k == "npc") {
        return Err("no completion NPC".into());
    }
    unsupported(start, START_REQUIREMENTS, "Check/0")?;
    unsupported(end, END_REQUIREMENTS, "Check/1")?;
    unsupported(&keys.start_act, START_ACTIONS, "Act/0")?;
    unsupported(&keys.end_act, END_ACTIONS, "Act/1")?;
    Ok(())
}

fn unsupported(keys: &[String], allowed: &[&str], node: &str) -> Result<(), String> {
    match keys
        .iter()
        .find(|k| !allowed.contains(&k.as_str()) && !is_speech(k))
    {
        Some(key) => Err(format!("{node}/{key}")),
        None => Ok(()),
    }
}

/// `Act` nodes may repeat conversation text under numbered keys and `yes`/
/// `no`; that is speech, not an action.
fn is_speech(key: &str) -> bool {
    key.bytes().all(|b| b.is_ascii_digit()) || key == "yes" || key == "no"
}

#[derive(Debug, Default)]
pub struct QuestCatalog {
    supported: BTreeMap<i16, QuestTemplate>,
    unsupported: BTreeMap<i16, String>,
}

impl QuestCatalog {
    pub fn load_from_nx(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut catalog = Self::default();
        let checks = node(&nx, "Check.img")?;
        for quest in nx.child_indices(checks)? {
            let Ok(id) = nx.node_name(quest)?.parse::<i16>() else {
                continue;
            };
            let keys = QuestKeys {
                start_check: keys_at(&nx, &format!("Check.img/{id}/0"))?,
                end_check: keys_at(&nx, &format!("Check.img/{id}/1"))?,
                start_act: keys_at(&nx, &format!("Act.img/{id}/0"))?.unwrap_or_default(),
                end_act: keys_at(&nx, &format!("Act.img/{id}/1"))?.unwrap_or_default(),
            };
            match classify(&keys).and_then(|()| template(&nx, id).map_err(|e| e.to_string())) {
                Ok(template) => {
                    catalog.supported.insert(id, template);
                }
                Err(reason) => {
                    catalog.unsupported.insert(id, reason);
                }
            }
        }
        Ok(catalog)
    }

    pub fn quest(&self, id: i16) -> Option<&QuestTemplate> {
        self.supported.get(&id)
    }

    pub fn supported(&self) -> impl Iterator<Item = &QuestTemplate> {
        self.supported.values()
    }

    /// Why a quest in the data is not supported, if it is not.
    pub fn unsupported_reason(&self, id: i16) -> Option<&str> {
        self.unsupported.get(&id).map(String::as_str)
    }
}

fn template(nx: &NxDataFile, id: i16) -> Result<QuestTemplate, GameDataError> {
    let start = node(nx, &format!("Check.img/{id}/0"))?;
    let end = node(nx, &format!("Check.img/{id}/1"))?;
    let mut requirements = StartRequirements {
        npc: nx
            .int_child(start, "npc")?
            .ok_or_else(|| invalid("missing start NPC"))?,
        min_level: nx
            .int_child(start, "lvmin")?
            .map(|v| small(v, "level"))
            .transpose()?,
        max_level: nx
            .int_child(start, "lvmax")?
            .map(|v| small(v, "level"))
            .transpose()?,
        ..StartRequirements::default()
    };
    if let Some(jobs) = nx.child_by_name(start, "job")? {
        for job in nx.child_indices(jobs)? {
            let name = nx.node_name(job)?;
            let value = nx
                .int_child(jobs, &name)?
                .ok_or_else(|| invalid("non-integer job requirement"))?;
            requirements.jobs.push(small(value, "job")?);
        }
    }
    if let Some(quests) = nx.child_by_name(start, "quest")? {
        for prerequisite in nx.child_indices(quests)? {
            let quest_id = nx
                .int_child(prerequisite, "id")?
                .ok_or_else(|| invalid("prerequisite without id"))?;
            // A prerequisite without a state must not have been started; the
            // data uses this for mutually exclusive quests.
            let state = nx
                .int_child(prerequisite, "state")?
                .unwrap_or(QuestState::NotStarted as i32);
            requirements.quests.push(QuestPrerequisite {
                quest_id: small(quest_id, "quest id")?,
                state: QuestState::from_number(state)
                    .ok_or_else(|| invalid("invalid prerequisite state"))?,
            });
        }
    }
    let mut end_items = Vec::new();
    if let Some(items) = nx.child_by_name(end, "item")? {
        for item in nx.child_indices(items)? {
            let item_id = nx
                .int_child(item, "id")?
                .ok_or_else(|| invalid("item requirement without id"))?;
            end_items.push((item_id, nx.int_child(item, "count")?.unwrap_or(0)));
        }
    }
    let actions = path_index(nx, &format!("Act.img/{id}/1"))?;
    let action_int = |key: &str| match actions {
        Some(index) => nx.int_child(index, key),
        None => Ok(None),
    };
    let exp = action_int("exp")?.unwrap_or(0);
    if exp < 0 {
        return Err(invalid("negative EXP reward"));
    }
    let name = match path_index(nx, &format!("QuestInfo.img/{id}"))? {
        Some(info) => nx.string_child(info, "name")?.unwrap_or_default(),
        None => String::new(),
    };
    Ok(QuestTemplate {
        id,
        name,
        start: requirements,
        end_npc: nx
            .int_child(end, "npc")?
            .ok_or_else(|| invalid("missing completion NPC"))?,
        end_items,
        start_script: script(nx, start, "startscript")?,
        end_script: script(nx, end, "endscript")?,
        exp,
        next_quest: action_int("nextQuest")?
            .map(|id| small(id, "quest id"))
            .transpose()?,
    })
}

/// A supported script name, if the node names one.
fn script(nx: &NxDataFile, parent: u32, key: &str) -> Result<Option<String>, GameDataError> {
    match nx.string_child(parent, key)? {
        None => Ok(None),
        Some(name) if SCRIPTS.contains(&name.as_str()) => Ok(Some(name)),
        Some(name) => Err(invalid(&format!("unsupported script {name}"))),
    }
}

fn keys_at(nx: &NxDataFile, path: &str) -> Result<Option<Vec<String>>, GameDataError> {
    match path_index(nx, path)? {
        Some(index) => keys(nx, index).map(Some),
        None => Ok(None),
    }
}

fn keys(nx: &NxDataFile, index: u32) -> Result<Vec<String>, GameDataError> {
    nx.child_indices(index)?
        .into_iter()
        .map(|child| nx.node_name(child))
        .collect()
}

fn path_index(nx: &NxDataFile, path: &str) -> Result<Option<u32>, GameDataError> {
    let mut index = 0;
    for key in path.split('/') {
        match nx.child_by_name(index, key)? {
            Some(child) => index = child,
            None => return Ok(None),
        }
    }
    Ok(Some(index))
}

fn node(nx: &NxDataFile, path: &str) -> Result<u32, GameDataError> {
    path_index(nx, path)?.ok_or_else(|| invalid(&format!("missing quest node {path}")))
}

fn small(value: i32, what: &str) -> Result<i16, GameDataError> {
    i16::try_from(value).map_err(|_| invalid(&format!("{what} out of range")))
}

fn invalid(message: &str) -> GameDataError {
    GameDataError::InvalidData(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(start: &[&str], end: &[&str], start_act: &[&str], end_act: &[&str]) -> QuestKeys {
        let owned = |k: &[&str]| k.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        QuestKeys {
            start_check: Some(owned(start)),
            end_check: Some(owned(end)),
            start_act: owned(start_act),
            end_act: owned(end_act),
        }
    }

    #[test]
    fn talk_and_exp_quests_are_supported() {
        let quiz = keys(
            &["npc", "job", "quest"],
            &["npc"],
            &[],
            &["exp", "nextQuest"],
        );
        assert_eq!(classify(&quiz), Ok(()));
        // Act/0 sometimes repeats the offer text under numbered keys.
        let contact = keys(
            &["npc", "job", "lvmax"],
            &["npc"],
            &["0", "1", "yes", "no"],
            &["exp"],
        );
        assert_eq!(classify(&contact), Ok(()));
    }

    #[test]
    fn any_unsupported_key_marks_the_quest_unsupported() {
        let hunting = keys(&["npc"], &["npc", "mob"], &[], &["exp"]);
        assert_eq!(classify(&hunting), Err("Check/1/mob".into()));
        let item_reward = keys(&["npc"], &["npc"], &[], &["exp", "item"]);
        assert_eq!(classify(&item_reward), Err("Act/1/item".into()));
        let given_item = keys(&["npc"], &["npc"], &["item"], &[]);
        assert_eq!(classify(&given_item), Err("Act/0/item".into()));
        let money = keys(&["npc"], &["npc"], &[], &["money"]);
        assert_eq!(classify(&money), Err("Act/1/money".into()));
    }

    #[test]
    fn quests_need_a_start_and_a_completion_npc() {
        let auto_start = keys(&["job"], &["npc"], &[], &[]);
        assert!(classify(&auto_start).is_err());
        let auto_complete = keys(&["npc"], &[], &[], &[]);
        assert!(classify(&auto_complete).is_err());
        let mut no_end = keys(&["npc"], &[], &[], &[]);
        no_end.end_check = None;
        assert!(classify(&no_end).is_err());
    }

    /// The supported set must match `supported_quests.txt`, which HeavenClient
    /// reads, so the two can't disagree about what is offered.
    #[test]
    fn supported_quests_match_the_golden_list() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let actual: Vec<i16> = catalog.supported().map(|q| q.id).collect();
        let golden: Vec<i16> = include_str!("../supported_quests.txt")
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().parse().unwrap())
            .collect();
        assert_eq!(actual, golden, "update game-data/supported_quests.txt");
    }

    #[test]
    fn loads_beginner_quests_from_assets() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let heena = catalog.quest(1031).unwrap();
        assert_eq!((heena.start.npc, heena.end_npc, heena.exp), (2101, 2100, 5));
        assert_eq!(
            (heena.start.jobs.as_slice(), heena.start.max_level),
            (&[0][..], Some(10))
        );
        let nina = catalog.quest(1032).unwrap();
        assert_eq!(nina.next_quest, Some(1033));
        let sen = catalog.quest(1033).unwrap();
        assert_eq!(
            sen.start.quests,
            [QuestPrerequisite {
                quest_id: 1032,
                state: QuestState::Completed
            }]
        );
        let rain: Vec<i32> = (1009..=1015)
            .map(|id| catalog.quest(id).unwrap().exp)
            .collect();
        assert_eq!(rain, [2, 4, 6, 10, 10, 10, 20]);
        assert_eq!(catalog.quest(1036).unwrap().exp, 40);
        // Roger's Apple runs through its scripts and needs every apple eaten.
        let roger = catalog.quest(1021).unwrap();
        assert_eq!(roger.start_script.as_deref(), Some("q1021s"));
        assert_eq!(roger.end_script.as_deref(), Some("q1021e"));
        assert_eq!(roger.end_items, [(2010007, 0)]);
        // Item rewards and hunting stay unsupported until those systems exist.
        assert_eq!(catalog.unsupported_reason(1034), Some("Act/0/item"));
        assert!(catalog.unsupported_reason(1035).is_some());
    }
}
