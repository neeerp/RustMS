//! Items and mesos lying on one field. Pure state; `FieldActor` sends the
//! packets.

use crate::actor::object_ids::{self, ObjectIds};
use game::pet::IGNORED_MESOS;
use game_data::footholds::Footholds;
use net::loot::Loot;
use net::packet::build::world::drop::DropView;
use net::ClientId;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

/// How long only the killer, or their party, may pick a drop up.
pub const OWNERSHIP: Duration = Duration::from_secs(15);
/// How long a drop lies on the ground.
pub const LIFETIME: Duration = Duration::from_secs(180);
/// How far away a player may pick something up.
const REACH: (i32, i32) = (800, 600);
/// Spacing between drops from one kill.
const SPREAD: i16 = 25;
/// Spacing between drops a boss flings out (`explosiveReward`).
const EXPLOSIVE_SPREAD: i16 = 40;

/// Who may pick a drop up before `OWNERSHIP` runs out: the v83 ownership
/// type, which its client reads as 0 (the owner, by character id), 1 (the
/// owner's party, by party id), 2 (anyone) or 3 (anyone, flung out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ownership {
    Owner,
    Party(i32),
    /// A mob with `publicReward`.
    FreeForAll,
    /// A mob with `explosiveReward`.
    Explosive,
}

#[derive(Clone, Debug)]
pub struct Drop {
    pub object_id: i32,
    pub loot: Loot,
    pub owner: ClientId,
    pub ownership: Ownership,
    pub x: i16,
    pub y: i16,
    pub dropper: i32,
    pub dropped_at: Instant,
    /// A player threw it away, so anyone may pick it up at once.
    pub player_drop: bool,
}

impl Drop {
    pub fn view(&self, now: Instant) -> DropView {
        let (item_or_meso, meso) = match self.loot {
            Loot::Item { item_id, .. } => (item_id, false),
            Loot::Mesos(amount) => (amount, true),
            Loot::Dropped(ref item) => (item.item_id, false),
        };
        DropView {
            object_id: self.object_id,
            item_or_meso,
            meso,
            owner: match self.ownership {
                Ownership::Party(party) if self.owned(now) => party,
                _ => self.owner,
            },
            ownership: match self.ownership {
                Ownership::Explosive => 3,
                _ if !self.owned(now) => 2,
                Ownership::Owner => 0,
                Ownership::Party(_) => 1,
                Ownership::FreeForAll => 2,
            },
            x: self.x,
            y: self.y,
            dropper: self.dropper,
            player_drop: self.player_drop,
        }
    }

    /// Who may see it: quest items only show to their owner.
    pub fn visible_to(&self, client: ClientId) -> bool {
        self.loot.quest().is_none() || self.owner == client
    }

    /// Still kept for its owner or their party.
    fn owned(&self, now: Instant) -> bool {
        !self.player_drop && now < self.dropped_at + OWNERSHIP
    }

