//! What a new character may look like and start with, from `Etc.nx`
//! (`MakeCharInfo.img`). The client offers exactly these lists on its
//! creation screen.

use crate::{GameDataError, NxDataFile};
use std::collections::HashMap;
use std::path::Path;

/// The lists, in the order of the creation packet: face, hair, hair colour,
/// skin, top, bottom, shoes and weapon.
type Choices = [Vec<i32>; 8];

#[derive(Debug, Default)]
pub struct MakeCharInfo {
    /// By the creation packet's class flag and gender (0 male, 1 female).
    choices: HashMap<(i32, u8), Choices>,
}

impl MakeCharInfo {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let root = nx
            .child_by_name(0, "MakeCharInfo.img")?
            .ok_or_else(|| GameDataError::InvalidData("missing MakeCharInfo.img".to_string()))?;
        let mut choices = HashMap::new();
        // Cygnus, Explorer and Aran; the Explorer lists are under Info.
        for (class, parent, prefix) in [
            (0, Some(root), "PremiumChar"),
            (1, nx.child_by_name(root, "Info")?, "Char"),
            (2, Some(root), "OrientChar"),
        ] {
            let Some(parent) = parent else {
                continue;
            };
            for (gender, suffix) in [(0, "Male"), (1, "Female")] {
                let Some(set) = nx.child_by_name(parent, &format!("{prefix}{suffix}"))? else {
                    continue;
                };
                let mut lists = Choices::default();
                for (index, list) in lists.iter_mut().enumerate() {
                    let Some(node) = nx.child_by_name(set, &index.to_string())? else {
                        continue;
                    };
                    for entry in nx.child_indices(node)? {
                        if let Some(value) = nx.int_child(node, &nx.node_name(entry)?)? {
                            list.push(value);
                        }
                    }
                }
                choices.insert((class, gender), lists);
            }
        }
        Ok(Self { choices })
    }

    /// Whether a character of `class` and `gender` may be made with `chosen`
    /// (in the order of the creation packet).
    pub fn allows(&self, class: i32, gender: u8, chosen: [i32; 8]) -> bool {
        self.choices.get(&(class, gender)).is_some_and(|lists| {
            lists
                .iter()
                .zip(chosen)
                .all(|(list, value)| list.contains(&value))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_lists_offered_for_each_class_and_gender() {
        let Some(path) = crate::test_assets::path("Etc.nx") else {
            return;
        };
        let info = MakeCharInfo::load(path).unwrap();
        let explorer = [20000, 30030, 7, 3, 1040002, 1060002, 1072001, 1302000];
        assert!(info.allows(1, 0, explorer));
        // The female lists, and other classes' gear, are not offered.
        assert!(!info.allows(1, 1, explorer));
        assert!(!info.allows(
            1,
            0,
            [20000, 30030, 7, 3, 1040002, 1060002, 1072001, 1442079]
        ));
        assert!(!info.allows(
            1,
            0,
            [20000, 30030, 1, 3, 1040002, 1060002, 1072001, 1302000]
        ));
        assert!(!info.allows(3, 0, explorer));
        assert!(info.allows(
            2,
            1,
            [21700, 31000, 0, 0, 1042167, 1062115, 1072383, 1442079]
        ));
        assert!(info.allows(
            0,
            1,
            [21000, 31040, 2, 1, 1041011, 1061008, 1072038, 1312004]
        ));
    }
}
