//! Reactor templates from `Reactor.nx` (`<id>.img/<state>/event`), looked up
//! on demand.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

/// One way a reactor can leave a state (`<state>/event/<n>`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorEvent {
    /// 0 is any hit, 1 and 2 a hit facing right or left, 3 and 4 the same
    /// in the air, and 5 a skill. 100 and up are other triggers, such as an
    /// item dropped on it.
    pub kind: i32,
    /// The state it moves to.
    pub next: Option<u8>,
    /// For an item dropped on it (100): the item and how many, exactly.
    pub item: Option<(i32, i16)>,
    /// For an item dropped on it: where the item must land, from the
    /// reactor's position (`lt`, `rb`). For a skill (5): where the user
    /// must stand.
    pub area: Option<((i16, i16), (i16, i16))>,
    /// For a skill (5): the skills that set it off (`activeSkillID`).
    pub skills: Vec<i32>,
}

/// The event kind of an item dropped on a reactor.
pub const ITEM_DROPPED: i32 = 100;
/// The event kind of a skill used near a reactor.
pub const SKILL_USED: i32 = 5;

/// What can happen to a reactor in one state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorState {
    pub events: Vec<ReactorEvent>,
    /// How long it stays in this state before going back on its own, in
    /// milliseconds (`event/timeOut`), through its last event (type 101).
    pub time_out: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorTemplate {
    pub reactor_id: i32,
    /// Indexed by state.
    pub states: Vec<ReactorState>,
    /// The server script run when it breaks (`action`).
    pub action: Option<String>,
}

/// How well an event of `kind` fits a swing with `flags` (bit 0 facing
/// left, bit 1 on a foothold): 0 is best, `None` doesn't fit (v83
/// 0x736091). A hit from the side or in the air beats a plain one.
pub fn swing_rank(flags: i32, kind: i32) -> Option<u8> {
    let left = flags & 1 != 0;
    let grounded = flags & 2 != 0;
    match (kind, grounded) {
        (0, true) => Some(1),
        (0, false) => Some(2),
        (1, _) if !left => Some(u8::from(!grounded)),
        (2, _) if left => Some(u8::from(!grounded)),
        (3, false) if !left => Some(0),
        (4, false) if left => Some(0),
        _ => None,
    }
}

impl ReactorTemplate {
    /// The event a swing with `flags` triggers in `state`, as its index, and
    /// the state it moves to. As in v83, the first event of the best rank
    /// wins (0x7348a0).
    pub fn swing(&self, state: u8, flags: i32) -> Option<(u8, u8)> {
        let events = &self.states.get(usize::from(state))?.events;
        let mut best: Option<(u8, usize)> = None;
        for (index, event) in events.iter().enumerate() {
            let Some(rank) = swing_rank(flags, event.kind) else {
                continue;
            };
            if best.map_or(true, |(best, _)| rank < best) {
                best = Some((rank, index));
            }
            if rank == 0 {
                break;
            }
        }
        let (_, index) = best?;
        Some((u8::try_from(index).ok()?, events[index].next?))
    }

    /// The event an item dropped in `state` triggers, as its index, and the
    /// state it moves to: `item_id`, `quantity` of it, landing at `(dx, dy)`
    /// from the reactor.
    pub fn item_dropped(
        &self,
        state: u8,
        item_id: i32,
        quantity: i16,
        (dx, dy): (i16, i16),
    ) -> Option<(u8, u8)> {
        let events = &self.states.get(usize::from(state))?.events;
        let index = events.iter().position(|event| {
            event.kind == ITEM_DROPPED
                && event.item == Some((item_id, quantity))
                && event.area.is_some_and(|((left, top), (right, bottom))| {
                    (left..=right).contains(&dx) && (top..=bottom).contains(&dy)
                })
        })?;
        Some((u8::try_from(index).ok()?, events[index].next?))
    }

    /// The event a skill `skill_id` used in `state` triggers, as its index,
    /// the state it moves to, and where its user must stand, from the
    /// reactor's position (v83 0x735ab7: the first that lists the skill).
    pub fn skill_used(
        &self,
        state: u8,
        skill_id: i32,
    ) -> Option<(u8, u8, ((i16, i16), (i16, i16)))> {
        let events = &self.states.get(usize::from(state))?.events;
        let index = events
            .iter()
            .position(|event| event.kind == SKILL_USED && event.skills.contains(&skill_id))?;
        let event = &events[index];
        Some((u8::try_from(index).ok()?, event.next?, event.area?))
    }

    /// Whether an item dropped on it moves it on from `state`.
    pub fn takes_items(&self, state: u8) -> bool {
        self.states
            .get(usize::from(state))
            .is_some_and(|state| state.events.iter().any(|event| event.kind == ITEM_DROPPED))
    }

    /// How long it waits in `state` before it goes back on its own, and the
    /// state it goes to (Cosmic `getTimeoutState`: its last event's).
    pub fn time_out(&self, state: u8) -> Option<(u32, u8)> {
        let state = self.states.get(usize::from(state))?;
        Some((state.time_out?, state.events.last()?.next?))
    }