    /// Whether `client`, in `party`, may pick it up: quest items only their
    /// owner, others whoever its ownership allows until it runs out.
    fn takeable_by(&self, client: ClientId, party: Option<i32>, now: Instant) -> bool {
        if self.owner == client {
            return true;
        }
        if self.loot.quest().is_some() {
            return false;
        }
        match self.ownership {
            Ownership::Owner => !self.owned(now),
            Ownership::Party(owners) => party == Some(owners) || !self.owned(now),
            Ownership::FreeForAll | Ownership::Explosive => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PickupError {
    Gone,
    TooFar,
    NotYours,
    /// A pet without the item for it, or a drop a player threw away.
    NotForPets,
    /// Nobody may pick it up (`pickUpBlock`).
    Blocked,
}

pub struct DropSet {
    drops: BTreeMap<i32, Drop>,
    object_ids: ObjectIds,
    footholds: Footholds,
    /// How long a drop lies here; `None` on everlast maps, where drops
    /// stay until picked up.
    lifetime: Option<Duration>,
}

impl Default for DropSet {
    fn default() -> Self {
        Self::new(Footholds::default())
    }
}

impl DropSet {
    /// Drops for a field with these footholds to land on.
    pub fn new(footholds: Footholds) -> Self {
        Self {
            drops: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::DROPS),
            footholds,
            lifetime: Some(LIFETIME),
        }
    }

    /// Drops that never vanish (`info/everlast`).
    pub fn everlast(self) -> Self {
        Self {
            lifetime: None,
            ..self
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &Drop> {
        self.drops.values()
    }

    /// The highest ground at `x` that is at or below `y`.
    pub fn ground_below(&self, x: i16, y: i16) -> Option<i16> {
        self.footholds.ground_below(x, y)
    }

    /// Spread `loot` around (x, y), each on the ground below its place (see
    /// `Footholds::drop_position`), and return the new drops.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        loot: Vec<Loot>,
        owner: ClientId,
        ownership: Ownership,
        x: i16,
        y: i16,
        dropper: i32,
        now: Instant,
    ) -> Vec<&Drop> {
        let count = loot.len() as i16;
        let spread = if ownership == Ownership::Explosive {
            EXPLOSIVE_SPREAD
        } else {
            SPREAD
        };
        let mut ids = Vec::new();
        for (index, loot) in loot.into_iter().enumerate() {
            let object_id = self.object_ids.next();
            let offset = (index as i16 * 2 - (count - 1)) * spread / 2;
            let (drop_x, drop_y) = self
                .footholds
                .drop_position((x, y), x.saturating_add(offset));
            self.drops.insert(
                object_id,
                Drop {
                    object_id,
                    loot,
                    owner,
                    ownership,
                    x: drop_x,
                    y: drop_y,
                    dropper,
                    dropped_at: now,
                    player_drop: false,
                },
            );
            ids.push(object_id);
        }
        ids.iter().filter_map(|id| self.drops.get(id)).collect()
    }

    /// Shake `amount` mesos out of the mob `dropper` at `from` for anyone,
    /// pets aside, on the ground below `place` (Pickpocket).
    pub fn pocket(
        &mut self,
        amount: i32,
        owner: ClientId,
        from: (i16, i16),
        place: i16,
        dropper: i32,
        now: Instant,
    ) -> &Drop {
        let object_id = self.object_ids.next();
        let (x, y) = self.footholds.drop_position(from, place);
        self.drops.entry(object_id).or_insert(Drop {
            object_id,
            loot: Loot::Mesos(amount),
            owner,
            ownership: Ownership::FreeForAll,
            x,
            y,
            dropper,
            dropped_at: now,
            player_drop: true,
        })
    }

    /// Put down what `owner` threw away at (x, y).
    pub fn throw(&mut self, loot: Loot, owner: ClientId, x: i16, y: i16, now: Instant) -> &Drop {
        let object_id = self.object_ids.next();
        self.drops.entry(object_id).or_insert(Drop {
            object_id,
            loot,
            owner,
            ownership: Ownership::Owner,
            x,
            y,
            dropper: 0,
            dropped_at: now,
            player_drop: true,
        })
    }

    /// An object id for a drop that is shown but never lies on the field.
    pub fn next_object_id(&mut self) -> i32 {
        self.object_ids.next()
    }

    /// Take a drop for `client`, in `party`, standing at (x, y).
    pub fn take(
        &mut self,
        client: ClientId,
        party: Option<i32>,
        object_id: i32,
        x: i16,
        y: i16,
        now: Instant,
    ) -> Result<Drop, PickupError> {
        let drop = self.drops.get(&object_id).ok_or(PickupError::Gone)?;
        if drop.loot.blocked() {
            return Err(PickupError::Blocked);
        }
        if (i32::from(drop.x) - i32::from(x)).abs() > REACH.0
            || (i32::from(drop.y) - i32::from(y)).abs() > REACH.1
        {
            return Err(PickupError::TooFar);
        }
        if !drop.takeable_by(client, party, now) {
            return Err(PickupError::NotYours);
        }
        self.drops.remove(&object_id).ok_or(PickupError::Gone)
    }

    /// Take a drop for the pet of `client` standing at (x, y), which takes
    /// mesos and items as `takes` says, but not the ids `ignored` (mesos
    /// as `IGNORED_MESOS`). As in v83 (FUN_0050483a), pets leave what
    /// players threw away.
    pub fn take_for_pet(
        &mut self,
        client: ClientId,
        party: Option<i32>,
        object_id: i32,
        (x, y): (i16, i16),
        now: Instant,
        (mesos, items): (bool, bool),
        ignored: &[i32],
    ) -> Result<Drop, PickupError> {
        let drop = self.drops.get(&object_id).ok_or(PickupError::Gone)?;
        let (wanted, id) = match &drop.loot {
            Loot::Mesos(_) => (mesos, IGNORED_MESOS),
            Loot::Item { item_id, .. } => (items, *item_id),
            Loot::Dropped(item) => (items, item.item_id),
        };
        if drop.player_drop || !wanted || ignored.contains(&id) {
            return Err(PickupError::NotForPets);
        }
        self.take(client, party, object_id, x, y, now)
    }

    /// Set off mesos for `client`, in `party`, with Meso Explosion: the
    /// mesos they may take where it `reaches`, which are gone at once.
    pub fn explode(
        &mut self,
        client: ClientId,
        party: Option<i32>,
        object_id: i32,
        reaches: impl Fn(i16, i16) -> bool,
        now: Instant,
    ) -> Option<Drop> {
        let drop = self.drops.get(&object_id)?;
        let mesos = matches!(drop.loot, Loot::Mesos(_));
        if !mesos || !drop.takeable_by(client, party, now) || !reaches(drop.x, drop.y) {
            return None;
        }
        self.drops.remove(&object_id)
    }

    /// Put back a drop that couldn't be picked up (a full inventory).
    pub fn restore(&mut self, drop: Drop) {
        self.drops.insert(drop.object_id, drop);
    }

    /// Remove expired drops and return their object ids.
    pub fn expire(&mut self, now: Instant) -> Vec<i32> {
        let Some(lifetime) = self.lifetime else {
            return Vec::new();
        };
        let expired: Vec<i32> = self
            .drops
            .values()
            .filter(|drop| now >= drop.dropped_at + lifetime)
            .map(|drop| drop.object_id)
            .collect();
        for id in &expired {
            self.drops.remove(id);
        }
        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owners_pick_up_first_then_anyone() {
        let now = Instant::now();
        let mut drops = DropSet::default();
        let ids: Vec<i32> = drops
            .spawn(
                vec![
                    Loot::Mesos(5),
                    Loot::Item {
                        item_id: 4000019,
                        quantity: 1,
                        quest: None,
                    },
                ],
                1,
                Ownership::Owner,
                100,
                0,
                9,
                now,
            )
            .iter()
            .map(|d| d.object_id)
            .collect();
        assert_eq!(ids.len(), 2);
        assert_eq!(drops.all().map(|d| d.x).collect::<Vec<_>>(), [88, 112]);
        assert_eq!(
            drops.take(2, None, ids[0], 100, 0, now).unwrap_err(),
            PickupError::NotYours
        );
        assert_eq!(
            drops.take(1, None, ids[0], 2000, 0, now).unwrap_err(),
            PickupError::TooFar
        );
        assert_eq!(
            drops.take(1, None, ids[0], 100, 0, now).unwrap().loot,
            Loot::Mesos(5)
        );
        assert_eq!(
            drops.take(1, None, ids[0], 100, 0, now).unwrap_err(),
            PickupError::Gone
        );
        assert!(drops.take(2, None, ids[1], 100, 0, now + OWNERSHIP).is_ok());
    }

    #[test]
    fn pets_take_what_their_items_allow_but_not_what_players_threw() {
        let now = Instant::now();
        let mut drops = DropSet::default();
        let ids: Vec<i32> = drops
            .spawn(
                vec![
                    Loot::Mesos(5),
                    Loot::Item {
                        item_id: 4000019,
                        quantity: 1,
                        quest: None,
                    },
                ],
                1,
                Ownership::Owner,
                100,
                0,
                9,
                now,
            )
            .iter()
            .map(|d| d.object_id)
            .collect();
        let thrown = drops.throw(Loot::Mesos(7), 1, 100, 0, now).object_id;
        let at = (100, 0);
        // A meso magnet alone leaves items, and no pet takes what was thrown.
        assert_eq!(
            drops
                .take_for_pet(1, None, ids[1], at, now, (true, false), &[])
                .unwrap_err(),
            PickupError::NotForPets
        );
        assert_eq!(
            drops
                .take_for_pet(1, None, thrown, at, now, (true, true), &[])
                .unwrap_err(),
            PickupError::NotForPets
        );
        // Someone else's pet waits for the owner's time to pass.
        assert_eq!(
            drops
                .take_for_pet(2, None, ids[0], at, now, (true, true), &[])
                .unwrap_err(),
            PickupError::NotYours
        );
        // The exception list leaves what it names, mesos by IGNORED_MESOS.
        for (id, ignored) in [(ids[0], IGNORED_MESOS), (ids[1], 4000019)] {
            assert_eq!(
                drops
                    .take_for_pet(1, None, id, at, now, (true, true), &[ignored])
                    .unwrap_err(),
                PickupError::NotForPets
            );
        }
        let drop = drops
            .take_for_pet(1, None, ids[0], at, now, (true, false), &[4000019])
            .unwrap();
        assert_eq!(drop.loot, Loot::Mesos(5));
    }

    #[test]
    fn party_drops_are_the_partys_and_boss_drops_anyones() {
        let now = Instant::now();
        let mut drops = DropSet::default();
        let party =
            drops.spawn(vec![Loot::Mesos(5)], 1, Ownership::Party(7), 0, 0, 9, now)[0].object_id;
        let view = |drops: &DropSet, id, at| {
            let view = drops.all().find(|d| d.object_id == id).unwrap().view(at);
            (view.owner, view.ownership)
        };
        // The client goes by the party id until the time runs out.
        assert_eq!(view(&drops, party, now), (7, 1));
        assert_eq!(view(&drops, party, now + OWNERSHIP), (1, 2));
        assert_eq!(
            drops.take(2, Some(8), party, 0, 0, now).unwrap_err(),
            PickupError::NotYours
        );
        assert!(drops.take(2, Some(7), party, 0, 0, now).is_ok());

        let public =
            drops.spawn(vec![Loot::Mesos(5)], 1, Ownership::FreeForAll, 0, 0, 9, now)[0].object_id;
        assert_eq!(view(&drops, public, now), (1, 2));
        assert!(drops.take(2, None, public, 0, 0, now).is_ok());

        // Flung wider than the usual 25 apart.
        let flung: Vec<(i32, i16)> = drops
            .spawn(
                vec![Loot::Mesos(5), Loot::Mesos(6)],
                1,
                Ownership::Explosive,
                0,
                0,
                9,
                now,
            )
            .iter()
            .map(|d| (d.object_id, d.x))
            .collect();
        assert_eq!(flung.iter().map(|&(_, x)| x).collect::<Vec<_>>(), [-20, 20]);
        assert_eq!(view(&drops, flung[0].0, now), (1, 3));
        assert!(drops.take(2, None, flung[0].0, 0, 0, now).is_ok());
    }

    #[test]
    fn thrown_drops_are_anyones_at_once() {
        let now = Instant::now();
        let mut drops = DropSet::default();
        let item = game::Item::stack(2010000, 3);
        let drop = drops.throw(Loot::Dropped(item.clone()), 1, 40, 7, now);
        let (id, view) = (drop.object_id, drop.view(now));
        assert_eq!((view.item_or_meso, view.meso), (2010000, false));
        assert_eq!(
            (view.ownership, view.dropper, view.player_drop),
            (2, 0, true)
        );
        assert_eq!(
            drops.take(2, None, id, 40, 7, now).unwrap().loot,
            Loot::Dropped(item)
        );
    }

    #[test]
    fn quest_items_stay_with_their_owner_and_drops_expire() {
        let now = Instant::now();
        let mut drops = DropSet::default();
        let id = drops.spawn(
            vec![Loot::Item {
                item_id: 4031802,
                quantity: 1,
                quest: Some(1035),
            }],
            1,
            Ownership::Owner,
            0,
            0,
            9,
            now,
        )[0]
        .object_id;
        let drop = drops.all().next().unwrap();
        assert!(drop.visible_to(1) && !drop.visible_to(2));
        assert_eq!(
            drops
                .take(2, None, id, 0, 0, now + OWNERSHIP * 2)
                .unwrap_err(),
            PickupError::NotYours
        );
        assert_eq!(drops.expire(now + LIFETIME), [id]);
        assert_eq!(drops.all().count(), 0);
    }

    #[test]
    fn drops_on_everlast_maps_never_expire() {
        let now = Instant::now();
        let mut drops = DropSet::default().everlast();
        let item = game::Item::stack(2010000, 3);
        drops.throw(Loot::Dropped(item), 1, 0, 0, now);
        assert!(drops.expire(now + LIFETIME * 100).is_empty());
        assert_eq!(drops.all().count(), 1);
    }

    #[test]
    fn drops_land_on_footholds_and_not_in_walls() {
        use game_data::footholds::Foothold;

        // A cliff on the left of x 0: upper ground at y 100, lower at y 200.
        let footholds = Footholds::new(vec![
            Foothold {
                x1: -300,
                y1: 100,
                x2: 0,
                y2: 100,
            },
            Foothold {
                x1: 0,
                y1: 100,
                x2: 0,
                y2: 200,
            },
            Foothold {
                x1: 0,
                y1: 200,
                x2: 300,
                y2: 200,
            },
        ]);
        let mut drops = DropSet::new(footholds);
        // A snail stopped against the cliff, killed a little above its floor.
        drops.spawn(
            vec![Loot::Mesos(5), Loot::Mesos(6), Loot::Mesos(7)],
            1,
            Ownership::Owner,
            0,
            195,
            9,
            Instant::now(),
        );
        let placed: Vec<(i16, i16)> = drops.all().map(|d| (d.x, d.y)).collect();
        assert_eq!(placed, [(0, 200), (0, 200), (25, 200)]);
    }
}
