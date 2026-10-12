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
        (self.skill_id / 10_000) % 10 == 2
    }
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

    pub fn set(&mut self, record: SkillRecord) {
        self.skills.insert(record.skill_id, record);
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
}
