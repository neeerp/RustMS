//! The asset-defined Rain quiz chain. Other quest objective types are not
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
    fn loads_all_seven_quizzes_from_assets() {
        let data = QuizQuestData::load_from_nx(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Quest.nx"),
        )
        .unwrap();
        let answers: Vec<_> = data.quests().map(|q| q.correct_choice).collect();
        assert_eq!(answers, [0, 0, 0, 3, 1, 2, 0]);
        assert_eq!(data.quests().map(|q| q.experience).sum::<i32>(), 62);
        assert_eq!(data.quest(1015).unwrap().prerequisite, Some(1014));
        assert!(!data.quest(1009).unwrap().accepted.is_empty());
    }
}
