use std::collections::BTreeMap;
use std::time::SystemTime;

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
