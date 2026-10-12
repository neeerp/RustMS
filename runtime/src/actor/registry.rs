use crate::actor::field::{FieldActor, FieldMapEntityNpc};
use crate::message::{FieldHandle, FieldKey};
use crate::supervisor::spawn_supervised;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

const FIELD_QUEUE_CAPACITY: usize = 1024;
const MAP_NPC_OBJECT_ID_BASE: i32 = 1_000_000_000;

/// Live field actors keyed by channel, map, and instance. Client actors look
/// up their field here and then talk to it directly.
pub struct FieldRegistry {
    fields: Mutex<HashMap<FieldKey, FieldHandle>>,
    tick: Duration,
}

impl FieldRegistry {
    pub fn new(tick: Duration) -> Arc<Self> {
        Arc::new(Self {
            fields: Mutex::new(HashMap::new()),
            tick,
        })
    }

    /// The handle for `key`, starting its actor if needed.
    pub fn field(self: &Arc<Self>, key: FieldKey) -> FieldHandle {
        let mut fields = self.fields.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = fields.get(&key) {
            if !handle.tx.is_closed() {
                return handle.clone();
            }
        }

        let (tx, rx) = mpsc::channel(FIELD_QUEUE_CAPACITY);
        let handle = FieldHandle { key, tx };
        let actor = FieldActor::new(key, rx, load_field_map_npcs(key), self.tick);
        let registry: Weak<Self> = Arc::downgrade(self);
        let exited = handle.clone();
        spawn_supervised(format!("field {key:?}"), actor.run(), move || {
            if let Some(registry) = registry.upgrade() {
                registry.forget(&exited);
            }
        });
        fields.insert(key, handle.clone());
        handle
    }

    /// Drop a handle whose actor has exited, unless it was already replaced.
    fn forget(&self, exited: &FieldHandle) {
        let mut fields = self.fields.lock().unwrap_or_else(|e| e.into_inner());
        if fields
            .get(&exited.key)
            .is_some_and(|current| current.tx.same_channel(&exited.tx))
        {
            fields.remove(&exited.key);
        }
    }
}

fn load_field_map_npcs(field_key: FieldKey) -> Vec<FieldMapEntityNpc> {
    let Ok(game_data) = net::get_game_data() else {
        warn!(field = ?field_key, "Failed to load game data for field NPCs");
        return Vec::new();
    };
    let Some(field) = game_data.field(field_key.map_id) else {
        return Vec::new();
    };

    field
        .map_npcs
        .iter()
        .enumerate()
        .filter_map(|(index, npc)| {
            i32::try_from(index).ok().map(|offset| FieldMapEntityNpc {
                object_id: MAP_NPC_OBJECT_ID_BASE + offset,
                npc_id: npc.npc_id,
                x: npc.x,
                y: npc.y,
                flip: npc.flip,
                foothold: npc.foothold,
                rx0: npc.rx0,
                rx1: npc.rx1,
            })
        })
        .collect()
}
