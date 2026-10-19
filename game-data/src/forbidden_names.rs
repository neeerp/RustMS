//! The words a character name may not contain, from `Etc.nx`: every
//! `ForbiddenName.img` entry, and each `Curse.img` entry up to its first
//! comma. The client checks names against the same list before sending them.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

#[derive(Debug, Default)]
pub struct ForbiddenNames {
    /// Lowercase.
    words: Vec<String>,
}

impl ForbiddenNames {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut words = Vec::new();
        for image in ["ForbiddenName.img", "Curse.img"] {
            let Some(list) = nx.child_by_name(0, image)? else {
                continue;
            };
            for entry in nx.child_indices(list)? {
                let Some(word) = nx.string_child(list, &nx.node_name(entry)?)? else {
                    continue;
                };
                let word = match image {
                    "Curse.img" => word.split(',').next().unwrap_or_default().trim(),
                    _ => word.as_str(),
                };
                if !word.is_empty() {
                    words.push(word.to_ascii_lowercase());
                }
            }
        }
        Ok(Self { words })
    }

    /// Whether `name` contains a listed word anywhere, ignoring case.
    pub fn forbids(&self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        self.words.iter().any(|word| name.contains(word.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_containing_a_listed_word_are_forbidden() {
        let Some(path) = crate::test_assets::path("Etc.nx") else {
            return;
        };
        let names = ForbiddenNames::load(path).unwrap();
        assert!(names.forbids("Admin"));
        assert!(names.forbids("xxMAPLExx"), "anywhere in the name");
        assert!(names.forbids("bltchy"), "Curse.img words too");
        assert!(!names.forbids("Rowan"));
    }
}
