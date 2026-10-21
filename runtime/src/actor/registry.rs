use crate::actor::field::{FieldActor, FieldMapEntityNpc};
use crate::actor::field_drops::DropSet;
use crate::actor::field_mobs::{full_spawn, mob_capacity, MobSet};
use crate::actor::field_reactors::ReactorSet;
use crate::message::{FieldHandle, FieldKey, FieldMessage};
use crate::supervisor::spawn_supervised;
use std::collections::{HashMap, HashSet};
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
    /// Events' copies of their maps (channel, instance id) still running.
    instances: Mutex<HashSet<(u8, u32)>>,
    tick: Duration,
}

impl FieldRegistry {
    pub fn new(tick: Duration) -> Arc<Self> {
        Arc::new(Self {
            fields: Mutex::new(HashMap::new()),
            instances: Mutex::new(HashSet::new()),
            tick,
        })
    }

    /// The handle for `key`, starting its actor if needed. A field of an
    /// event's copy that has closed is started for the caller alone, and
    /// stops when they leave it.
    pub fn field(self: &Arc<Self>, key: FieldKey) -> FieldHandle {
        let closed = key.instance_id != 0
            && !self
                .instances
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains(&(key.channel_id, key.instance_id));
        let mut fields = self.fields.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = fields.get(&key) {
            if !handle.tx.is_closed() {
                return handle.clone();
            }
        }

        let (tx, rx) = mpsc::channel(FIELD_QUEUE_CAPACITY);
        let handle = FieldHandle { key, tx };
        let actor = FieldActor::new(
            key,
            rx,
            load_field_map_npcs(key),
            load_field_mobs(key),
            load_field_drops(key),
            load_field_reactors(key),
            self.tick,
        );
        let registry: Weak<Self> = Arc::downgrade(self);
        let exited = handle.clone();
        spawn_supervised(format!("field {key:?}"), actor.run(), move || {
            if let Some(registry) = registry.upgrade() {
                registry.forget(&exited);
            }
        });
        if !closed {
            fields.insert(key, handle.clone());
        }
        handle
    }

    /// An event's copy of its maps opens: its fields start as members
    /// arrive.
    pub fn open_instance(&self, channel_id: u8, instance_id: u32) {
        self.instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((channel_id, instance_id));
    }

    /// The event is over: its copy's fields stop, and anyone still on one
    /// finds it gone.
    pub fn close_instance(&self, channel_id: u8, instance_id: u32) {
        self.instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(channel_id, instance_id));
        let mut fields = self.fields.lock().unwrap_or_else(|e| e.into_inner());
        fields.retain(|key, handle| {
            let closing = (key.channel_id, key.instance_id) == (channel_id, instance_id);
            if closing {
                let _ = handle.tx.try_send(FieldMessage::Close);
            }
            !closing
        });
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

fn load_field_mobs(field_key: FieldKey) -> MobSet {
    let Some(field) = net::get_game_data()
        .ok()
        .and_then(|data| data.field(field_key.map_id))
    else {
        return MobSet::default();
    };
    let mobs = MobSet::new(field.mob_spawns.iter().filter_map(|spawn| {
        match net::mobs::info(spawn.mob_id) {
            Some(info) => Some((spawn.clone(), info)),
            None => {
                warn!(field = ?field_key, mob_id = spawn.mob_id, "Unknown mob in spawn list");
                None
            }
        }
    }))
    .with_capacity(mob_capacity(field.footholds.bounds(), field.mob_rate));
    if full_spawn() {
        mobs.filling_every_point()
    } else {
        mobs
    }
}

fn load_field_reactors(field_key: FieldKey) -> ReactorSet {
    let Some(field) = net::get_game_data()
        .ok()
        .and_then(|data| data.field(field_key.map_id))
    else {
        return ReactorSet::default();
    };
    ReactorSet::new(field.reactors.iter().filter_map(|reactor| {
        match net::reactors::template(reactor.reactor_id) {
            Some(template) => Some((reactor.clone(), template)),
            None => {
                warn!(field = ?field_key, reactor_id = reactor.reactor_id, "Unknown reactor on map");
                None
            }
        }
    }))
}

fn load_field_drops(field_key: FieldKey) -> DropSet {
    let field = net::get_game_data()
        .ok()
        .and_then(|data| data.field(field_key.map_id));
    let drops = DropSet::new(
        field
            .map(|field| field.footholds.clone())
            .unwrap_or_default(),
    );
    match field {
        Some(field) if field.everlast => drops.everlast(),
        _ => drops,
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