    /// Whether moving to `next` through event `event` acts as breaking does
    /// though the reactor stays (Cosmic `hitReactor`'s end state): `next`
    /// waits to go back on its own, and its own event of that index leads
    /// back, as the Forest of Poison Haze's pond does after it takes the
    /// poison.
    pub fn acts_in(&self, next: u8, event: u8) -> bool {
        self.time_out(next).is_some()
            && self.states[usize::from(next)]
                .events
                .get(usize::from(event))
                .is_none_or(|event| event.next.is_none_or(|after| after < next))
    }

    /// Whether `state` ends the reactor (it breaks on reaching it).
    pub fn is_last(&self, state: u8) -> bool {
        self.states
            .get(usize::from(state))
            .map_or(true, |state| state.events.is_empty())
    }
}

pub struct ReactorData {
    nx: NxDataFile,
}

impl ReactorData {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        Ok(Self {
            nx: NxDataFile::open(path.as_ref())?,
        })
    }

    pub fn template(&self, reactor_id: i32) -> Result<Option<ReactorTemplate>, GameDataError> {
        let nx = &self.nx;
        let Some(mut image) = nx.child_by_name(0, &format!("{reactor_id:07}.img"))? else {
            return Ok(None);
        };
        // A reactor that reuses another's states keeps its own action.
        let own_action = nx.string_child(image, "action")?;
        // Some reactors reuse another reactor's states through info/link.
        if let Some(info) = nx.child_by_name(image, "info")? {
            if let Some(link) = nx.string_child(info, "link")? {
                if let Some(linked) = nx.child_by_name(0, &format!("{link}.img"))? {
                    image = linked;
                }
            }
        }
        let action = match own_action {
            Some(action) => Some(action),
            None => nx.string_child(image, "action")?,
        };
        let mut states = Vec::new();
        while let Some(node) = nx.child_by_name(image, &states.len().to_string())? {
            let mut state = ReactorState::default();
            if let Some(events) = nx.child_by_name(node, "event")? {
                state.time_out = nx
                    .int_child(events, "timeOut")?
                    .and_then(|ms| u32::try_from(ms).ok());
                while let Some(event) = nx.child_by_name(events, &state.events.len().to_string())? {
                    let kind = nx.int_child(event, "type")?.unwrap_or(0);
                    let mut item = None;
                    let mut area = None;
                    let mut skills = Vec::new();
                    let corner = |key| -> Result<Option<(i16, i16)>, GameDataError> {
                        Ok(nx.vector_child(event, key)?.and_then(|(x, y)| {
                            Some((i16::try_from(x).ok()?, i16::try_from(y).ok()?))
                        }))
                    };
                    if kind == ITEM_DROPPED {
                        // Some item ids are strings.
                        item = nx.int_child(event, "0")?.zip(
                            nx.int_child(event, "1")?
                                .and_then(|count| i16::try_from(count).ok()),
                        );
                        area = corner("lt")?.zip(corner("rb")?);
                    }
                    if kind == SKILL_USED {
                        area = corner("lt")?.zip(corner("rb")?);
                        if let Some(list) = nx.child_by_name(event, "activeSkillID")? {
                            while let Some(skill) = nx.int_child(list, &skills.len().to_string())? {
                                skills.push(skill);
                            }
                        }
                    }
                    state.events.push(ReactorEvent {
                        kind,
                        next: nx
                            .int_child(event, "state")?
                            .and_then(|next| u8::try_from(next).ok()),
                        item,
                        area,
                        skills,
                    });
                }
            }
            states.push(state);
        }
        Ok(Some(ReactorTemplate {
            reactor_id,
            states,
            action,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amhersts_scrap_box_breaks_on_the_fourth_hit() {
        let Some(path) = crate::test_assets::path("Reactor.nx") else {
            return;
        };
        let data = ReactorData::load(path).unwrap();
        let reactor = data.template(2001).unwrap().unwrap();
        assert_eq!(reactor.action.as_deref(), Some("mBoxItem0"));
        let mut state = 0;
        let mut hits = 0;
        while !reactor.is_last(state) {
            state = reactor.swing(state, 2).unwrap().1;
            hits += 1;
        }
        assert_eq!((hits, state), (4, 4));
        assert!(data.template(9999998).unwrap().is_none());
    }

    #[test]
    fn crimsonwood_keep_s_sigils_light_for_their_job_s_skills() {
        let Some(path) = crate::test_assets::path("Reactor.nx") else {
            return;
        };
        let data = ReactorData::load(path).unwrap();
        let area = ((-125, -19), (123, 122));
        // The warriors' sigil of the second and third stages, lit from its
        // first state.
        let sigil = data.template(6109000).unwrap().unwrap();
        assert_eq!(sigil.action.as_deref(), Some("glpqskill0"));
        assert_eq!(sigil.skill_used(0, 1211002), Some((0, 1, area)));
        assert_eq!(sigil.skill_used(0, 1111008), Some((0, 1, area)));
        assert_eq!(sigil.skill_used(0, 3211006), None);
        assert!(sigil.swing(0, 2).is_none() && sigil.is_last(1));
        // The pirates', for Flash Fist among others.
        let pirates = data.template(6109004).unwrap().unwrap();
        assert!(pirates.skill_used(0, 5001001).is_some());
        // The fourth stage's: only once it is set to its first state.
        let fourth = data.template(6109016).unwrap().unwrap();
        assert_eq!(fourth.skill_used(0, 1211002), None);
        assert_eq!(fourth.skill_used(1, 1211002), Some((0, 2, area)));
    }

    #[test]
    fn the_resurrection_mound_takes_the_cold_flame_then_the_black_book() {
        let Some(path) = crate::test_assets::path("Reactor.nx") else {
            return;
        };
        let data = ReactorData::load(path).unwrap();
        let mound = data.template(2302003).unwrap().unwrap();
        assert!(mound.takes_items(0) && mound.takes_items(1) && !mound.takes_items(2));
        assert_eq!(mound.item_dropped(0, 4001108, 1, (0, -100)), Some((0, 1)));
        assert_eq!(mound.item_dropped(0, 4001108, 2, (0, -100)), None);
        assert_eq!(mound.item_dropped(0, 4001108, 1, (60, -100)), None);
        assert_eq!(mound.item_dropped(0, 4001107, 1, (0, -100)), None);
        assert_eq!(mound.item_dropped(1, 4001107, 1, (43, -69)), Some((0, 2)));
        assert!(mound.is_last(2));
        assert!(mound.swing(0, 2).is_none());

        // An egg takes its own egg, then three hits.
        let egg = data.template(2401001).unwrap().unwrap();
        assert_eq!(egg.item_dropped(0, 4001113, 1, (0, 0)), Some((0, 1)));
        assert_eq!(egg.item_dropped(0, 4001114, 1, (0, 0)), None);
        assert!(egg.swing(1, 2).is_some());

        // Linked reactors share states but keep their own action.
        let mob_box = data.template(1021000).unwrap().unwrap();
        assert_eq!(mob_box.action.as_deref(), Some("s4hitmanMob0"));
        assert_eq!(mob_box.states.len(), 5);
    }

    #[test]
    fn swing_ranks_follow_v83() {
        // Facing right and left, on a foothold and in the air.
        let ranks = |kind| [2, 3, 0, 1].map(|flags| swing_rank(flags, kind));
        assert_eq!(ranks(0), [Some(1), Some(1), Some(2), Some(2)]);
        assert_eq!(ranks(1), [Some(0), None, Some(1), None]);
        assert_eq!(ranks(2), [None, Some(0), None, Some(1)]);
        assert_eq!(ranks(3), [None, None, Some(0), None]);
        assert_eq!(ranks(4), [None, None, None, Some(0)]);
        assert_eq!(ranks(5), [None; 4]);
        assert_eq!(ranks(100), [None; 4]);
    }

    #[test]
    fn a_swing_takes_the_event_for_its_side() {
        let event = |kind, next| ReactorEvent {
            kind,
            next: Some(next),
            ..ReactorEvent::default()
        };
        let reactor = ReactorTemplate {
            states: vec![
                ReactorState {
                    events: vec![event(101, 9), event(0, 1), event(2, 2)],
                    ..ReactorState::default()
                },
                ReactorState {
                    events: vec![event(1, 3)],
                    ..ReactorState::default()
                },
            ],
            ..ReactorTemplate::default()
        };
        assert_eq!(reactor.swing(0, 2), Some((1, 1)));
        assert_eq!(reactor.swing(0, 3), Some((2, 2)));
        assert_eq!(reactor.swing(1, 3), None);
        assert_eq!(reactor.swing(2, 2), None);
    }

    #[test]
    fn the_forests_pond_acts_on_its_poison_and_fills_again_and_the_spine_takes_four() {
        let Some(path) = crate::test_assets::path("Reactor.nx") else {
            return;
        };
        let data = ReactorData::load(path).unwrap();
        let pond = data.template(3002000).unwrap().unwrap();
        assert_eq!(pond.item_dropped(0, 4001161, 1, (0, 0)), Some((0, 1)));
        assert_eq!(pond.time_out(1), Some((2000, 0)));
        assert!(pond.acts_in(1, 0) && !pond.is_last(1));
        assert_eq!(pond.time_out(0), None);

        let spine = data.template(3009000).unwrap().unwrap();
        let mut state = 0;
        while let Some((_, next)) = spine.item_dropped(state, 4001162, 1, (0, 0)) {
            assert!(!spine.acts_in(next, 0));
            state = next;
        }
        assert_eq!(state, 4);
        assert!(spine.is_last(4));
    }

    #[test]
    fn amherst_has_five_scrap_boxes() {
        let Some(path) = crate::test_assets::path("Map.nx") else {
            return;
        };
        let data = crate::GameData::load_from_nx_map(path).unwrap();
        let amherst = &data.field(1010000).unwrap().reactors;
        assert_eq!(amherst.len(), 5);
        assert!(amherst.iter().all(|r| r.reactor_id == 2000));
        assert_eq!(
            (amherst[0].x, amherst[0].y, amherst[0].reactor_time),
            (610, 259, 90)
        );
    }
}
