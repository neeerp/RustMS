//! The mists (affected areas) on one field: clouds a skill leaves that put
//! statuses on the mobs in them. Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use net::packet::build::world::mist::MistView;
use net::skills::Inflict;
use net::ClientId;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

/// How often a mist reaches for the mobs in it (OdinMS-family servers).
pub const REACH_INTERVAL: Duration = Duration::from_millis(2500);

pub struct Mist {
    pub view: MistView,
    /// Who it credits for what it does.
    pub owner: ClientId,
    pub inflict: Inflict,
    /// When it starts: the view's delay after it was made.
    start: Instant,
    until: Instant,
    next_reach: Instant,
}

impl Mist {
    /// Whether something standing at `at` is in it.
    pub fn covers(&self, at: (i16, i16)) -> bool {
        let (left, top, right, bottom) = self.view.area;
        let (x, y) = (i32::from(at.0), i32::from(at.1));
        (left..=right).contains(&x) && (top..=bottom).contains(&y)
    }
}

pub struct MistSet {
    mists: BTreeMap<i32, Mist>,
    object_ids: ObjectIds,
}

impl Default for MistSet {
    fn default() -> Self {
        Self {
            mists: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::MISTS),
        }
    }
}

impl MistSet {
    /// Make a mist, filling in its object id, that starts after the view's
    /// delay and lasts `lasts` from then.
    pub fn add(
        &mut self,
        mut view: MistView,
        owner: ClientId,
        inflict: Inflict,
        lasts: Duration,
        now: Instant,
    ) -> MistView {
        view.object_id = self.object_ids.next();
        let start = now + Duration::from_millis(100 * view.delay.max(0) as u64);
        self.mists.insert(
            view.object_id,
            Mist {
                view,
                owner,
                inflict,
                start,
                until: start + lasts,
                next_reach: start,
            },
        );
        view
    }

    pub fn views(&self) -> impl Iterator<Item = &MistView> {
        self.mists.values().map(|mist| &mist.view)
    }

    /// Remove the mists whose time is up; returns their object ids.
    pub fn expire(&mut self, now: Instant) -> Vec<i32> {
        let gone: Vec<i32> = self
            .mists
            .values()
            .filter(|mist| mist.until <= now)
            .map(|mist| mist.view.object_id)
            .collect();
        for object_id in &gone {
            self.mists.remove(object_id);
        }
        gone
    }

    /// The mists that reach for the mobs in them now, once each
    /// `REACH_INTERVAL` from their start.
    pub fn reaching(&mut self, now: Instant) -> Vec<&Mist> {
        self.mists
            .values_mut()
            .filter(|mist| mist.start <= now && mist.next_reach <= now)
            .map(|mist| {
                mist.next_reach = now + REACH_INTERVAL;
                &*mist
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::MobStat;
    use net::packet::build::world::mist::MistKind;
    use net::skills::{self, Amount};

    fn poison_mist(set: &mut MistSet, now: Instant) -> MistView {
        let inflict = Inflict {
            skill_id: skills::POISON_MIST,
            prop: 70,
            duration: Duration::from_secs(40),
            statuses: vec![(MobStat::Poison, Amount::Poison { level: 30 })],
        };
        let view = MistView {
            object_id: 0,
            kind: MistKind::Poison,
            owner_id: 7,
            skill_id: skills::POISON_MIST,
            level: 30,
            delay: 8,
            area: (-200, -150, 200, 150),
            element: 0,
        };
        set.add(view, 1, inflict, Duration::from_secs(40), now)
    }

    #[test]
    fn a_mist_reaches_from_its_start_every_interval_until_its_time_is_up() {
        let mut set = MistSet::default();
        let now = Instant::now();
        let view = poison_mist(&mut set, now);
        assert_eq!(view.object_id, object_ids::MISTS.start);
        assert!(set.reaching(now).is_empty());
        let start = now + Duration::from_millis(800);
        assert_eq!(set.reaching(start).len(), 1);
        assert!(set.reaching(start + REACH_INTERVAL / 2).is_empty());
        assert_eq!(set.reaching(start + REACH_INTERVAL).len(), 1);

        let end = start + Duration::from_secs(40);
        assert!(set.expire(end - Duration::from_millis(1)).is_empty());
        assert_eq!(set.expire(end), [view.object_id]);
        assert_eq!(set.views().count(), 0);
    }

    #[test]
    fn a_mist_covers_its_area() {
        let mut set = MistSet::default();
        let now = Instant::now();
        poison_mist(&mut set, now);
        let mist = set.reaching(now + Duration::from_secs(1))[0];
        assert!(mist.covers((200, 150)) && mist.covers((-200, -150)));
        assert!(!mist.covers((201, 0)) && !mist.covers((0, -151)));
    }
}
