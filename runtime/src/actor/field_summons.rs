//! The summons on one field: each player's birds, puppets, and octopi.
//! Pure state; `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use net::packet::build::world::summon::SummonView;
use net::summons::{self, Support};
use net::ClientId;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

pub struct Summon {
    pub view: SummonView,
    pub owner: ClientId,
    /// A Puppet's HP; the others can't be hurt.
    pub hp: Option<i32>,
    until: Instant,
    /// When a Beholder may next heal, and next buff, its owner: 10 s after
    /// it comes, then each skill's `x` s after the last (FUN_007a373a).
    next_heal: Instant,
    next_hex: Instant,
}

/// How much sooner than due a Beholder's owner may ask, for their clock.
const SUPPORT_SLACK: Duration = Duration::from_millis(500);

impl Summon {
    /// Whether this Beholder may give `support` at `now`; if so, it next
    /// may in that skill's time.
    pub fn may_support(&mut self, support: &Support, now: Instant) -> bool {
        if self.view.skill_id != summons::BEHOLDER {
            return false;
        }
        let next = match support {
            Support::Heal { .. } => &mut self.next_heal,
            Support::Hex { .. } => &mut self.next_hex,
        };
        if now + SUPPORT_SLACK < *next {
            return false;
        }
        *next = now + support.every();
        true
    }
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
                next_heal: now + Duration::from_secs(10),
                next_hex: now + Duration::from_secs(10),
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

    #[test]
    fn a_beholder_heals_and_buffs_at_its_skills_pace() {
        let mut set = SummonSet::default();
        let now = Instant::now();
        let lasts = Duration::from_secs(600);
        let (beholder, _) = set.add(view(7, summons::BEHOLDER), 1, None, lasts, now);
        let (puppet, _) = set.add(view(7, PUPPET), 1, Some(4000), lasts, now);
        let heal = Support::Heal {
            hp: 40,
            every: Duration::from_secs(10),
        };
        let hex = Support::Hex {
            stat: game::buff::BuffStat::Pdd,
            value: 20,
            lasts: Duration::from_secs(20),
            every: Duration::from_secs(48),
        };
        let at = |secs: f64| now + Duration::from_secs_f64(secs);

        assert!(!set
            .owned(1, puppet.object_id)
            .unwrap()
            .may_support(&heal, at(20.0)));
        let summon = set.owned(1, beholder.object_id).unwrap();
        assert!(!summon.may_support(&heal, at(9.0)), "not in its first 10 s");
        assert!(
            summon.may_support(&heal, at(9.6)),
            "a little early is the owner's clock"
        );
        assert!(!summon.may_support(&heal, at(15.0)));
        assert!(summon.may_support(&hex, at(15.0)), "each has its own time");
        assert!(summon.may_support(&heal, at(19.6)));
        assert!(!summon.may_support(&hex, at(60.0)));
        assert!(summon.may_support(&hex, at(63.0)));
    }
}
