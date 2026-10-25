//! The OX Quiz's questions, from `Etc.nx` (`OXQuiz.img/<set>/<number>`,
//! report b171): each one's answer `a`, 1 for O and 0 for X. The client
//! reads a question's text and explanation from its own copy by set and
//! number, so the server keeps only those and the answer.

use crate::{GameDataError, NxDataFile};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OxQuestion {
    pub set: u8,
    /// Numbered from 1 and again from 100 in each set.
    pub number: u16,
    /// O is right, else X.
    pub o: bool,
    /// Its text has no Korean, which the client can't draw.
    pub english: bool,
}

#[derive(Debug, Default)]
pub struct OxQuiz {
    sets: BTreeMap<u8, Vec<OxQuestion>>,
}

impl OxQuiz {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut sets = BTreeMap::new();
        let Some(quiz) = nx.child_by_name(0, "OXQuiz.img")? else {
            return Ok(Self { sets });
        };
        for set_node in nx.child_indices(quiz)? {
            let Ok(set) = nx.node_name(set_node)?.parse::<u8>() else {
                continue;
            };
            let mut questions = Vec::new();
            for node in nx.child_indices(set_node)? {
                let (Ok(number), Some(answer)) =
                    (nx.node_name(node)?.parse::<u16>(), nx.int_child(node, "a")?)
                else {
                    continue;
                };
                let text = nx.string_child(node, "q")?.unwrap_or_default();
                questions.push(OxQuestion {
                    set,
                    number,
                    o: answer != 0,
                    english: !text.chars().any(korean),
                });
            }
            questions.sort_by_key(|question| question.number);
            sets.insert(set, questions);
        }
        Ok(Self { sets })
    }

    /// The sets, each with its questions in order.
    pub fn sets(&self) -> impl Iterator<Item = &[OxQuestion]> {
        self.sets.values().map(Vec::as_slice)
    }
}

/// Hangul, or the CJK ideographs and punctuation Korean text uses.
fn korean(c: char) -> bool {
    matches!(c, '\u{1100}'..='\u{11FF}' | '\u{3000}'..='\u{9FFF}' | '\u{AC00}'..='\u{D7AF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_sets_numbered_from_one_and_from_a_hundred() {
        let Some(path) = crate::test_assets::path("Etc.nx") else {
            return;
        };
        let quiz = OxQuiz::load(path).unwrap();
        let sets: Vec<_> = quiz.sets().collect();
        assert_eq!(sets.len(), 9);
        assert_eq!(sets.iter().map(|set| set.len()).sum::<usize>(), 553);
        let first = sets[0];
        assert_eq!(first.len(), 50);
        // "The weakest monster in MapleStory is the BLUE SNAIL." X; "RED
        // DRAKE is the only Drake that attacks long-range." O.
        assert_eq!(
            first[0],
            OxQuestion {
                set: 1,
                number: 1,
                o: false,
                english: true
            }
        );
        assert_eq!(first[29].number, 100);
        assert!(first[29].o);
        assert_eq!(first[28].number, 29);
        let english: Vec<_> = sets
            .iter()
            .map(|set| set.iter().filter(|q| q.english).count())
            .collect();
        assert_eq!(english, [41, 42, 33, 25, 42, 26, 27, 0, 0]);
    }
}
