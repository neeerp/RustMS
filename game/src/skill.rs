use std::collections::BTreeMap;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillRecord {
    pub skill_id: i32,
    pub level: i32,
    pub master_level: i32,
    pub expiration: Option<SystemTime>,
}

impl SkillRecord {
    /// Fourth-job skills also carry a master level on the wire.
    pub fn is_fourth_job(&self) -> bool {
        is_fourth_job(self.skill_id)
    }
}

/// Fourth-job skills can only be raised up to the master level the
/// character has been granted.
pub fn is_fourth_job(skill_id: i32) -> bool {
    (skill_id / 10_000) % 10 == 2
}

/// Blessing of the Fairy for a character of `job`: 12, or the Noblesse's
/// 10000012 or the Legend's 20000012 (v83 SetFrom, 0x7801bb). None for
/// Evan, whose 20010012 has no data.
pub fn fairy_blessing(job: i16) -> Option<i32> {
    match job / 1000 {
        _ if job / 100 == 22 || job == 2001 => None,
        tree @ 0..=2 => Some(i32::from(tree) * 10_000_000 + 12),
        _ => None,
    }
}

/// Blessing of the Fairy's level for a linked character of `level`: one for
/// each 10 levels, as Cosmic grants it (the client takes the record as
/// sent).
pub fn fairy_blessing_level(level: i16) -> i32 {
    i32::from(level / 10).clamp(0, 20)
}

/// Three Snails, Recovery, and Nimble Feet (and their Cygnus and Aran
/// copies) are raised from the beginner pool rather than SP.
pub fn uses_beginner_sp(skill_id: i32) -> bool {
    (1000..=1002).contains(&(skill_id % 10_000_000))
}

/// Whether a character of `job` can learn `skill_id`: the skill belongs to
/// its job or to one it advanced through (beginner, first, second, third).
pub fn in_job_tree(skill_id: i32, job: i16) -> bool {
    let skill_job = skill_id / 10_000;
    let job = i32::from(job);
    skill_job == job / 1000 * 1000
        || skill_job == job / 100 * 100
        || (skill_job / 10 == job / 10 && skill_job <= job)
}

/// A character's learned skills keyed by skill id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillBook {
    skills: BTreeMap<i32, SkillRecord>,
}

impl SkillBook {
    pub fn from_records(records: impl IntoIterator<Item = SkillRecord>) -> Self {
        Self {
            skills: records.into_iter().map(|s| (s.skill_id, s)).collect(),
        }
    }

    pub fn get(&self, skill_id: i32) -> Option<&SkillRecord> {
        self.skills.get(&skill_id)
    }

    /// The skill's level, 0 if it was never learned.
    pub fn level(&self, skill_id: i32) -> i32 {
        self.get(skill_id).map_or(0, |s| s.level)
    }

    pub fn set(&mut self, record: SkillRecord) {
        self.skills.insert(record.skill_id, record);
    }

    pub fn remove(&mut self, skill_id: i32) {
        self.skills.remove(&skill_id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &SkillRecord> {
        self.skills.values()
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_fairy_blesses_by_the_linked_characters_tens() {
        let levels: Vec<_> = [1, 9, 10, 199, 200].map(fairy_blessing_level).into();
        assert_eq!(levels, [0, 0, 1, 19, 20]);
        assert_eq!(fairy_blessing(0), Some(12));
        assert_eq!(fairy_blessing(522), Some(12));
        assert_eq!(fairy_blessing(1311), Some(10_000_012));
        assert_eq!(fairy_blessing(2112), Some(20_000_012));
        assert_eq!(fairy_blessing(2001), None);
        assert_eq!(fairy_blessing(2218), None);
        assert_eq!(fairy_blessing(900), Some(12));
    }

    use super::*;

    #[test]
    fn fourth_job_skills_are_detected_from_their_job_digit() {
        let record = |skill_id| SkillRecord {
            skill_id,
            level: 1,
            master_level: 0,
            expiration: None,
        };
        assert!(record(1121008).is_fourth_job());
        assert!(record(21120002).is_fourth_job());
        assert!(!record(1001004).is_fourth_job());
        assert!(!record(1000).is_fourth_job());
    }

    #[test]
    fn jobs_learn_the_skills_of_every_job_they_advanced_through() {
        // Hero: beginner, warrior, fighter, crusader, hero.
        for skill_id in [1000, 1001004, 1101006, 1111002, 1121008] {
            assert!(in_job_tree(skill_id, 112), "{skill_id}");
        }
        // Not a page's, a magician's, or a fourth job's yet.
        for skill_id in [1201004, 2001002, 1121008, 10001000] {
            assert!(!in_job_tree(skill_id, 111), "{skill_id}");
        }
        assert!(in_job_tree(1000, 0));
        assert!(!in_job_tree(1001004, 0));
        assert!(in_job_tree(20001000, 2100) && in_job_tree(21000002, 2112));
        assert!(!in_job_tree(1000, 2000));
    }

    #[test]
    fn only_the_three_beginner_skills_use_the_beginner_pool() {
        for skill_id in [1000, 1001, 1002, 10001001, 20001002] {
            assert!(uses_beginner_sp(skill_id), "{skill_id}");
        }
        for skill_id in [1003, 12, 1001004, 10001003] {
            assert!(!uses_beginner_sp(skill_id), "{skill_id}");
        }
    }
}
