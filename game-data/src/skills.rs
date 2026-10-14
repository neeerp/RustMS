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
    /// What each level does, from level 1.
    pub levels: Vec<SkillLevel>,
}

impl SkillInfo {
    pub fn level(&self, level: i32) -> Option<&SkillLevel> {
        usize::try_from(level - 1)
            .ok()
            .and_then(|index| self.levels.get(index))
    }
}

/// One `level/<n>` entry. Missing values are 0, except the counts, which
/// default to 1 as the client reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillLevel {
    pub mp_cost: i32,
    /// Percent of weapon damage (`damage`).
    pub damage: i32,
    /// Damage that ignores stats (`fixdamage`), as Three Snails does.
    pub fixed_damage: i32,
    pub mob_count: i32,
    pub attack_count: i32,
    /// Buff duration in seconds (`time`).
    pub duration: i32,
    /// Seconds before the skill can be used again (`cooltime`).
    pub cooldown: i32,
    pub speed: i32,
    /// The skill's own value, e.g. Recovery's HP per heal.
    pub x: i32,
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
        let mut levels = Vec::new();
        if let Some(level_nodes) = nx.child_by_name(skill, "level")? {
            for n in 1..=nx.node_at(level_nodes)?.child_count {
                let Some(level) = nx.child_by_name(level_nodes, &n.to_string())? else {
                    return Err(GameDataError::InvalidData(format!(
                        "skill {skill_id} has no level {n}"
                    )));
                };
                let int = |key| -> Result<i32, GameDataError> {
                    Ok(nx.int_child(level, key)?.unwrap_or(0))
                };
                levels.push(SkillLevel {
                    mp_cost: int("mpCon")?,
                    damage: int("damage")?,
                    fixed_damage: int("fixdamage")?,
                    mob_count: nx.int_child(level, "mobCount")?.unwrap_or(1),
                    attack_count: nx.int_child(level, "attackCount")?.unwrap_or(1),
                    duration: int("time")?,
                    cooldown: int("cooltime")?,
                    speed: int("speed")?,
                    x: int("x")?,
                });
            }
        }
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
            max_level: levels.len() as i32,
            invisible: nx.int_child(skill, "invisible")?.unwrap_or(0) != 0,
            requirements,
            levels,
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
        let snails_3 = three_snails.level(3).unwrap();
        assert_eq!(
            (snails_3.fixed_damage, snails_3.mp_cost, snails_3.mob_count),
            (40, 7, 1)
        );
        assert!(three_snails.level(4).is_none() && three_snails.level(0).is_none());

        let recovery = data.info(1001).unwrap().unwrap().levels[0].clone();
        assert_eq!(
            (
                recovery.x,
                recovery.duration,
                recovery.cooldown,
                recovery.mp_cost
            ),
            (4, 30, 120, 5)
        );
        let nimble_feet = data.info(1002).unwrap().unwrap().levels[2].clone();
        assert_eq!((nimble_feet.speed, nimble_feet.duration), (20, 12));

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
