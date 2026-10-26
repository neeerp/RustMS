//! The reactors of one field: breakable boxes and the like. Pure state;
//! `FieldActor` sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use game_data::reactors::ReactorTemplate;
use game_data::MapReactorTemplate;
use net::packet::build::world::reactor::ReactorView;
use rand::seq::SliceRandom;
use rand::Rng;
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
    /// Placed while the field ran (`ReactorSet::place`), not the map's.
    placed: bool,
}

pub struct LiveReactor {
    pub object_id: i32,
    spot: usize,
    pub reactor_id: i32,
    pub state: u8,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    /// When it goes back on its own, and to which state (`event/timeOut`).
    revert: Option<(Instant, u8)>,
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
    /// The reactor moved to the view's state through `event` of the state
    /// it left.
    Moved { view: ReactorView, event: u8 },
    /// It broke: it is gone (the view has the state it reached) and drops
    /// its loot.
    Broke(ReactorView),
    /// It moved as `Moved` does but does what breaking would, staying
    /// (`net::reactors::acts_staying`): the Forest of Poison Haze's pond,
    /// which drops its loot as it takes the poison and fills again on its
    /// own (`ReactorTemplate::acts_in`), and its spine.
    Acted { view: ReactorView, event: u8 },
}

pub struct ReactorSet {
    spots: Vec<Spot>,
    live: BTreeMap<i32, LiveReactor>,
    object_ids: ObjectIds,
    /// The day (Sunday 0) whose record the music box takes, fixed as the
    /// field loads (`net::reactors::music_day`).
    day: u8,
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
                    placed: false,
                })
                .collect(),
            live: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::REACTORS),
            day: net::reactors::music_day(std::time::SystemTime::now()),
        }
    }

    #[cfg(test)]
    pub fn set_day(&mut self, day: u8) {
        self.day = day;
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
                    state: net::reactors::first_state(spot.template.reactor_id),
                    x: spot.template.x,
                    y: spot.template.y,
                    flip: spot.template.flip,
                    revert: None,
                },
            );
            spawned.push(object_id);
        }
        spawned
    }

    /// Start afresh, as when the field loaded: every reactor goes without a
    /// trace and every spot gets one again, the broken ones that never come
    /// back too. For a field nobody is on. Returns the new reactors' object
    /// ids.
    /// Bring back every reactor that has broken, now, whether or not it
    /// was due to come back; those standing stay. Returns the new
    /// reactors' object ids.
    pub fn refill(&mut self, now: Instant) -> Vec<i32> {
        for spot in &mut self.spots {
            spot.spent = false;
            spot.next_spawn = None;
        }
        self.respawn(now)
    }

    pub fn renew(&mut self, now: Instant) -> Vec<i32> {
        self.live.clear();
        self.spots.retain(|spot| !spot.placed);
        for spot in &mut self.spots {
            spot.live = None;
            spot.next_spawn = None;
            spot.spent = false;
        }
        self.respawn(now)
    }

    /// Place `data`'s reactor, which the map hasn't, at `(x, y)` in
    /// `state`: once broken it never comes back, nor when the field starts
    /// afresh. Returns its object id.
    pub fn place(&mut self, data: ReactorTemplate, (x, y): (i16, i16), state: u8) -> i32 {
        let object_id = self.object_ids.next();
        let reactor_id = data.reactor_id;
        self.spots.push(Spot {
            template: MapReactorTemplate {
                reactor_id,
                x,
                y,
                flip: false,
                reactor_time: -1,
                name: String::new(),
            },
            data,
            live: Some(object_id),
            next_spawn: None,
            spent: false,
            placed: true,
        });
        self.live.insert(
            object_id,
            LiveReactor {
                object_id,
                spot: self.spots.len() - 1,
                reactor_id,
                state,
                x,
                y,
                flip: false,
                revert: None,
            },
        );
        object_id
    }

    /// Deal the live reactors' places out among them afresh, so that one
    /// may stand where another stood; those that stay put
    /// (`net::reactors::shuffles`) keep theirs.
    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        let moving = |r: &&mut LiveReactor| net::reactors::shuffles(r.reactor_id);
        let mut places: Vec<_> = (self.live.values_mut().filter(moving))
            .map(|r| (r.x, r.y))
            .collect();
        places.shuffle(rng);
        for (reactor, (x, y)) in self.live.values_mut().filter(moving).zip(places) {
            reactor.x = x;
            reactor.y = y;
        }
    }

    /// Put every reactor named `name` in `state`, as a script does (the
    /// elevator's car leaving or docking); returns the views of those that
    /// changed. It goes through no event, so nothing breaks.
    pub fn set_state(&mut self, name: &str, state: u8) -> Vec<ReactorView> {
        self.move_to(name, state, |now| now != state)
    }

    /// As `set_state`, but only those still short of `state`, so a second
    /// script asking for it (two members talking to Eak at once) can't undo
    /// the hits since the first.
    pub fn force_forward(&mut self, name: &str, state: u8) -> Vec<ReactorView> {
        self.move_to(name, state, |now| now < state)
    }

    /// Open every reactor named `name` that is shut (in state 0) to state
    /// 1, as a script hits it (Cosmic `hitReactor` on a state with no
    /// events): it shuts again when its time there is up.
    pub fn open(&mut self, name: &str, now: Instant) -> Vec<ReactorView> {
        let spots = &self.spots;
        self.live
            .values_mut()
            .filter(|reactor| spots[reactor.spot].template.name == name && reactor.state == 0)
            .map(|reactor| {
                reactor.state = 1;
                reactor.revert = (spots[reactor.spot].data.time_out(1))
                    .map(|(ms, to)| (now + Duration::from_millis(ms.into()), to));
                reactor.view()
            })
            .collect()
    }

    fn move_to(&mut self, name: &str, state: u8, moves: impl Fn(u8) -> bool) -> Vec<ReactorView> {
        let spots = &self.spots;
        self.live
            .values_mut()
            .filter(|reactor| spots[reactor.spot].template.name == name && moves(reactor.state))
            .map(|reactor| {
                reactor.state = state;
                reactor.revert = None;
                reactor.view()
            })
            .collect()
    }

    /// Put back every reactor whose time in its state is up (Cosmic
    /// `refreshReactorTimeout`); returns their views.
    pub fn revert(&mut self, now: Instant) -> Vec<ReactorView> {
        let spots = &self.spots;
        self.live
            .values_mut()
            .filter(|reactor| reactor.revert.is_some_and(|(at, _)| at <= now))
            .map(|reactor| {
                let (at, state) = reactor.revert.take().unwrap_or((now, reactor.state));
                reactor.state = state;
                reactor.revert = spots[reactor.spot]
                    .data
                    .time_out(state)
                    .map(|(ms, to)| (at + Duration::from_millis(ms.into()), to));
                reactor.view()
            })
            .collect()
    }

    /// Where reactor `reactor_id`'s spot is, whether or not it is there.
    pub fn spot_of(&self, reactor_id: i32) -> Option<(i16, i16)> {
        self.spots
            .iter()
            .find(|spot| spot.template.reactor_id == reactor_id)
            .map(|spot| (spot.template.x, spot.template.y))
    }

    /// The map's name for live reactor `object_id`, if it has one.
    pub fn name(&self, object_id: i32) -> Option<&str> {
        let reactor = self.live.get(&object_id)?;
        Some(self.spots[reactor.spot].template.name.as_str())
    }

    /// The live reactor named `name`, if any.
    pub fn named(&self, name: &str) -> Option<&LiveReactor> {
        self.live
            .values()
            .find(|reactor| self.spots[reactor.spot].template.name == name)
    }

    /// A player at `from` swung at `object_id`; `flags` are the client's
    /// (bit 0 facing left, bit 1 on a foothold). `None` if it is gone, out
    /// of reach, or no event fits.
    pub fn hit(
        &mut self,
        object_id: i32,
        flags: i32,
        from: (i16, i16),
        now: Instant,
    ) -> Option<Hit> {
        let reactor = self.live.get(&object_id)?;
        if (i32::from(reactor.x) - i32::from(from.0)).abs() > REACH_X
            || (i32::from(reactor.y) - i32::from(from.1)).abs() > REACH_Y
        {
            return None;
        }
        let (event, next) = self.spots[reactor.spot].data.swing(reactor.state, flags)?;
        self.advance(object_id, event, next, now)
    }

    /// The reactor an item lying at `(x, y)` would move on: `quantity` of
    /// `item_id`, exactly, within the area of an event of its state.
    pub fn taking(&self, item_id: i32, quantity: i16, (x, y): (i16, i16)) -> Option<i32> {
        self.live
            .values()
            .find(|reactor| {
                self.item_event(reactor, item_id, quantity, (x, y))
                    .is_some()
            })
            .map(|reactor| reactor.object_id)
    }

    /// `object_id` takes the item lying at `(x, y)`, if it still would.
    pub fn take_item(
        &mut self,
        object_id: i32,
        item_id: i32,
        quantity: i16,
        at: (i16, i16),
        now: Instant,
    ) -> Option<Hit> {
        let reactor = self.live.get(&object_id)?;
        let (event, next) = self.item_event(reactor, item_id, quantity, at)?;
        self.advance(object_id, event, next, now)
    }

    /// Whether any reactor here waits for an item now.
    pub fn any_takes_items(&self) -> bool {
        self.live
            .values()
            .any(|reactor| self.spots[reactor.spot].data.takes_items(reactor.state))
    }

    /// Whether `object_id`, in its state, waits for an item.
    pub fn takes_items(&self, object_id: i32) -> bool {
        self.live
            .get(&object_id)
            .is_some_and(|reactor| self.spots[reactor.spot].data.takes_items(reactor.state))
    }

    fn item_event(
        &self,
        reactor: &LiveReactor,
        item_id: i32,
        quantity: i16,
        (x, y): (i16, i16),
    ) -> Option<(u8, u8)> {
        let name = &self.spots[reactor.spot].template.name;
        if !net::reactors::takes(reactor.reactor_id, name, item_id, self.day) {
            return None;
        }
        let from = (x.checked_sub(reactor.x)?, y.checked_sub(reactor.y)?);
        self.spots[reactor.spot]
            .data
            .item_dropped(reactor.state, item_id, quantity, from)
    }

    /// Move `object_id` to `next` through `event`, breaking it if that is
    /// its last state, unless it is one that stays (`net::reactors::stays`).
    /// If `next` waits to go back on its own, it will, unless it is an
    /// end state (`ReactorTemplate::acts_in`), as Cosmic's `hitReactor`
    /// refreshes no timeout there; but one that acts staying
    /// (`net::reactors::acts_staying`, the forest's pond) does go back.
    fn advance(&mut self, object_id: i32, event: u8, next: u8, now: Instant) -> Option<Hit> {
        let reactor = self.live.get_mut(&object_id)?;
        let spot = &mut self.spots[reactor.spot];
        reactor.state = next;
        let last = spot.data.is_last(next);
        let ends = spot.data.acts_in(next, event);
        let acts = net::reactors::acts_staying(reactor.reactor_id) && (ends || last);
        reactor.revert = (spot.data.time_out(next))
            .filter(|_| acts || !ends)
            .map(|(ms, to)| (now + Duration::from_millis(ms.into()), to));
        if acts {
            return Some(Hit::Acted {
                view: reactor.view(),
                event,
            });
        }
        if !last || net::reactors::stays(reactor.reactor_id) {
            return Some(Hit::Moved {
                view: reactor.view(),
                event,
            });
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
    use game_data::reactors::{ReactorEvent, ReactorState, ITEM_DROPPED};
    use rand::SeedableRng;

    /// Facing right on a foothold.
    const SWING: i32 = 2;

    fn scrap_box(x: i16, reactor_time: i32) -> (MapReactorTemplate, ReactorTemplate) {
        let hit = |next| ReactorState {
            events: vec![ReactorEvent {
                kind: 0,
                next: Some(next),
                ..ReactorEvent::default()
            }],
            ..ReactorState::default()
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
                states: vec![hit(1), hit(2), hit(3), hit(4), ReactorState::default()],
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
            match reactors.hit(id, SWING, (600, 254), start) {
                Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, state),
                other => panic!("hit {state}: {other:?}"),
            }
        }
        match reactors.hit(id, SWING, (600, 254), start) {
            Some(Hit::Broke(view)) => assert_eq!((view.object_id, view.state), (id, 4)),
            other => panic!("last hit: {other:?}"),
        }
        assert!(reactors.get(id).is_none());
        assert!(reactors.hit(id, SWING, (600, 254), start).is_none());

        assert!(reactors.respawn(start + Duration::from_secs(89)).is_empty());
        let respawned = reactors.respawn(start + Duration::from_secs(90));
        assert_eq!(respawned.len(), 1);
        assert_ne!(respawned[0], id);
        assert_eq!(reactors.get(respawned[0]).unwrap().state, 0);
    }

    #[test]
    fn a_placed_reactor_breaks_for_good() {
        let (_, data) = scrap_box(0, 90);
        let mut reactors = ReactorSet::new([]);
        let start = Instant::now();
        let id = reactors.place(data, (100, 254), 1);
        assert_eq!(reactors.get(id).map(|r| (r.state, r.x)), Some((1, 100)));
        for state in 2..=3 {
            match reactors.hit(id, SWING, (100, 254), start) {
                Some(Hit::Moved { view, .. }) => assert_eq!(view.state, state),
                other => panic!("hit to {state}: {other:?}"),
            }
        }
        assert!(matches!(
            reactors.hit(id, SWING, (100, 254), start),
            Some(Hit::Broke(_))
        ));
        assert!(reactors
            .respawn(start + Duration::from_secs(900))
            .is_empty());
        assert!(reactors.renew(start).is_empty());
    }

    #[test]
    fn a_script_sets_the_state_of_the_reactors_it_names() {
        let (mut template, data) = scrap_box(-156, 0);
        template.name = "elevator".into();
        let mut reactors = ReactorSet::new(
            [template.clone(), scrap_box(610, 90).0]
                .into_iter()
                .map(|template| (template, data.clone())),
        );
        let now = Instant::now();
        let [car, other] = reactors.respawn(now)[..] else {
            panic!("two reactors");
        };
        let changed = reactors.set_state("elevator", 1);
        assert_eq!(changed.len(), 1);
        assert_eq!((changed[0].object_id, changed[0].state), (car, 1));
        assert_eq!(reactors.get(other).unwrap().state, 0);
        assert!(reactors.set_state("elevator", 1).is_empty());
        assert!(reactors.set_state("nothing", 3).is_empty());
        assert_eq!(reactors.named("elevator").unwrap().object_id, car);
        assert!(reactors.named("nothing").is_none());
        assert_eq!(reactors.set_state("elevator", 0)[0].state, 0);
        assert_eq!(reactors.force_forward("elevator", 2)[0].state, 2);
        assert!(reactors.force_forward("elevator", 1).is_empty());
    }

    #[test]
    fn a_box_out_of_reach_is_not_hit() {
        let mut reactors = ReactorSet::new([scrap_box(610, 90)]);
        let now = Instant::now();
        let id = reactors.respawn(now)[0];
        assert!(reactors.hit(id, SWING, (1200, 254), now).is_none());
        assert!(reactors.hit(id, SWING, (610, -100), now).is_none());
        assert_eq!(reactors.get(id).unwrap().state, 0);
    }

    #[test]
    fn a_box_without_reactor_time_stays_broken() {
        let mut reactors = ReactorSet::new([scrap_box(610, 0)]);
        let now = Instant::now();
        let id = reactors.respawn(now)[0];
        for _ in 0..4 {
            reactors.hit(id, SWING, (610, 254), now).unwrap();
        }
        assert!(reactors.respawn(now + Duration::from_secs(3600)).is_empty());
    }

    #[test]
    fn a_refilled_field_brings_its_broken_boxes_back_and_keeps_the_rest() {
        let mut reactors = ReactorSet::new([scrap_box(610, 0), scrap_box(700, 30)]);
        let now = Instant::now();
        let ids = reactors.respawn(now);
        for _ in 0..4 {
            reactors.hit(ids[0], SWING, (610, 254), now).unwrap();
        }
        reactors.hit(ids[1], SWING, (700, 254), now).unwrap();
        let refilled = reactors.refill(now);
        assert_eq!(refilled.len(), 1);
        assert!(!ids.contains(&refilled[0]));
        assert_eq!(reactors.get(refilled[0]).unwrap().state, 0);
        assert_eq!(reactors.get(ids[1]).unwrap().state, 1, "still hit");
        assert!(reactors.refill(now).is_empty());
    }

    #[test]
    fn a_renewed_field_has_its_broken_boxes_back() {
        let mut reactors = ReactorSet::new([scrap_box(610, 0), scrap_box(700, 30)]);
        let now = Instant::now();
        let ids = reactors.respawn(now);
        for _ in 0..4 {
            reactors.hit(ids[0], SWING, (610, 254), now).unwrap();
        }
        let renewed = reactors.renew(now);
        assert_eq!(renewed.len(), 2);
        assert_eq!(reactors.live().count(), 2);
        assert!(renewed.iter().all(|id| !ids.contains(id)));
        assert!(reactors.live().all(|reactor| reactor.state == 0));
    }

    #[test]
    fn shuffled_reactors_keep_the_same_places_between_them() {
        let xs = [100, 200, 300, 400, 500];
        let mut reactors = ReactorSet::new(xs.map(|x| scrap_box(x, 0)));
        reactors.respawn(Instant::now());
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut moved = false;
        for _ in 0..8 {
            reactors.shuffle(&mut rng);
            let mut places: Vec<_> = reactors.live().map(|r| r.x).collect();
            moved |= places != xs;
            places.sort();
            assert_eq!(places, xs);
        }
        assert!(moved);
    }

    /// A Magatia side room's way back (`2618003`), shut, which opens for
    /// two seconds.
    fn room_exit(x: i16) -> (MapReactorTemplate, ReactorTemplate) {
        (
            MapReactorTemplate {
                reactor_id: 2618003,
                x,
                y: 224,
                flip: false,
                reactor_time: 0,
                name: "rnj31_out".into(),
            },
            ReactorTemplate {
                reactor_id: 2618003,
                states: vec![
                    ReactorState::default(),
                    ReactorState {
                        events: vec![ReactorEvent {
                            kind: 101,
                            next: Some(0),
                            ..ReactorEvent::default()
                        }],
                        time_out: Some(2000),
                    },
                ],
                action: None,
            },
        )
    }

    #[test]
    fn a_magatia_room_s_way_back_stays_put_as_its_boxes_move() {
        let xs = [100, 200, 300, 400, 500];
        let boxes = xs.map(|x| scrap_box(x, 0));
        let mut reactors = ReactorSet::new(boxes.into_iter().chain([room_exit(1272)]));
        reactors.respawn(Instant::now());
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut moved = false;
        for _ in 0..8 {
            reactors.shuffle(&mut rng);
            let exit = reactors.named("rnj31_out").unwrap();
            assert_eq!((exit.x, exit.y), (1272, 224));
            let mut places: Vec<_> = (reactors.live())
                .filter(|r| r.reactor_id != 2618003)
                .map(|r| r.x)
                .collect();
            moved |= places != xs;
            places.sort();
            assert_eq!(places, xs);
        }
        assert!(moved);
    }

    #[test]
    fn a_door_opened_as_a_script_hits_it_shuts_again_on_its_own() {
        let mut reactors = ReactorSet::new([room_exit(1272)]);
        let now = Instant::now();
        let [id] = reactors.respawn(now)[..] else {
            panic!("one reactor");
        };
        // No swing opens it.
        assert!(reactors.hit(id, SWING, (1272, 224), now).is_none());
        let opened = reactors.open("rnj31_out", now);
        assert_eq!(
            opened
                .iter()
                .map(|v| (v.object_id, v.state))
                .collect::<Vec<_>>(),
            [(id, 1)]
        );
        // Open, it doesn't open again, nor wait longer.
        let later = now + Duration::from_millis(1500);
        assert!(reactors.open("rnj31_out", later).is_empty());
        assert!(reactors.open("elsewhere", now).is_empty());
        assert!(reactors
            .revert(now + Duration::from_millis(1999))
            .is_empty());
        let shut = reactors.revert(now + Duration::from_millis(2000));
        assert_eq!(
            shut.iter()
                .map(|v| (v.object_id, v.state))
                .collect::<Vec<_>>(),
            [(id, 0)]
        );
        assert!(reactors.revert(now + Duration::from_secs(60)).is_empty());
        assert_eq!(reactors.open("rnj31_out", later).len(), 1);
    }

    /// A mound that takes one Cold Flame, then one Black Book, landing
    /// within 40 px of it.
    fn mound() -> (MapReactorTemplate, ReactorTemplate) {
        let takes = |item_id, next| ReactorState {
            events: vec![ReactorEvent {
                kind: ITEM_DROPPED,
                next: Some(next),
                item: Some((item_id, 1)),
                area: Some(((-40, -40), (40, 0))),
            }],
            ..ReactorState::default()
        };
        (
            MapReactorTemplate {
                reactor_id: 2302003,
                x: 105,
                y: 164,
                flip: false,
                reactor_time: -1,
                name: String::new(),
            },
            ReactorTemplate {
                reactor_id: 2302003,
                states: vec![
                    takes(4001108, 1),
                    takes(4001107, 2),
                    ReactorState::default(),
                ],
                action: None,
            },
        )
    }

    #[test]
    fn a_mound_takes_its_items_in_turn_where_they_land() {
        let mut reactors = ReactorSet::new([mound(), scrap_box(610, 90)]);
        let now = Instant::now();
        let [id, scrap] = reactors.respawn(now)[..] else {
            panic!("two reactors");
        };
        assert!(reactors.takes_items(id) && !reactors.takes_items(scrap));
        // Not a swing, nor the wrong item, count or place.
        assert!(reactors.hit(id, SWING, (105, 164), now).is_none());
        assert_eq!(reactors.taking(4001107, 1, (105, 150)), None);
        assert_eq!(reactors.taking(4001108, 2, (105, 150)), None);
        assert_eq!(reactors.taking(4001108, 1, (160, 150)), None);
        assert_eq!(reactors.taking(4001108, 1, (105, 170)), None);
        assert_eq!(reactors.taking(4001108, 1, (105, 150)), Some(id));

        match reactors.take_item(id, 4001108, 1, (105, 150), now) {
            Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, 1),
            other => panic!("{other:?}"),
        }
        // Moved on, it no longer takes the first.
        assert!(reactors
            .take_item(id, 4001108, 1, (105, 150), now)
            .is_none());
        assert_eq!(reactors.taking(4001107, 1, (70, 124)), Some(id));
        match reactors.take_item(id, 4001107, 1, (70, 124), now) {
            Some(Hit::Broke(view)) => assert_eq!(view.state, 2),
            other => panic!("{other:?}"),
        }
        assert!(!reactors.takes_items(id));
        assert!(reactors.respawn(now + Duration::from_secs(3600)).is_empty());
    }

    /// The Forest of Poison Haze's pond as its NX has it: it takes one
    /// Concentrated Poison, then two seconds on goes back through its own
    /// event 0 (type 101); and its spine, which takes four Diluted Poisons.
    fn pond_and_spine() -> [(MapReactorTemplate, ReactorTemplate); 2] {
        let takes = |item_id, next| ReactorEvent {
            kind: ITEM_DROPPED,
            next: Some(next),
            item: Some((item_id, 1)),
            area: Some(((-100, -50), (100, 30))),
        };
        let spot = |reactor_id, x, name: &str| MapReactorTemplate {
            reactor_id,
            x,
            y: 188,
            flip: false,
            reactor_time: 0,
            name: name.into(),
        };
        let pond = ReactorTemplate {
            reactor_id: 3002000,
            states: vec![
                ReactorState {
                    events: vec![takes(4001161, 1)],
                    ..ReactorState::default()
                },
                ReactorState {
                    events: vec![ReactorEvent {
                        kind: 101,
                        next: Some(0),
                        ..ReactorEvent::default()
                    }],
                    time_out: Some(2000),
                },
            ],
            action: Some("pFItem0".into()),
        };
        let spine = ReactorTemplate {
            reactor_id: net::reactors::ELLIN_SPINE,
            states: (1..=4)
                .map(|next| ReactorState {
                    events: vec![takes(4001162, next)],
                    ..ReactorState::default()
                })
                .chain([ReactorState::default()])
                .collect(),
            action: None,
        };
        [
            (spot(3002000, 1057, ""), pond),
            (spot(net::reactors::ELLIN_SPINE, 2331, "spine"), spine),
        ]
    }

    #[test]
    fn the_pond_acts_on_its_poison_and_fills_again_on_its_own() {
        let mut reactors = ReactorSet::new(pond_and_spine());
        let now = Instant::now();
        let [pond, _] = reactors.respawn(now)[..] else {
            panic!("two reactors");
        };
        assert!(reactors.revert(now + Duration::from_secs(60)).is_empty());
        for round in 0..2u64 {
            let at = now + Duration::from_secs(10 * round);
            match reactors.take_item(pond, 4001161, 1, (1057, 180), at) {
                Some(Hit::Acted { view, event: 0 }) => {
                    assert_eq!((view.object_id, view.state), (pond, 1))
                }
                other => panic!("round {round}: {other:?}"),
            }
            // Full, it takes no more until it has gone back.
            assert!(!reactors.takes_items(pond));
            assert!(reactors.revert(at + Duration::from_millis(1999)).is_empty());
            let back = reactors.revert(at + Duration::from_millis(2000));
            assert_eq!(back.len(), 1);
            assert_eq!((back[0].object_id, back[0].state), (pond, 0));
            assert!(reactors.takes_items(pond));
            assert!(reactors.revert(at + Duration::from_secs(5)).is_empty());
        }
    }

    #[test]
    fn another_reactor_reaching_a_state_with_a_time_out_only_moves() {
        let [(mut spot, mut pond), _] = pond_and_spine();
        spot.reactor_id = 3002001;
        pond.reactor_id = 3002001;
        // Into the timed state through an event that leads on there: it
        // goes back on its own.
        pond.states[1].events.insert(
            0,
            ReactorEvent {
                kind: 0,
                next: Some(1),
                ..ReactorEvent::default()
            },
        );
        let mut reactors = ReactorSet::new([(spot.clone(), pond.clone())]);
        let now = Instant::now();
        let [id] = reactors.respawn(now)[..] else {
            panic!("one reactor");
        };
        match reactors.take_item(id, 4001161, 1, (1057, 180), now) {
            Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, 1),
            other => panic!("{other:?}"),
        }
        let back = reactors.revert(now + Duration::from_secs(2));
        assert_eq!(back.iter().map(|view| view.state).collect::<Vec<_>>(), [0]);

        // Into its end state (its own event 0 leads back): it neither acts
        // nor goes back, as in Cosmic.
        pond.states[1].events.remove(0);
        let mut reactors = ReactorSet::new([(spot, pond)]);
        let [id] = reactors.respawn(now)[..] else {
            panic!("one reactor");
        };
        match reactors.take_item(id, 4001161, 1, (1057, 180), now) {
            Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, 1),
            other => panic!("{other:?}"),
        }
        assert!(reactors.revert(now + Duration::from_secs(60)).is_empty());
    }

    #[test]
    fn the_spine_stays_and_acts_on_its_fourth_poison() {
        let mut reactors = ReactorSet::new(pond_and_spine());
        let now = Instant::now();
        let [_, spine] = reactors.respawn(now)[..] else {
            panic!("two reactors");
        };
        for state in 1..=3 {
            match reactors.take_item(spine, 4001162, 1, (2331, 180), now) {
                Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, state),
                other => panic!("{state}: {other:?}"),
            }
        }
        match reactors.take_item(spine, 4001162, 1, (2331, 180), now) {
            Some(Hit::Acted { view, event: 0 }) => assert_eq!(view.state, 4),
            other => panic!("{other:?}"),
        }
        assert_eq!(reactors.get(spine).map(|reactor| reactor.state), Some(4));
        assert!(!reactors.takes_items(spine));
        assert!(reactors.revert(now + Duration::from_secs(60)).is_empty());
    }

    #[test]
    fn the_music_box_takes_only_the_days_record() {
        let records = || ReactorState {
            events: (0..7)
                .map(|day| ReactorEvent {
                    kind: ITEM_DROPPED,
                    next: Some(day + 1),
                    item: Some((net::reactors::SUNDAY_RECORD + i32::from(day), 1)),
                    area: Some(((-52, -64), (40, 79))),
                })
                .collect(),
            ..ReactorState::default()
        };
        let music = (
            MapReactorTemplate {
                reactor_id: net::reactors::MUSIC,
                x: -1706,
                y: -240,
                flip: false,
                reactor_time: -1,
                name: net::reactors::MUSIC_BOX.into(),
            },
            ReactorTemplate {
                reactor_id: net::reactors::MUSIC,
                states: (0..8).map(|_| records()).collect(),
                action: None,
            },
        );
        let mut reactors = ReactorSet::new([music]);
        reactors.set_day(2);
        let now = Instant::now();
        let [id] = reactors.respawn(now)[..] else {
            panic!("one reactor");
        };
        // Monday's and Wednesday's records stay where they land.
        assert_eq!(reactors.taking(4001057, 1, (-1706, -240)), None);
        assert!(reactors
            .take_item(id, 4001059, 1, (-1706, -240), now)
            .is_none());
        match reactors.take_item(id, 4001058, 1, (-1706, -240), now) {
            Some(Hit::Moved { view, event: 2 }) => assert_eq!(view.state, 3),
            other => panic!("{other:?}"),
        }
        assert_eq!(reactors.named(net::reactors::MUSIC_BOX).unwrap().state, 3);
    }

    #[test]
    fn a_moonflower_stays_in_bloom() {
        let (mut template, mut data) = mound();
        template.reactor_id = 9108000;
        data.reactor_id = 9108000;
        data.states.remove(1);
        data.states[0].events[0].item = Some((4001095, 1));
        let mut reactors = ReactorSet::new([(template, data)]);
        let now = Instant::now();
        let id = reactors.respawn(now)[0];
        match reactors.take_item(id, 4001095, 1, (105, 150), now) {
            Some(Hit::Moved { view, event: 0 }) => assert_eq!(view.state, 1),
            other => panic!("{other:?}"),
        }
        assert_eq!(reactors.get(id).unwrap().state, 1);
        assert!(!reactors.takes_items(id));
    }
}
