//! Party searches (`PARTY_SEARCH_START`, reports b034 and b035). A party's
//! leader asks for members of some levels and jobs, and every so often the
//! world directory invites one character who fits. Pure state; the
//! directory picks the characters and sends the packets.

use net::ClientId;
use std::collections::{BTreeMap, HashSet};

/// What the leader asked for in v83's Party Search window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub min_level: i32,
    pub max_level: i32,
    /// How big the party should grow; v83 stops searching once it has.
    pub members: i32,
    /// The window's job checkboxes, one bit each (see [`job_bit`]).
    pub jobs: u32,
}

/// The highest level there is, which v83 won't search past.
const MAX_LEVEL: i32 = 200;
/// The widest level range v83 searches.
const LEVEL_SPAN: i32 = 30;
/// The window's 25 checkboxes.
const JOB_BITS: u32 = (1 << 25) - 1;

impl Wanted {
    /// v83's own checks before it sends the search (FUN_0087943e), for a
    /// leader of `level` whose party has `size` members.
    pub fn is_valid(&self, level: u8, size: usize) -> bool {
        let level = i32::from(level);
        self.max_level <= MAX_LEVEL
            && self.max_level - self.min_level <= LEVEL_SPAN
            && self.min_level <= self.max_level
            && (self.min_level..=self.max_level).contains(&level)
            && (2..=6).contains(&self.members)
            && self.members as usize > size
            && self.jobs != 0
            && self.jobs & !JOB_BITS == 0
    }

    pub fn fits(&self, level: u8, job: i16) -> bool {
        (self.min_level..=self.max_level).contains(&i32::from(level)) && self.jobs & job_bit(job) != 0
    }
}

/// The checkbox that covers `job` (FUN_0087796a's table): a beginner, Aran,
/// and then for each class its head, its three branches, and its Cygnus
/// knight. A box covers every advancement of its branch. Jobs none names,
/// such as GMs', count as the "All" box.
pub fn job_bit(job: i16) -> u32 {
    let n = match job {
        _ if job % 1000 == 0 => 1,
        2100..=2199 => 2,
        100 => 3,
        110..=119 => 4,
        120..=129 => 5,
        130..=139 => 6,
        1100..=1199 => 7,
        200 => 8,
        210..=219 => 9,
        220..=229 => 10,
        230..=239 => 11,
        1200..=1299 => 12,
        500 => 13,
        510..=519 => 14,
        520..=529 => 15,
        1500..=1599 => 16,
        400 => 17,
        410..=419 => 18,
        420..=429 => 19,
        1400..=1499 => 20,
        300 => 21,
        310..=319 => 22,
        320..=329 => 23,
        1300..=1399 => 24,
        _ => 0,
    };
    1 << n
}

struct Search {
    party_id: i32,
    wanted: Wanted,
    /// Asked already; nobody is asked twice by one party's search.
    invited: HashSet<ClientId>,
}

#[derive(Default)]
pub struct PartySearches {
    /// By leader, in a fixed order so each round goes the same way.
    searches: BTreeMap<ClientId, Search>,
}

impl PartySearches {
    /// Start `leader`'s search for `party_id`, afresh: v83 pauses with the
    /// same packet as it stops, so resuming is a new start.
    pub fn start(&mut self, leader: ClientId, party_id: i32, wanted: Wanted) {
        let search = Search {
            party_id,
            wanted,
            invited: HashSet::new(),
        };
        self.searches.insert(leader, search);
    }

    pub fn stop(&mut self, leader: ClientId) {
        self.searches.remove(&leader);
    }

    /// Each search: its leader, party, and what it wants.
    pub fn all(&self) -> Vec<(ClientId, i32, Wanted)> {
        self.searches
            .iter()
            .map(|(&leader, search)| (leader, search.party_id, search.wanted))
            .collect()
    }

    pub fn has_invited(&self, leader: ClientId, who: ClientId) -> bool {
        self.searches
            .get(&leader)
            .is_some_and(|search| search.invited.contains(&who))
    }

    pub fn invited(&mut self, leader: ClientId, who: ClientId) {
        if let Some(search) = self.searches.get_mut(&leader) {
            search.invited.insert(who);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted(jobs: u32) -> Wanted {
        Wanted {
            min_level: 20,
            max_level: 40,
            members: 6,
            jobs,
        }
    }

    #[test]
    fn checks_what_v83_checks() {
        let all = JOB_BITS;
        assert!(wanted(all).is_valid(30, 1));
        assert!(!wanted(all).is_valid(19, 1));
        assert!(!wanted(all).is_valid(41, 1));
        assert!(!wanted(0).is_valid(30, 1));
        assert!(!wanted(1 << 25).is_valid(30, 1));
        let wide = Wanted {
            max_level: 51,
            ..wanted(all)
        };
        assert!(!wide.is_valid(30, 1));
        let backwards = Wanted {
            min_level: 41,
            ..wanted(all)
        };
        assert!(!backwards.is_valid(30, 1));
        let small = Wanted {
            members: 3,
            ..wanted(all)
        };
        assert!(small.is_valid(30, 2));
        assert!(!small.is_valid(30, 3));
        let one = Wanted {
            members: 1,
            ..wanted(all)
        };
        assert!(!one.is_valid(30, 0));
    }

    #[test]
    fn a_box_covers_its_whole_branch() {
        assert_eq!(job_bit(0), 1 << 1);
        assert_eq!(job_bit(1000), 1 << 1);
        assert_eq!(job_bit(2112), 1 << 2);
        assert_eq!(job_bit(100), 1 << 3);
        assert_eq!(job_bit(112), 1 << 4);
        assert_eq!(job_bit(1111), 1 << 7);
        assert_eq!(job_bit(232), 1 << 11);
        assert_eq!(job_bit(522), 1 << 15);
        assert_eq!(job_bit(1400), 1 << 20);
        assert_eq!(job_bit(312), 1 << 22);
        assert_eq!(job_bit(1312), 1 << 24);
        assert_eq!(job_bit(900), 1);
        assert!(wanted(1 << 4).fits(25, 111));
        assert!(!wanted(1 << 4).fits(25, 121));
        assert!(!wanted(1 << 4).fits(41, 111));
    }

    #[test]
    fn a_search_remembers_who_it_asked() {
        let mut searches = PartySearches::default();
        searches.start(1, 7, wanted(JOB_BITS));
        searches.invited(1, 2);
        assert!(searches.has_invited(1, 2));
        assert!(!searches.has_invited(1, 3));
        searches.start(1, 7, wanted(1 << 4));
        assert!(!searches.has_invited(1, 2));
        assert_eq!(searches.all(), [(1, 7, wanted(1 << 4))]);
        searches.stop(1);
        assert!(searches.all().is_empty());
    }
}
