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
    /// reactor's position (`lt`, `rb`).
    pub area: Option<((i16, i16), (i16, i16))>,
}

/// The event kind of an item dropped on a reactor.
pub const ITEM_DROPPED: i32 = 100;

/// What can happen to a reactor in one state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorState {
    pub events: Vec<ReactorEvent>,
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

    /// Whether an item dropped on it moves it on from `state`.
    pub fn takes_items(&self, state: u8) -> bool {
        self.states
            .get(usize::from(state))
            .is_some_and(|state| state.events.iter().any(|event| event.kind == ITEM_DROPPED))
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
                while let Some(event) = nx.child_by_name(events, &state.events.len().to_string())? {
                    let kind = nx.int_child(event, "type")?.unwrap_or(0);
                    let mut item = None;
                    let mut area = None;
                    if kind == ITEM_DROPPED {
                        // Some item ids are strings.
                        item = nx.int_child(event, "0")?.zip(
                            nx.int_child(event, "1")?
                                .and_then(|count| i16::try_from(count).ok()),
                        );
                        let corner = |key| -> Result<Option<(i16, i16)>, GameDataError> {
                            Ok(nx.vector_child(event, key)?.and_then(|(x, y)| {
                                Some((i16::try_from(x).ok()?, i16::try_from(y).ok()?))
                            }))
                        };
                        area = corner("lt")?.zip(corner("rb")?);
                    }
                    state.events.push(ReactorEvent {
                        kind,
                        next: nx
                            .int_child(event, "state")?
                            .and_then(|next| u8::try_from(next).ok()),
                        item,
                        area,
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
                },
                ReactorState {
                    events: vec![event(1, 3)],
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
