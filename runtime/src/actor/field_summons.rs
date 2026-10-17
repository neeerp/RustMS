//! The summons on one field: each player's birds, puppets, and octopi.
//! Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use net::packet::build::world::summon::SummonView;
use net::summons;
use net::ClientId;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

pub struct Summon {
    pub view: SummonView,
    pub owner: ClientId,
    /// A Puppet's HP; the others can't be hurt.
    pub hp: Option<i32>,
    until: Instant,
}

pub struct SummonSet {
    summons: BTreeMap<i32, Summon>,
    object_ids: ObjectIds,
}

impl Default for SummonSet {
    fn default() -> Self {
        Self {
            summons: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::SUMMONS),
        }
    }
}

impl SummonSet {
    /// Summon `view` for `lasts`, filling in its object id. A player has
    /// one summon of each kind (`summons::stat`): the one it replaces is
    /// returned.
    pub fn add(
        &mut self,
        mut view: SummonView,
        owner: ClientId,
        hp: Option<i32>,
        lasts: Duration,
        now: Instant,
    ) -> (SummonView, Option<Summon>) {
        let stat = summons::stat(view.skill_id);
        let replaced = self
            .summons
            .values()
            .find(|summon| summon.owner == owner && summons::stat(summon.view.skill_id) == stat)
            .map(|summon| summon.view.object_id)
            .and_then(|object_id| self.summons.remove(&object_id));
        view.object_id = self.object_ids.next();
        self.summons.insert(
            view.object_id,
            Summon {
                view,
                owner,
                hp,
                until: now + lasts,
            },
        );
        (view, replaced)
    }

    /// `owner`'s summon `object_id`.
    pub fn owned(&mut self, owner: ClientId, object_id: i32) -> Option<&mut Summon> {
        self.summons
            .get_mut(&object_id)
            .filter(|summon| summon.owner == owner)
    }

    pub fn remove(&mut self, object_id: i32) -> Option<Summon> {
        self.summons.remove(&object_id)
    }

    /// Remove `owner`'s summons of `skill_id`, or all of them.
    pub fn remove_owned(&mut self, owner: ClientId, skill_id: Option<i32>) -> Vec<Summon> {
        let gone: Vec<i32> = self
            .summons
            .values()
            .filter(|summon| {
                summon.owner == owner && skill_id.is_none_or(|id| id == summon.view.skill_id)
            })
            .map(|summon| summon.view.object_id)
            .collect();
        gone.iter()
            .filter_map(|object_id| self.summons.remove(object_id))
            .collect()
    }

    /// Remove the summons whose time is up.
    pub fn expire(&mut self, now: Instant) -> Vec<Summon> {
        let gone: Vec<i32> = self
            .summons
            .values()
            .filter(|summon| summon.until <= now)
            .map(|summon| summon.view.object_id)
            .collect();
        gone.iter()
            .filter_map(|object_id| self.summons.remove(object_id))
            .collect()
    }

    pub fn views(&self) -> impl Iterator<Item = &SummonView> {
        self.summons.values().map(|summon| &summon.view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use net::summons::{Assist, Moves};

    fn view(owner_id: i32, skill_id: i32) -> SummonView {
        let (moves, assist) = summons::kind(skill_id).unwrap();
        SummonView {
            owner_id,
            object_id: 0,
            skill_id,
            owner_level: 70,
            level: 20,
            x: 0,
            y: 0,
            stance: 0,
            moves,
            assist,
        }
    }

    const HAWK: i32 = 3111005;
    const PUPPET: i32 = 3111002;

    #[test]
    fn a_player_has_one_bird_and_one_puppet_until_their_time_is_up() {
        let mut set = SummonSet::default();
        let now = Instant::now();
        let lasts = Duration::from_secs(60);
        let (hawk, replaced) = set.add(view(7, HAWK), 1, None, lasts, now);
        assert_eq!(hawk.object_id, object_ids::SUMMONS.start);
        assert!(replaced.is_none());
        assert_eq!(
            (hawk.moves, hawk.assist),
            (Moves::CircleFollow, Assist::Attack)
        );
        let (puppet, replaced) = set.add(view(7, PUPPET), 1, Some(4000), lasts, now);
        assert!(replaced.is_none());
        set.add(view(8, HAWK), 2, None, lasts, now);

        let later = now + Duration::from_secs(10);
        let (again, replaced) = set.add(view(7, HAWK), 1, None, lasts, later);
        assert_eq!(replaced.unwrap().view.object_id, hawk.object_id);
        assert_eq!(set.views().count(), 3);

        assert!(set.owned(2, puppet.object_id).is_none());
        assert_eq!(set.owned(1, puppet.object_id).unwrap().hp, Some(4000));

        let gone = set.expire(now + lasts);
        let gone: Vec<i32> = gone.iter().map(|summon| summon.view.object_id).collect();
        assert_eq!(gone.len(), 2);
        assert!(!gone.contains(&again.object_id));
    }

    #[test]
    fn an_owner_dismisses_one_kind_or_all_of_theirs() {
        let mut set = SummonSet::default();
        let now = Instant::now();
        let lasts = Duration::from_secs(60);
        set.add(view(7, HAWK), 1, None, lasts, now);
        set.add(view(7, PUPPET), 1, Some(4000), lasts, now);
        set.add(view(8, HAWK), 2, None, lasts, now);
        let gone = set.remove_owned(1, Some(PUPPET));
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].view.skill_id, PUPPET);
        assert_eq!(set.remove_owned(1, None).len(), 1);
        assert_eq!(set.views().count(), 1);
    }
}
