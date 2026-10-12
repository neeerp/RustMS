//! Asset-defined beginner dialogue quests. Other quest objective types are not
//! activated until their requirements and actions have server implementations.
use crate::{GameDataError, NxDataFile};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct QuizChoice {
    pub id: i32,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct QuizQuestTemplate {
    pub id: i16,
    pub npc_id: i32,
    pub name: String,
    pub prerequisite: Option<i16>,
    pub required_job: i16,
    pub experience: i32,
    pub offer: Vec<String>,
    pub accepted: Vec<String>,
    pub declined: Vec<String>,
    pub question: String,
    pub choices: Vec<QuizChoice>,
    pub correct_choice: i32,
    pub wrong_answers: BTreeMap<i32, String>,
    pub correct: Vec<String>,
}

#[derive(Debug)]
pub struct QuizQuestData {
    quests: BTreeMap<i16, QuizQuestTemplate>,
}

#[derive(Clone, Debug)]
pub struct ContactQuestTemplate {
    pub id: i16,
    pub name: String,
    pub start_npc: i32,
    pub finish_npc: i32,
    pub required_job: i16,
    pub max_level: i16,
    pub prerequisite: Option<i16>,
    pub experience: i32,
    pub next_quest: Option<i16>,
    pub offer: Vec<String>,
    pub accepted: Vec<String>,
    pub declined: Vec<String>,
    pub finished: Vec<String>,
    pub reminder: Vec<String>,
}

#[derive(Debug)]
pub struct ContactQuestData {
    quests: BTreeMap<i16, ContactQuestTemplate>,
}

impl ContactQuestData {
    pub fn load_from_nx(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut quests = BTreeMap::new();
        // These existing quests require only conversations. Do not activate
        // the following item/hunting quests by ignoring their requirements.
        for id in [1031, 1032, 1033] {
            let start = node(&nx, &format!("Check.img/{id}/0"))?;
            let finish = node(&nx, &format!("Check.img/{id}/1"))?;
            let action_start = node(&nx, &format!("Act.img/{id}/0"))?;
            let action_finish = node(&nx, &format!("Act.img/{id}/1"))?;
            check_keys(&nx, start, &["npc", "job", "lvmax", "quest"])?;
            check_keys(&nx, finish, &["npc"])?;
            // Some Act/0 nodes contain duplicate localized speech, not actions.
            check_keys(&nx, action_start, &["0", "1", "2", "3", "no", "yes"])?;
            check_keys(&nx, action_finish, &["exp", "nextQuest"])?;
            let job = node(&nx, &format!("Check.img/{id}/0/job"))?;
            if nx.child_indices(job)?.len() != 1 || nx.int_child(job, "0")? != Some(0) {
                return Err(invalid("unsupported contact quest job requirement"));
            }
            let prerequisite = if let Some(requirements) = nx.child_by_name(start, "quest")? {
                if nx.child_indices(requirements)?.len() != 1 {
                    return Err(invalid("unsupported contact quest prerequisites"));
                }
                let requirement = node(&nx, &format!("Check.img/{id}/0/quest/0"))?;
                if nx.int_child(requirement, "state")? != Some(2) {
                    return Err(invalid("unsupported contact prerequisite state"));
                }
                Some(quest_id(
                    nx.int_child(requirement, "id")?
                        .ok_or_else(|| invalid("missing prerequisite"))?,
                )?)
            } else {
                None
            };
            let offer = node(&nx, &format!("Say.img/{id}/0"))?;
            let finished = node(&nx, &format!("Say.img/{id}/1"))?;
            let offer_pages = pages(&nx, offer)?;
            let accepted = optional_pages(&nx, offer, "yes")?;
            let declined = optional_pages(&nx, offer, "no")?;
            if offer_pages.is_empty() || accepted.is_empty() || declined.is_empty() {
                return Err(invalid("incomplete contact quest offer"));
            }
            let mut finish_pages = pages(&nx, finished)?;
            finish_pages.extend(optional_pages(&nx, finished, "yes")?);
            let reminder = node(&nx, &format!("Say.img/{id}/1/stop/npc"))?;
            quests.insert(
                id,
                ContactQuestTemplate {
                    id,
                    name: text(&nx, node(&nx, &format!("QuestInfo.img/{id}"))?, "name")?,
                    start_npc: nx
                        .int_child(start, "npc")?
                        .ok_or_else(|| invalid("missing start NPC"))?,
                    finish_npc: nx
                        .int_child(finish, "npc")?
                        .ok_or_else(|| invalid("missing finish NPC"))?,
                    required_job: 0,
                    max_level: quest_id(
                        nx.int_child(start, "lvmax")?
                            .ok_or_else(|| invalid("missing maximum level"))?,
                    )?,
                    prerequisite,
                    experience: nx.int_child(action_finish, "exp")?.unwrap_or(0),
                    next_quest: nx
                        .int_child(action_finish, "nextQuest")?
                        .map(quest_id)
                        .transpose()?,
                    offer: offer_pages,
                    accepted,
                    declined,
                    finished: finish_pages,
                    reminder: pages(&nx, reminder)?,
                },
            );
        }
        Ok(Self { quests })
    }
    pub fn quests(&self) -> impl Iterator<Item = &ContactQuestTemplate> {
        self.quests.values()
    }
    pub fn quest(&self, id: i16) -> Option<&ContactQuestTemplate> {
        self.quests.get(&id)
    }
}

fn quest_id(id: i32) -> Result<i16, GameDataError> {
    i16::try_from(id).map_err(|_| invalid("quest integer out of range"))
}
fn check_keys(nx: &NxDataFile, parent: u32, allowed: &[&str]) -> Result<(), GameDataError> {
    for index in nx.child_indices(parent)? {
        let name = nx.node_name(index)?;
        if !allowed.contains(&name.as_str()) {
            return Err(invalid(&format!(
                "unsupported quest requirement/action {name}"
            )));
        }
    }
    Ok(())
}
fn optional_pages(nx: &NxDataFile, parent: u32, key: &str) -> Result<Vec<String>, GameDataError> {
    nx.child_by_name(parent, key)?
        .map(|n| pages(nx, n))
        .transpose()
        .map(|p| p.unwrap_or_default())
}

impl QuizQuestData {
    pub fn load_from_nx(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut quests = BTreeMap::new();
        for id in 1009..=1015 {
            let check = node(&nx, &format!("Check.img/{id}/0"))?;
            let completion = node(&nx, &format!("Act.img/{id}/1"))?;
            let info = node(&nx, &format!("QuestInfo.img/{id}"))?;
            let offer = node(&nx, &format!("Say.img/{id}/0"))?;
            let quiz = node(&nx, &format!("Say.img/{id}/1"))?;
            let answers = node(&nx, &format!("Say.img/{id}/1/stop/0"))?;
            let raw_question = text(&nx, quiz, "0")?;
            let (question, choices) =
                parse_choices(&raw_question).map_err(|e| invalid(&format!("quest {id}: {e}")))?;
            let correct_choice = nx
                .int_child(answers, "answer")?
                .ok_or_else(|| invalid("missing quiz answer"))?
                - 1;
            if !choices.iter().any(|choice| choice.id == correct_choice) {
                return Err(invalid("quiz answer is not a choice"));
            }
            let mut wrong_answers = BTreeMap::new();
            for choice in &choices {
                if choice.id != correct_choice {
                    wrong_answers.insert(choice.id, text(&nx, answers, &choice.id.to_string())?);
                }
            }
            let mut correct = vec![text(&nx, quiz, "1")?];
            if let Some(yes) = nx.child_by_name(quiz, "yes")? {
                correct.extend(pages(&nx, yes)?);
            }
            let prerequisite = if let Some(requirements) = nx.child_by_name(check, "quest")? {
                let requirement = nx
                    .child_by_name(requirements, "0")?
                    .ok_or_else(|| invalid("missing prerequisite"))?;
                if nx.int_child(requirement, "state")? != Some(2) {
                    return Err(invalid("unsupported prerequisite state"));
                }
                Some(
                    i16::try_from(
                        nx.int_child(requirement, "id")?
                            .ok_or_else(|| invalid("missing prerequisite id"))?,
                    )
                    .map_err(|_| invalid("quest id out of range"))?,
                )
            } else {
                None
            };
            let job = nx
                .child_by_name(check, "job")?
                .ok_or_else(|| invalid("missing job requirement"))?;
            let required_job =
                nx.int_child(job, "0")?
                    .ok_or_else(|| invalid("missing required job"))? as i16;
            let npc_id = nx
                .int_child(check, "npc")?
                .ok_or_else(|| invalid("missing NPC"))?;
            if npc_id != 12101 || required_job != 0 {
                return Err(invalid("unexpected Rain quiz requirements"));
            }
            let accepted = nx
                .child_by_name(offer, "yes")?
                .map(|n| pages(&nx, n))
                .transpose()?
                .unwrap_or_default();
            let declined = nx
                .child_by_name(offer, "no")?
                .map(|n| pages(&nx, n))
                .transpose()?
                .unwrap_or_default();
            quests.insert(
                id,
                QuizQuestTemplate {
                    id,
                    npc_id,
                    name: text(&nx, info, "name")?,
                    prerequisite,
                    required_job,
                    experience: nx
                        .int_child(completion, "exp")?
                        .ok_or_else(|| invalid("missing EXP reward"))?,
                    offer: pages(&nx, offer)?,
                    accepted,
                    declined,
                    question,
                    choices,
                    correct_choice,
                    wrong_answers,
                    correct,
                },
            );
        }
        Ok(Self { quests })
    }

    pub fn quests(&self) -> impl Iterator<Item = &QuizQuestTemplate> {
        self.quests.values()
    }
    pub fn quest(&self, id: i16) -> Option<&QuizQuestTemplate> {
        self.quests.get(&id)
    }
}

fn invalid(message: &str) -> GameDataError {
    GameDataError::InvalidData(message.to_string())
}
fn node(nx: &NxDataFile, path: &str) -> Result<u32, GameDataError> {
    let mut index = 0;
    for key in path.split('/') {
        index = nx
            .child_by_name(index, key)?
            .ok_or_else(|| invalid(&format!("missing quest node {path}")))?;
    }
    Ok(index)
}
fn text(nx: &NxDataFile, parent: u32, key: &str) -> Result<String, GameDataError> {
    nx.string_child(parent, key)?
        .map(|s| s.replace("\\n", "\n"))
        .ok_or_else(|| invalid(&format!("missing quest text {key}")))
}
fn pages(nx: &NxDataFile, parent: u32) -> Result<Vec<String>, GameDataError> {
    let mut numbered = Vec::new();
    for index in nx.child_indices(parent)? {
        let key = nx.node_name(index)?;
        if let Ok(order) = key.parse::<u32>() {
            numbered.push((order, text(nx, parent, &key)?));
        }
    }
    numbered.sort_by_key(|(order, _)| *order);
    Ok(numbered.into_iter().map(|(_, text)| text).collect())
}
fn parse_choices(raw: &str) -> Result<(String, Vec<QuizChoice>), GameDataError> {
    let first = raw
        .find("#L")
        .ok_or_else(|| invalid("quiz has no choices"))?;
    let question = raw[..first].trim().to_string();
    let mut choices = Vec::new();
    let mut offset = first;
    while let Some(start) = raw[offset..].find("#L") {
        let id_start = offset + start + 2;
        let id_end = id_start
            + raw[id_start..]
                .find('#')
                .ok_or_else(|| invalid("malformed choice id"))?;
        let label_start = id_end + 1;
        let label_end = label_start
            + raw[label_start..]
                .find("#l")
                .ok_or_else(|| invalid("malformed choice label"))?;
        choices.push(QuizChoice {
            id: raw[id_start..id_end]
                .parse()
                .map_err(|_| invalid("invalid choice id"))?,
            label: raw[label_start..label_end].trim().to_string(),
        });
        offset = label_end + 2;
    }
    Ok((question, choices))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_existing_contact_quests_without_enabling_item_objectives() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let data = ContactQuestData::load_from_nx(path).unwrap();
        let heena = data.quest(1031).unwrap();
        assert_eq!(
            (
                heena.start_npc,
                heena.finish_npc,
                heena.experience,
                heena.max_level
            ),
            (2101, 2100, 5, 10)
        );
        assert_eq!(heena.offer.len(), 4);
        assert_eq!(heena.finished.len(), 3);
        let nina = data.quest(1032).unwrap();
        assert_eq!(
            (nina.start_npc, nina.finish_npc, nina.next_quest),
            (2102, 2001, Some(1033))
        );
        assert!(nina.finished.is_empty());
        assert_eq!(data.quest(1033).unwrap().prerequisite, Some(1032));
        assert!(data.quest(1034).is_none());
    }
    #[test]
    fn loads_all_seven_quizzes_from_assets() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let data = QuizQuestData::load_from_nx(path).unwrap();
        let answers: Vec<_> = data.quests().map(|q| q.correct_choice).collect();
        assert_eq!(answers, [0, 0, 0, 3, 1, 2, 0]);
        assert_eq!(data.quests().map(|q| q.experience).sum::<i32>(), 62);
        assert_eq!(data.quest(1015).unwrap().prerequisite, Some(1014));
        assert!(!data.quest(1009).unwrap().accepted.is_empty());
    }
}
