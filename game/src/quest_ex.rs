//! The records the client keeps beside the quest log, by quest ("quest
//! record ex"): each a `k=v;k=v` string, whose values the Quest Log's texts
//! show through their `#j` tokens. The party quests keep their tries,
//! clears, best times and ranks in them, on their info quests (1200-1206,
//! 1301, started as records: `crate::quest::RECORDS`).

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuestExRecords(BTreeMap<i16, String>);

impl QuestExRecords {
    pub fn from_raw(records: impl IntoIterator<Item = (i16, String)>) -> Self {
        Self(records.into_iter().collect())
    }

    /// Each quest's record as the client and the database hold it.
    pub fn iter(&self) -> impl Iterator<Item = (i16, &str)> {
        self.0
            .iter()
            .map(|(&quest_id, raw)| (quest_id, raw.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn raw(&self, quest_id: i16) -> Option<&str> {
        self.0.get(&quest_id).map(String::as_str)
    }

    /// The value under `key` in `quest_id`'s record.
    pub fn get(&self, quest_id: i16, key: &str) -> Option<&str> {
        pairs(self.raw(quest_id)?)
            .find(|&(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// `get` read as a number, 0 if absent.
    pub fn number(&self, quest_id: i16, key: &str) -> u32 {
        self.get(quest_id, key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    /// Set `key` in `quest_id`'s record, in its place if it is there.
    pub fn set(&mut self, quest_id: i16, key: &str, value: impl ToString) {
        let value = value.to_string();
        let raw = self.0.entry(quest_id).or_default();
        let mut found = false;
        let mut next: Vec<String> = pairs(raw)
            .map(|(k, v)| {
                if k == key {
                    found = true;
                    format!("{k}={value}")
                } else {
                    format!("{k}={v}")
                }
            })
            .collect();
        if !found {
            next.push(format!("{key}={value}"));
        }
        *raw = next.join(";");
    }
}

/// The client splits a record on `;`, then each part on `=`.
fn pairs(raw: &str) -> impl Iterator<Item = (&str, &str)> {
    raw.split(';').filter_map(|pair| pair.split_once('='))
}

/// What happened to the player in a party quest, for its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyQuestOutcome {
    /// They entered it: one more try.
    Entered,
    /// Their team won it this long after they entered, on `at`.
    Cleared { took: Duration, at: SystemTime },
    /// A Monster Carnival battle ended.
    Carnival(CarnivalOutcome),
    /// They left a won party quest, perhaps with its reward: only the
    /// rewards are looked at again.
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarnivalOutcome {
    Won,
    Lost,
    Drew,
}

/// The fewest wins (clears, or a Monster Carnival's victories) for each
/// rank, highest first; below them all is F. The client draws the rank's
/// image (`UI/Basic.img/PQuestRank`) and has no rule of its own.
pub const RANKS: &[(&str, u32)] = &[("S", 100), ("A", 50), ("B", 30), ("C", 10), ("D", 1)];

pub fn rank(wins: u32) -> &'static str {
    RANKS
        .iter()
        .find(|&&(_, least)| wins >= least)
        .map_or("F", |&(rank, _)| rank)
}

/// Record `outcome` in the party quest record `quest_id`; `held` says, for
/// each of its reward items in turn, whether the player has it, kept as
/// `have` for a single item or `have0`, `have1`... for several. A reward
/// once had stays had.
pub fn record_party_quest(
    records: &mut QuestExRecords,
    quest_id: i16,
    outcome: PartyQuestOutcome,
    held: &[bool],
) {
    let bump = |records: &mut QuestExRecords, key: &str| {
        let n = records.number(quest_id, key) + 1;
        records.set(quest_id, key, n);
    };
    match outcome {
        PartyQuestOutcome::Entered => bump(records, "try"),
        PartyQuestOutcome::Cleared { took, at } => {
            bump(records, "cmp");
            let secs = took.as_secs();
            let best = records
                .get(quest_id, "min")
                .zip(records.get(quest_id, "sec"))
                .and_then(|(min, sec)| {
                    Some(min.parse::<u64>().ok()? * 60 + sec.parse::<u64>().ok()?)
                });
            if best.map_or(true, |best| secs < best) {
                records.set(quest_id, "min", secs / 60);
                records.set(quest_id, "sec", secs % 60);
                records.set(quest_id, "date", date(at));
            }
        }
        PartyQuestOutcome::Carnival(CarnivalOutcome::Won) => bump(records, "vic"),
        PartyQuestOutcome::Carnival(CarnivalOutcome::Lost) => bump(records, "lose"),
        PartyQuestOutcome::Carnival(CarnivalOutcome::Drew) => bump(records, "draw"),
        PartyQuestOutcome::Left => {}
    }
    for (i, &has) in held.iter().enumerate() {
        let key = if held.len() == 1 {
            "have".to_string()
        } else {
            format!("have{i}")
        };
        let had = has || records.number(quest_id, &key) > 0;
        records.set(quest_id, &key, u8::from(had));
    }
    let wins = records.number(quest_id, "cmp") + records.number(quest_id, "vic");
    records.set(quest_id, "rank", rank(wins));
}

/// `at`'s day (UTC) as the record's `date` shows it, `yyyy/mm/dd`.
fn date(at: SystemTime) -> String {
    let days = at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400) as i64;
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}/{month:02}/{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_set_in_place_or_added() {
        let mut records = QuestExRecords::from_raw([(1200, "try=1;cmp=0".to_string())]);
        records.set(1200, "try", 2);
        records.set(1200, "rank", "D");
        assert_eq!(records.raw(1200), Some("try=2;cmp=0;rank=D"));
        assert_eq!(records.get(1200, "cmp"), Some("0"));
        assert_eq!(records.number(1201, "try"), 0);
    }

    #[test]
    fn a_clear_counts_and_keeps_the_best_time() {
        let mut records = QuestExRecords::default();
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_234_567_890);
        record_party_quest(&mut records, 1201, PartyQuestOutcome::Entered, &[false]);
        assert_eq!(records.raw(1201), Some("try=1;have=0;rank=F"));
        let cleared = |took| PartyQuestOutcome::Cleared {
            took: Duration::from_secs(took),
            at,
        };
        record_party_quest(&mut records, 1201, cleared(754), &[true]);
        record_party_quest(&mut records, 1201, cleared(900), &[false]);
        assert_eq!(
            records.raw(1201),
            Some("try=1;have=1;rank=D;cmp=2;min=12;sec=34;date=2009/02/13")
        );
        let mut left = QuestExRecords::default();
        record_party_quest(&mut left, 1200, PartyQuestOutcome::Entered, &[false]);
        record_party_quest(&mut left, 1200, PartyQuestOutcome::Left, &[true]);
        assert_eq!(left.raw(1200), Some("try=1;have=1;rank=F"));
    }

    #[test]
    fn carnival_outcomes_and_several_rewards() {
        let mut records = QuestExRecords::default();
        for outcome in [
            CarnivalOutcome::Won,
            CarnivalOutcome::Lost,
            CarnivalOutcome::Drew,
        ] {
            record_party_quest(
                &mut records,
                1301,
                PartyQuestOutcome::Carnival(outcome),
                &[false, true],
            );
        }
        assert_eq!(
            records.raw(1301),
            Some("vic=1;have0=0;have1=1;rank=D;lose=1;draw=1")
        );
    }

    #[test]
    fn ranks_climb_with_wins() {
        assert_eq!(rank(0), "F");
        assert_eq!(rank(1), "D");
        assert_eq!(rank(10), "C");
        assert_eq!(rank(99), "A");
        assert_eq!(rank(100), "S");
    }
}
