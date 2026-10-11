use crate::dialogue::DialoguePage;
pub use game_data::quests::{QuizQuestData, QuizQuestTemplate};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

static QUESTS: OnceLock<Result<QuizQuestData, String>> = OnceLock::new();
pub fn data() -> Result<&'static QuizQuestData, &'static str> {
    QUESTS
        .get_or_init(|| {
            let path = std::env::var("RUSTMS_QUEST_NX_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("assets/game-data/Quest.nx"));
            QuizQuestData::load_from_nx(path).map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| e.as_str())
}

pub fn eligible<'a>(
    data: &'a QuizQuestData,
    npc_id: i32,
    job: i16,
    states: &BTreeMap<i16, i16>,
) -> Option<&'a QuizQuestTemplate> {
    data.quests().find(|q| {
        q.npc_id == npc_id
            && q.required_job == job
            && states.get(&q.id).copied().unwrap_or(0) != 2
            && q.prerequisite
                .map(|id| states.get(&id) == Some(&2))
                .unwrap_or(true)
    })
}

#[derive(Clone, Debug)]
pub enum QuestPrompt {
    Text(DialoguePage),
    YesNo { npc_id: i32, text: String },
    Selection { npc_id: i32, text: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestEffect {
    Start,
    Complete,
}

#[derive(Debug)]
enum Phase {
    Offer(usize),
    Intro(usize),
    Question,
    Wrong(String),
    Correct(usize),
    Declined(usize),
}

#[derive(Debug)]
pub struct QuizConversation {
    pub quest: QuizQuestTemplate,
    phase: Phase,
}

impl QuizConversation {
    pub fn new(quest: &QuizQuestTemplate, started: bool) -> Self {
        Self {
            quest: quest.clone(),
            phase: if started {
                Phase::Question
            } else {
                Phase::Offer(0)
            },
        }
    }

    pub fn prompt(&self) -> QuestPrompt {
        let q = &self.quest;
        let say = |text: String, previous: bool, next: bool| {
            QuestPrompt::Text(DialoguePage {
                npc_id: q.npc_id,
                text,
                has_previous: previous,
                has_next: next,
            })
        };
        match &self.phase {
            Phase::Offer(i) if *i + 1 == q.offer.len() && !q.declined.is_empty() => {
                QuestPrompt::YesNo {
                    npc_id: q.npc_id,
                    text: q.offer[*i].clone(),
                }
            }
            Phase::Offer(i) => say(q.offer[*i].clone(), *i > 0, true),
            Phase::Intro(i) => say(q.accepted[*i].clone(), *i > 0, true),
            Phase::Question => {
                let mut text = q.question.clone();
                text.push_str("\n\n");
                for choice in &q.choices {
                    text.push_str(&format!("#L{}#{}#l\n", choice.id, choice.label));
                }
                QuestPrompt::Selection {
                    npc_id: q.npc_id,
                    text,
                }
            }
            Phase::Wrong(text) => say(text.clone(), false, true),
            Phase::Correct(i) => say(q.correct[*i].clone(), *i > 0, *i + 1 < q.correct.len()),
            Phase::Declined(i) => say(q.declined[*i].clone(), *i > 0, *i + 1 < q.declined.len()),
        }
    }

    /// Effects are intent, not success: the runtime must commit them atomically
    /// before sending the next prompt or any quest/reward notification.
    pub fn respond(
        &mut self,
        message_type: u8,
        response: i8,
        selection: Option<i32>,
    ) -> Option<(QuestPrompt, Option<QuestEffect>)> {
        let expected = match self.prompt() {
            QuestPrompt::Text(_) => 0,
            QuestPrompt::YesNo { .. } => 1,
            QuestPrompt::Selection { .. } => 4,
        };
        if message_type != expected || response == -1 {
            return None;
        }
        let mut effect = None;
        match &mut self.phase {
            Phase::Offer(i) => {
                if expected == 1 && response == 0 {
                    self.phase = Phase::Declined(0);
                } else if response == 0 && *i > 0 {
                    *i -= 1;
                } else if response == 1 {
                    if *i + 1 < self.quest.offer.len() {
                        *i += 1;
                    } else {
                        effect = Some(QuestEffect::Start);
                        self.phase = if self.quest.accepted.is_empty() {
                            Phase::Question
                        } else {
                            Phase::Intro(0)
                        };
                    }
                } else {
                    return None;
                }
            }
            Phase::Intro(i) => {
                if response == 0 && *i > 0 {
                    *i -= 1;
                } else if response == 1 {
                    if *i + 1 < self.quest.accepted.len() {
                        *i += 1;
                    } else {
                        self.phase = Phase::Question;
                    }
                } else {
                    return None;
                }
            }
            Phase::Question => {
                if response != 1 {
                    return None;
                }
                let selected = selection?;
                if !self
                    .quest
                    .choices
                    .iter()
                    .any(|choice| choice.id == selected)
                {
                    return None;
                }
                if selected == self.quest.correct_choice {
                    effect = Some(QuestEffect::Complete);
                    self.phase = Phase::Correct(0);
                } else {
                    self.phase = Phase::Wrong(self.quest.wrong_answers.get(&selected)?.clone());
                }
            }
            Phase::Wrong(_) if response == 1 => self.phase = Phase::Question,
            Phase::Correct(i) => {
                if response == 0 && *i > 0 {
                    *i -= 1;
                } else if response == 1 && *i + 1 < self.quest.correct.len() {
                    *i += 1;
                } else {
                    return None;
                }
            }
            Phase::Declined(i) => {
                if response == 0 && *i > 0 {
                    *i -= 1;
                } else if response == 1 && *i + 1 < self.quest.declined.len() {
                    *i += 1;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
        Some((self.prompt(), effect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> QuizQuestTemplate {
        QuizQuestTemplate {
            id: 1009,
            npc_id: 12101,
            name: "Quiz".into(),
            prerequisite: None,
            required_job: 0,
            experience: 2,
            offer: vec!["offer".into()],
            accepted: vec!["intro".into()],
            declined: vec!["declined".into()],
            question: "question".into(),
            choices: vec![
                game_data::quests::QuizChoice {
                    id: 0,
                    label: "right".into(),
                },
                game_data::quests::QuizChoice {
                    id: 1,
                    label: "wrong".into(),
                },
            ],
            correct_choice: 0,
            wrong_answers: BTreeMap::from([(1, "retry".into())]),
            correct: vec!["correct".into()],
        }
    }
    #[test]
    fn decline_cancel_and_invalid_choices_have_no_reward() {
        let mut c = QuizConversation::new(&fixture(), false);
        assert_eq!(c.respond(1, 0, None).unwrap().1, None);
        assert!(c.respond(0, 1, None).is_none());
        assert!(QuizConversation::new(&fixture(), false)
            .respond(1, -1, None)
            .is_none());
        assert!(QuizConversation::new(&fixture(), true)
            .respond(4, 1, Some(99))
            .is_none());
        assert!(QuizConversation::new(&fixture(), true)
            .respond(0, 1, Some(0))
            .is_none());
    }
    #[test]
    fn acceptance_wrong_answer_retry_and_reward_are_separate_transitions() {
        let mut c = QuizConversation::new(&fixture(), false);
        assert_eq!(c.respond(1, 1, None).unwrap().1, Some(QuestEffect::Start));
        c.respond(0, 1, None).unwrap();
        assert_eq!(c.respond(4, 1, Some(1)).unwrap().1, None);
        c.respond(0, 1, None).unwrap();
        assert_eq!(
            c.respond(4, 1, Some(0)).unwrap().1,
            Some(QuestEffect::Complete)
        );
        assert!(c.respond(4, 1, Some(0)).is_none());
    }
}
