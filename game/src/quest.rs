use std::collections::{BTreeMap, BTreeSet};
use std::time::SystemTime;

/// Quest ids from here up are not in Quest.nx (its last is 29933). Scripts
/// keep markers under them as started quests (`QuestLog::mark`), saved with
/// the player rather than through quest start and completion, as upstream
/// servers keep job advancement steps.
pub const SCRIPT_MARKERS: i16 = 32000;

/// Quests whose record is an info string rather than kill counts: their
/// start sets it (`Act/0/info`, else it starts empty), NPCs change one
/// character at a time or set it whole, and completion needs it to read a
/// given value (`Check/1/infoex`). Only quests whose strings the server's
/// scripts can set are listed, as others could never be completed.
pub const INFO_QUESTS: &[i16] = &[
    2166, 2236, 3114, 3311, 3345, 3360, 3367, 3421, 3900, 3926, 3927, 3929,
];

/// Records that are no quest (Quest.nx has no such id) but that another
/// quest's completion (`Check/1 infoNumber`) or start (`Check/0`) reads:
/// an info string a script sets whole, kept as a started record. Only
/// records the server's scripts set are listed: Kyrin's tests' (6331 and 6371, for 6330 and
/// 6370), the past Papulatus's (6364, for "A New Revelation", 6363), Lord
/// Jonathan's (6401, for "Jonathan's Court", 6400), Save Delli!'s way
/// out's (6411, for "Mission Free the Monsters", 6410) and Nett's
/// Pyramid's kill count (7760, for "Protector of Pharaoh", 29932) and the
/// Abandoned Subway's (7662, and 7762, which its quest window shows, for
/// "Honorary Subway Worker", 29931), and the lost memory a job instructor gives back (7081, for "Memory Keeper",
/// 3507), and the black shadows the Cygnus Knights hunt: the one spotted in
/// the Pig Farm (20731, for "A Black Shadow in the Pig Farm", 20706) and
/// the one in the Magic Library (20732, for "Maybe it's Grendel!", 20718),
/// and an Aran's search of the Puppeteer's door (21761, for "The
/// Puppeteer's Cave", 21728) and the Puppeteer's fall in Lith Harbor's
/// Info Shop (21762, for "Gathering Some Strange Information", 21733),
/// and Pink Bean's fall (7402, for "Beyond the Ruins", 3522, and its
/// Cygnus Knights' twin, 3538),
/// and the growth of the Timer's Egg a player raises (7067, for "A Cute
/// Baby Bird", 3250) and the writing coming back on Mr. Do's Hanging
/// Scroll (21763, for "The Black Scroll", 21742), which feeding them in
/// their raise windows adds to, and an Aran's seeing the Ellin Forest seal
/// stone taken from its Warehouse (21764, for "The Lost Seal Stone of
/// Ellin Forest", 21753) and finding Athena Pierce in their own time
/// (21765, for "Info on Another Seal Stone", 21754), and the Puppeteer's
/// warning heard in its cave (21760, for "The Puppeteer's Warning", 21720);
/// also an Aran's runs with the wolves: through the Field of Wolves (21619,
/// for "Scadur's Advice", 21610) and out of the wolves' test room (21620,
/// for "The Wolves' Test", 21613); and a Romeo and Juliet run won (7072,
/// for "Yulete's Request", 3365, and "Romeo's Present", 3369).
/// The Mushroom Kingdom's opening scene's (2311) is read by The
/// Test's start (2312) instead. The party quests' info quests (1200-1206,
/// 1301, 1302), which the client never offers, are started on the first entry,
/// with no string, and keep their records beside the quest log
/// (`crate::quest_ex`).
pub const RECORDS: &[i16] = &[
    1200, 1201, 1202, 1203, 1204, 1205, 1206, 1301, 1302, 2311, 6331, 6364, 6371, 6401, 6411, 7067,
    7072, 7081, 7402, 7662, 7760, 7762, 20731, 20732, 21619, 21620, 21760, 21761, 21762, 21763,
    21764, 21765,
];

/// The timed titles (`QuestInfo` `timeLimit2`): Celebrity, Diligent
/// Explorer, Veteran Hunter and Donation King. Their started record is the
/// date they end, "yyyyMMddHHmmss" (`end_date`), which the client's Quest Log
/// counts down to (#Qdaylimit#..., 0x4e6a03).
pub const DATED_QUESTS: &[i16] = &[29002, 29003, 29400, 29503];

/// A timed title's end as its started record: "yyyyMMddHHmmss" in UTC, the
/// clock the server's FILETIMEs give the client.
pub fn end_date(at: SystemTime) -> String {
    let seconds = at
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    let (days, time) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}{:02}{:02}{:02}",
        time / 3600,
        time / 60 % 60,
        time % 60
    )
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestStatus {
    /// Kill counts of the quest's required mobs, in `Check/1/mob` order,
    /// or, for `INFO_QUESTS`, the info string (`kills` is then empty).
    Started {
        kills: Vec<u16>,
        info: Option<String>,
    },
    Completed {
        at: SystemTime,
    },
}

