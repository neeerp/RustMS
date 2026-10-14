//! Reactor templates from `Reactor.nx` (`<id>.img/<state>/event`), looked up
//! on demand.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

/// Event types an attack can trigger: a plain hit, and hits that only count
/// from one side.
const HIT_EVENT_TYPES: [i32; 3] = [0, 1, 2];

/// What hitting a reactor in one state does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorState {
    /// The state a hit moves it to, if an attack can hit it in this state.
    pub hit: Option<u8>,
    /// Only these skills trigger the hit (`activeSkillID`); empty means any
    /// attack.
    pub skills: Vec<i32>,
    /// The state has no events at all: the reactor is used up.
    pub last: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorTemplate {
    pub reactor_id: i32,
    /// Indexed by state.
    pub states: Vec<ReactorState>,
    /// The server script run when it breaks (`action`).
    pub action: Option<String>,
}

impl ReactorTemplate {
    /// The state a hit with `skill_id` (0 for a plain attack) moves `state`
    /// to.
    pub fn hit(&self, state: u8, skill_id: i32) -> Option<u8> {
        let current = self.states.get(usize::from(state))?;
        if !current.skills.is_empty() && !current.skills.contains(&skill_id) {
            return None;
        }
        current.hit
    }

    /// Whether `state` ends the reactor (it breaks on reaching it).
    pub fn is_last(&self, state: u8) -> bool {
        self.states
            .get(usize::from(state))
            .map_or(true, |state| state.last)
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
        // Some reactors reuse another reactor's states through info/link.
        if let Some(info) = nx.child_by_name(image, "info")? {
            if let Some(link) = nx.string_child(info, "link")? {
                if let Some(linked) = nx.child_by_name(0, &format!("{link}.img"))? {
                    image = linked;
                }
            }
        }
        let action = nx.string_child(image, "action")?;
        let mut states = Vec::new();
        while let Some(node) = nx.child_by_name(image, &states.len().to_string())? {
            let mut state = ReactorState {
                last: true,
                ..ReactorState::default()
            };
            if let Some(events) = nx.child_by_name(node, "event")? {
                for event in nx.child_indices(events)? {
                    let Some(kind) = nx.int_child(event, "type")? else {
                        continue;
                    };
                    state.last = false;
                    if state.hit.is_some() || !HIT_EVENT_TYPES.contains(&kind) {
                        continue;
                    }
                    let next = nx.int_child(event, "state")?.unwrap_or(0);
                    state.hit = u8::try_from(next).ok();
                    if let Some(skills) = nx.child_by_name(event, "activeSkillID")? {
                        for skill in nx.child_indices(skills)? {
                            let name = nx.node_name(skill)?;
                            if let Some(id) = nx.int_child(skills, &name)? {
                                state.skills.push(id);
                            }
                        }
                    }
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
            state = reactor.hit(state, 0).unwrap();
            hits += 1;
        }
        assert_eq!((hits, state), (4, 4));
        assert!(data.template(9999998).unwrap().is_none());
    }

    #[test]
    fn amherst_has_five_scrap_boxes() {
        let Some(path) = crate::test_assets::path("Map.nx") else {
            return;
        };
        let data = crate::GameData::load_from_nx_map(path).unwrap();
        let amherst = &data.field(1000000).unwrap().reactors;
        assert_eq!(amherst.len(), 5);
        assert!(amherst.iter().all(|r| r.reactor_id == 2001));
        assert_eq!(
            (amherst[0].x, amherst[0].y, amherst[0].reactor_time),
            (610, 254, 90)
        );
    }
}
