//! The kites flying on one field (report c014). Pure state; `FieldActor`
//! sends the packets.

use crate::actor::object_ids::{self, ObjectIds};
use net::packet::build::world::kite::KiteView;
use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};

/// How long a kite flies (Cosmic's `KITE_EXPIRE_TIME`; the v83 client
/// waits for the server to take it down).
pub const KITE_TIME: Duration = Duration::from_secs(60 * 60);

pub struct KiteSet {
    kites: BTreeMap<i32, (KiteView, Instant)>,
    object_ids: ObjectIds,
}

impl Default for KiteSet {
    fn default() -> Self {
        Self {
            kites: BTreeMap::new(),
            object_ids: ObjectIds::new(object_ids::KITES),
        }
    }
}

impl KiteSet {
    /// Fly a kite, filling in its object id, until `KITE_TIME` from `now`.
    pub fn add(&mut self, mut view: KiteView, now: Instant) -> &KiteView {
        view.object_id = self.object_ids.next();
        let object_id = view.object_id;
        self.kites.insert(object_id, (view, now + KITE_TIME));
        &self.kites[&object_id].0
    }

    pub fn views(&self) -> impl Iterator<Item = &KiteView> {
        self.kites.values().map(|(view, _)| view)
    }

    /// Take down the kites whose time is up; returns their object ids.
    pub fn expire(&mut self, now: Instant) -> Vec<i32> {
        let gone: Vec<i32> = self
            .kites
            .iter()
            .filter(|(_, (_, until))| *until <= now)
            .map(|(&object_id, _)| object_id)
            .collect();
        for object_id in &gone {
            self.kites.remove(object_id);
        }
        gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kite(name: &str) -> KiteView {
        KiteView {
            object_id: 0,
            item_id: 5080000,
            message: "hi\n\n".into(),
            name: name.into(),
            x: 10,
            y: 20,
        }
    }

    #[test]
    fn kites_fly_side_by_side_for_an_hour() {
        let mut set = KiteSet::default();
        let now = Instant::now();
        assert_eq!(set.add(kite("Al"), now).object_id, object_ids::KITES.start);
        let later = now + Duration::from_secs(60);
        assert_eq!(
            set.add(kite("Bo"), later).object_id,
            object_ids::KITES.start + 1
        );
        assert_eq!(set.views().count(), 2);
        assert!(set
            .expire(now + KITE_TIME - Duration::from_secs(1))
            .is_empty());
        assert_eq!(set.expire(now + KITE_TIME), [object_ids::KITES.start]);
        assert_eq!(
            set.views()
                .map(|view| view.name.as_str())
                .collect::<Vec<_>>(),
            ["Bo"]
        );
    }
}