impl QuestStatus {
    /// A started quest's record as the client and the database hold it: the
    /// info string, or the kill counts; `None` once completed.
    pub fn record_string(&self) -> Option<String> {
        match self {
            QuestStatus::Started {
                info: Some(info), ..
            } => Some(info.clone()),
            QuestStatus::Started { kills, .. } => Some(encode_kills(kills)),
            QuestStatus::Completed { .. } => None,
        }
    }
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
    /// The maps visited toward each explorer medal, keyed by the info
    /// record that shows their count (27010 for Victoria's, ...). They stay
    /// through a forfeit, so a restarted medal carries on.
    explored: BTreeMap<i16, BTreeSet<i32>>,
}

impl QuestLog {
    pub fn from_records(records: impl IntoIterator<Item = QuestRecord>) -> Self {
        Self {
            records: records.into_iter().map(|r| (r.quest_id, r)).collect(),
            explored: BTreeMap::new(),
        }
    }

    /// The log with these `(info record, map)` visits.
    pub fn with_explored(mut self, visits: impl IntoIterator<Item = (i16, i32)>) -> Self {
        for (record, map_id) in visits {
            self.explore(record, map_id);
        }
        self
    }

    /// Count a visit to `map_id` toward the medal whose info record is
    /// `record`; `false` if it was visited before.
    pub fn explore(&mut self, record: i16, map_id: i32) -> bool {
        self.explored.entry(record).or_default().insert(map_id)
    }

    /// Whether `map_id` was visited toward the medal with info record `record`.
    pub fn has_explored(&self, record: i16, map_id: i32) -> bool {
        self.explored
            .get(&record)
            .is_some_and(|maps| maps.contains(&map_id))
    }

    /// How many maps were visited toward the medal with info record `record`.
    pub fn explored(&self, record: i16) -> usize {
        self.explored.get(&record).map_or(0, BTreeSet::len)
    }

    /// Every `(info record, map)` visit.
    pub fn visits(&self) -> impl Iterator<Item = (i16, i32)> + '_ {
        (self.explored.iter())
            .flat_map(|(&record, maps)| maps.iter().map(move |&map| (record, map)))
    }

    pub fn upsert(&mut self, record: QuestRecord) {
        self.records.insert(record.quest_id, record);
    }

    /// Forget a quest, e.g. after it was forfeited.
    pub fn remove(&mut self, quest_id: i16) {
        self.records.remove(&quest_id);
    }

    /// Set a script marker (an id from `SCRIPT_MARKERS` up).
    pub fn mark(&mut self, marker: i16) {
        debug_assert!(marker >= SCRIPT_MARKERS);
        self.upsert(QuestRecord {
            quest_id: marker,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
    }

    pub fn markers(&self) -> impl Iterator<Item = i16> + '_ {
        self.records.range(SCRIPT_MARKERS..).map(|(&id, _)| id)
    }

    /// Kill counts for a started quest's required mobs.
    pub fn kills(&self, quest_id: i16) -> &[u16] {
        match self.status(quest_id) {
            Some(QuestStatus::Started { kills, .. }) => kills,
            _ => &[],
        }
    }

    /// The kill counts of a started quest, to count kills into.
    pub fn kills_mut(&mut self, quest_id: i16) -> Option<&mut Vec<u16>> {
        match &mut self.records.get_mut(&quest_id)?.status {
            QuestStatus::Started { kills, .. } => Some(kills),
            QuestStatus::Completed { .. } => None,
        }
    }

    /// A started info quest's string (`INFO_QUESTS`).
    pub fn info(&self, quest_id: i16) -> Option<&str> {
        match self.status(quest_id) {
            Some(QuestStatus::Started { info, .. }) => info.as_deref(),
            _ => None,
        }
    }

    /// Set a record (`RECORDS`) to `value`; `false` if it already reads so.
    pub fn set_record(&mut self, record: i16, value: &str) -> bool {
        debug_assert!(RECORDS.contains(&record));
        if self.info(record) == Some(value) {
            return false;
        }
        self.upsert(QuestRecord {
            quest_id: record,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: Some(value.to_owned()),
            },
        });
        true
    }

    /// Set a started info quest's whole string to `value`; `false`,
    /// changing nothing, if it isn't started or already reads so.
    pub fn set_info(&mut self, quest_id: i16, value: &str) -> bool {
        let Some(QuestRecord {
            status: QuestStatus::Started {
                info: Some(info), ..
            },
            ..
        }) = self.records.get_mut(&quest_id)
        else {
            return false;
        };
        if info == value {
            return false;
        }
        value.clone_into(info);
        true
    }

    /// Set character `slot` of a started quest's info string to `to` if it
    /// is `from`; `false`, changing nothing, otherwise.
    pub fn set_info_char(&mut self, quest_id: i16, slot: usize, from: u8, to: u8) -> bool {
        let Some(QuestRecord {
            status: QuestStatus::Started {
                info: Some(info), ..
            },
            ..
        }) = self.records.get_mut(&quest_id)
        else {
            return false;
        };
        if info.as_bytes().get(slot) != Some(&from) || !from.is_ascii() || !to.is_ascii() {
            return false;
        }
        info.replace_range(slot..=slot, &char::from(to).to_string());
        true
    }

    /// A started or completed quest's status; `None` if not started.
    pub fn status(&self, quest_id: i16) -> Option<&QuestStatus> {
        self.records.get(&quest_id).map(|r| &r.status)
    }

    pub fn state(&self, quest_id: i16) -> QuestState {
        match self.status(quest_id) {
            None => QuestState::NotStarted,
            Some(QuestStatus::Started { .. }) => QuestState::Started,
            Some(QuestStatus::Completed { .. }) => QuestState::Completed,
        }
    }

    pub fn started(&self) -> impl Iterator<Item = &QuestRecord> {
        self.records
            .values()
            .filter(|r| matches!(r.status, QuestStatus::Started { .. }))
    }

    pub fn completed(&self) -> impl Iterator<Item = (i16, SystemTime)> + '_ {
        self.records.values().filter_map(|r| match r.status {
            QuestStatus::Completed { at } => Some((r.quest_id, at)),
            QuestStatus::Started { .. } => None,
        })
    }
}

