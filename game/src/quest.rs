use std::collections::BTreeMap;
use std::time::SystemTime;

/// A quest's state as the client and the quest data number it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestState {
    NotStarted = 0,
    Started = 1,
    Completed = 2,
}

impl QuestState {
    pub fn from_number(number: i32) -> Option<Self> {
        match number {
            0 => Some(Self::NotStarted),
            1 => Some(Self::Started),
            2 => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestStatus {
    Started,
    Completed { at: SystemTime },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestRecord {
    pub quest_id: i16,
    pub status: QuestStatus,
}

/// A character's started and completed quests, keyed by quest id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuestLog {
    records: BTreeMap<i16, QuestRecord>,
}

impl QuestLog {
    pub fn from_records(records: impl IntoIterator<Item = QuestRecord>) -> Self {
        Self {
            records: records.into_iter().map(|r| (r.quest_id, r)).collect(),
        }
    }

    pub fn upsert(&mut self, record: QuestRecord) {
        self.records.insert(record.quest_id, record);
    }

    /// Forget a quest, e.g. after it was forfeited.
    pub fn remove(&mut self, quest_id: i16) {
        self.records.remove(&quest_id);
    }

    /// A started or completed quest's status; `None` if not started.
    pub fn status(&self, quest_id: i16) -> Option<&QuestStatus> {
        self.records.get(&quest_id).map(|r| &r.status)
    }

    pub fn state(&self, quest_id: i16) -> QuestState {
        match self.status(quest_id) {
            None => QuestState::NotStarted,
            Some(QuestStatus::Started) => QuestState::Started,
            Some(QuestStatus::Completed { .. }) => QuestState::Completed,
        }
    }

    pub fn started(&self) -> impl Iterator<Item = &QuestRecord> {
        self.records
            .values()
            .filter(|r| r.status == QuestStatus::Started)
    }

    pub fn completed(&self) -> impl Iterator<Item = (i16, SystemTime)> + '_ {
        self.records.values().filter_map(|r| match r.status {
            QuestStatus::Completed { at } => Some((r.quest_id, at)),
            QuestStatus::Started => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_tracks_start_completion_and_removal() {
        let mut log = QuestLog::default();
        assert_eq!(log.state(1031), QuestState::NotStarted);
        log.upsert(QuestRecord {
            quest_id: 1031,
            status: QuestStatus::Started,
        });
        assert_eq!(log.state(1031), QuestState::Started);
        let at = SystemTime::UNIX_EPOCH;
        log.upsert(QuestRecord {
            quest_id: 1031,
            status: QuestStatus::Completed { at },
        });
        assert_eq!(log.state(1031), QuestState::Completed);
        log.remove(1031);
        assert_eq!(log.state(1031), QuestState::NotStarted);
    }
}
