//! Skill templates from `Skill.nx` (`<job>.img/skill/<id>`), looked up on
//! demand.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillInfo {
    pub skill_id: i32,
    /// The number of `level` entries.
    pub max_level: i32,
    /// Hidden from the skill window until learned some other way.
    pub invisible: bool,
    /// Skills that must reach a level first (`req`): `(skill id, level)`.
    pub requirements: Vec<(i32, i32)>,
}

pub struct SkillData {
    nx: NxDataFile,
}

impl SkillData {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        Ok(Self {
            nx: NxDataFile::open(path.as_ref())?,
        })
    }

    pub fn info(&self, skill_id: i32) -> Result<Option<SkillInfo>, GameDataError> {
        let nx = &self.nx;
        let job = skill_id / 10_000;
        let Some(image) = nx.child_by_name(0, &format!("{job:03}.img"))? else {
            return Ok(None);
        };
        let Some(skills) = nx.child_by_name(image, "skill")? else {
            return Ok(None);
        };
        let Some(skill) = nx.child_by_name(skills, &format!("{skill_id:07}"))? else {
            return Ok(None);
        };
        let max_level = match nx.child_by_name(skill, "level")? {
            Some(levels) => i32::from(nx.node_at(levels)?.child_count),
            None => 0,
        };
        let mut requirements = Vec::new();
        if let Some(req) = nx.child_by_name(skill, "req")? {
            for child in nx.child_indices(req)? {
                let id = nx.node_name(child)?.parse().map_err(|_| {
                    GameDataError::InvalidData(format!("skill {skill_id} has a bad req id"))
                })?;
                let level = nx.int_child(req, &nx.node_name(child)?)?.unwrap_or(0);
                requirements.push((id, level));
            }
        }
        Ok(Some(SkillInfo {
            skill_id,
            max_level,
            invisible: nx.int_child(skill, "invisible")?.unwrap_or(0) != 0,
            requirements,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_skill_levels_requirements_and_flags() {
        let Some(path) = crate::test_assets::path("Skill.nx") else {
            return;
        };
        let data = SkillData::load(path).unwrap();

        let three_snails = data.info(1000).unwrap().unwrap();
        assert_eq!(three_snails.max_level, 3);
        assert!(!three_snails.invisible);

        let power_strike = data.info(1001004).unwrap().unwrap();
        assert_eq!(power_strike.max_level, 20);
        let slash_blast = data.info(1001005).unwrap().unwrap();
        assert_eq!(slash_blast.requirements, vec![(1001004, 1)]);

        let maple_warrior = data.info(2121000).unwrap().unwrap();
        assert!(maple_warrior.invisible);
        assert_eq!(maple_warrior.max_level, 30);

        assert!(data.info(1009999).unwrap().is_none());
        assert!(data.info(9_990_000).unwrap().is_none());
    }
}