/// Kill counts as the v83 progress string ("003012" for 3 and 12).
pub fn encode_kills(kills: &[u16]) -> String {
    kills
        .iter()
        .map(|count| format!("{:03}", count.min(&999)))
        .collect()
}

/// The kill counts in a progress string (malformed parts count as 0).
pub fn decode_kills(progress: &str) -> Vec<u16> {
    progress
        .as_bytes()
        .chunks(3)
        .map(|chunk| {
            std::str::from_utf8(chunk)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timed_titles_end_is_its_utc_date_and_time() {
        use std::time::{Duration, UNIX_EPOCH};
        let at = |seconds| UNIX_EPOCH + Duration::from_secs(seconds);
        assert_eq!(end_date(at(1_234_567_890)), "20090213233130");
        assert_eq!(end_date(UNIX_EPOCH), "19700101000000");
        assert_eq!(end_date(at(951_782_400)), "20000229000000");
        assert_eq!(end_date(at(1_798_761_599)), "20261231235959");
    }

    #[test]
    fn state_tracks_start_completion_and_removal() {
        let mut log = QuestLog::default();
        assert_eq!(log.state(1031), QuestState::NotStarted);
        log.upsert(QuestRecord {
            quest_id: 1031,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
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

    #[test]
    fn kill_counts_round_trip_and_end_with_the_quest() {
        assert_eq!(encode_kills(&[3, 12]), "003012");
        assert_eq!(decode_kills("003012"), [3, 12]);
        assert_eq!(decode_kills(""), Vec::<u16>::new());
        let mut log = QuestLog::default();
        log.upsert(QuestRecord {
            quest_id: 1037,
            status: QuestStatus::Started {
                kills: vec![0],
                info: None,
            },
        });
        log.kills_mut(1037).unwrap()[0] = 4;
        assert_eq!(encode_kills(log.kills(1037)), "004");
        log.upsert(QuestRecord {
            quest_id: 1037,
            status: QuestStatus::Completed {
                at: SystemTime::UNIX_EPOCH,
            },
        });
        assert_eq!(log.kills(1037), []);
        assert_eq!(log.kills_mut(1037), None);
    }

    #[test]
    fn info_strings_change_one_character_at_a_time() {
        let mut log = QuestLog::default();
        log.upsert(QuestRecord {
            quest_id: 3926,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: Some("2222".into()),
            },
        });
        assert!(log.set_info_char(3926, 1, b'2', b'3'));
        assert!(!log.set_info_char(3926, 1, b'2', b'3'));
        assert!(!log.set_info_char(3926, 4, b'2', b'3'));
        assert!(!log.set_info_char(3927, 0, b'2', b'3'));
        assert_eq!(log.info(3926), Some("2322"));
        // Or whole, once.
        assert!(log.set_info(3926, "5"));
        assert!(!log.set_info(3926, "5"));
        assert!(!log.set_info(3927, "5"));
        assert_eq!(log.info(3926), Some("5"));
        assert!(log.set_info(3926, "2322"));
        let status = log.status(3926).unwrap();
        assert_eq!(status.record_string().as_deref(), Some("2322"));
        let kills = QuestStatus::Started {
            kills: vec![3, 12],
            info: None,
        };
        assert_eq!(kills.record_string().as_deref(), Some("003012"));
    }

    #[test]
    fn explored_maps_count_once_and_outlast_a_forfeit() {
        let mut log = QuestLog::default().with_explored([(27010, 100000000)]);
        assert!(!log.explore(27010, 100000000));
        assert!(log.explore(27010, 101000000));
        assert!(log.explore(27011, 200000000));
        assert_eq!(log.explored(27010), 2);
        assert_eq!(log.explored(27012), 0);
        log.remove(29005);
        assert_eq!(log.explored(27010), 2);
        assert_eq!(log.explored(27011), 1);
        assert_eq!(log.visits().count(), 3);
    }
}
