//! The reactors of one field: breakable boxes and the like. Pure state;
//! `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use game_data::reactors::ReactorTemplate;
use game_data::MapReactorTemplate;
use net::packet::build::world::reactor::ReactorView;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

/// How far from a reactor a player can stand and still hit it: a little more
/// than the longest melee reach, since the field's idea of where the player
/// is trails their movement.
const REACH_X: i32 = 250;
const REACH_Y: i32 = 200;

struct Spot {
    template: MapReactorTemplate,
    data: ReactorTemplate,
    live: Option<i32>,
    next_spawn: Option<Instant>,
    /// Broken for good: it has no positive `reactorTime`.
    spent: bool,
}

pub struct LiveReactor {
    pub object_id: i32,
    spot: usize,
    pub reactor_id: i32,
    pub state: u8,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
}

impl LiveReactor {
    pub fn view(&self) -> ReactorView {
        ReactorView {
            object_id: self.object_id,
            reactor_id: self.reactor_id,
            state: self.state,
            x: self.x,
            y: self.y,
            flip: self.flip,
        }
    }
}

/// What a hit did.
#[derive(Debug, PartialEq, Eq)]
pub enum Hit {
    /// The reactor moved to the view's state.
    Moved(ReactorView),
    /// It broke: it is gone (the view has the state it broke from) and drops
    /// its loot.
    Broke(ReactorView),
}

pub struct ReactorSet {
    spots: Vec<Spot>,
    live: BTreeMap<i32, LiveReactor>,
    object_ids: ObjectIds,
}

impl Default for ReactorSet {
    fn default() -> Self {
        Self::new([])
    }
}

impl ReactorSet {
    pub fn new(spots: impl IntoIterator<Item = (MapReactorTemplate, ReactorTemplate)>) -> Self {
        Self {
            spots: spots
                .into_iter()
                .map(|(template, data)| Spot {
                    template,
                    data,
                    live: None,
                    next_spawn: None,
                    spent: false,
                })
                .collect(),
            live: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::REACTORS),
        }
    }

    pub fn live(&self) -> impl Iterator<Item = &LiveReactor> {
        self.live.values()
    }

    pub fn get(&self, object_id: i32) -> Option<&LiveReactor> {
        self.live.get(&object_id)
    }

    /// Spawn every empty spot that is due; returns the new reactors' object
    /// ids. Each spawn gets a fresh id: the client keeps a broken reactor
    /// under its old one.
    pub fn respawn(&mut self, now: Instant) -> Vec<i32> {
        let mut spawned = Vec::new();
        for (index, spot) in self.spots.iter_mut().enumerate() {
            if spot.live.is_some() || spot.spent || spot.next_spawn.is_some_and(|at| at > now) {
                continue;
            }
            let object_id = self.object_ids.next();
            spot.live = Some(object_id);
            spot.next_spawn = None;
            self.live.insert(
                object_id,
                LiveReactor {
                    object_id,
                    spot: index,
                    reactor_id: spot.template.reactor_id,
                    state: 0,
                    x: spot.template.x,
                    y: spot.template.y,
                    flip: spot.template.flip,
                },
            );
            spawned.push(object_id);
        }
        spawned
    }

    /// A player at `from` hit `object_id` with `skill_id` (0 for a plain
    /// attack). `None` if it is gone, out of reach, or can't be hit now.
    pub fn hit(
        &mut self,
        object_id: i32,
        skill_id: i32,
        from: (i16, i16),
        now: Instant,
    ) -> Option<Hit> {
        let reactor = self.live.get_mut(&object_id)?;
        if (i32::from(reactor.x) - i32::from(from.0)).abs() > REACH_X
            || (i32::from(reactor.y) - i32::from(from.1)).abs() > REACH_Y
        {
            return None;
        }
        let spot = &mut self.spots[reactor.spot];
        let next = spot.data.hit(reactor.state, skill_id)?;
        if !spot.data.is_last(next) {
            reactor.state = next;
            return Some(Hit::Moved(reactor.view()));
        }
        let broken = self.live.remove(&object_id)?;
        spot.live = None;
        match u64::try_from(spot.template.reactor_time) {
            Ok(secs) if secs > 0 => spot.next_spawn = Some(now + Duration::from_secs(secs)),
            _ => spot.spent = true,
        }
        Some(Hit::Broke(broken.view()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_data::reactors::ReactorState;

    fn scrap_box(x: i16, reactor_time: i32) -> (MapReactorTemplate, ReactorTemplate) {
        let hit = |next| ReactorState {
            hit: Some(next),
            skills: Vec::new(),
            last: false,
        };
        (
            MapReactorTemplate {
                reactor_id: 2001,
                x,
                y: 254,
                flip: false,
                reactor_time,
                name: String::new(),
            },
            ReactorTemplate {
                reactor_id: 2001,
                states: vec![
                    hit(1),
                    hit(2),
                    hit(3),
                    hit(4),
                    ReactorState {
                        last: true,
                        ..ReactorState::default()
                    },
                ],
                action: Some("mBoxItem0".into()),
            },
        )
    }

    #[test]
    fn a_box_breaks_on_the_fourth_hit_and_comes_back() {
        let mut reactors = ReactorSet::new([scrap_box(610, 90)]);
        let start = Instant::now();
        let [id] = reactors.respawn(start)[..] else {
            panic!("one reactor");
        };
        for state in 1..=3 {
            match reactors.hit(id, 0, (600, 254), start) {
                Some(Hit::Moved(view)) => assert_eq!(view.state, state),
                other => panic!("hit {state}: {other:?}"),
            }
        }
        match reactors.hit(id, 0, (600, 254), start) {
            Some(Hit::Broke(view)) => assert_eq!((view.object_id, view.state), (id, 3)),
            other => panic!("last hit: {other:?}"),
        }
        assert!(reactors.get(id).is_none());
        assert!(reactors.hit(id, 0, (600, 254), start).is_none());

        assert!(reactors.respawn(start + Duration::from_secs(89)).is_empty());
        let respawned = reactors.respawn(start + Duration::from_secs(90));
        assert_eq!(respawned.len(), 1);
        assert_ne!(respawned[0], id);
        assert_eq!(reactors.get(respawned[0]).unwrap().state, 0);
    }

    #[test]
    fn a_box_out_of_reach_is_not_hit() {
        let mut reactors = ReactorSet::new([scrap_box(610, 90)]);
        let now = Instant::now();
        let id = reactors.respawn(now)[0];
        assert!(reactors.hit(id, 0, (1200, 254), now).is_none());
        assert!(reactors.hit(id, 0, (610, -100), now).is_none());
        assert_eq!(reactors.get(id).unwrap().state, 0);
    }

    #[test]
    fn a_box_without_reactor_time_stays_broken() {
        let mut reactors = ReactorSet::new([scrap_box(610, 0)]);
        let now = Instant::now();
        let id = reactors.respawn(now)[0];
        for _ in 0..4 {
            reactors.hit(id, 0, (610, 254), now).unwrap();
        }
        assert!(reactors.respawn(now + Duration::from_secs(3600)).is_empty());
    }
}
