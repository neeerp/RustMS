use memmap2::Mmap;
use nx_pkg4::NxFile;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
pub mod commodities;
pub mod footholds;
pub mod forbidden_names;
pub mod items;
pub mod lua_strings;
pub mod make_char_info;
pub mod mobs;
pub mod quests;
pub mod reactors;
pub mod server_content;
pub mod skills;

const NODE_SIZE_BYTES: u64 = 20;
pub const NO_DESTINATION_MAP: i32 = 999_999_999;
pub const PORTAL_TYPE_START_POINT: i32 = 0;
/// `pt` of a town's door portals (`tp`), where Mystic Doors stand.
pub const PORTAL_TYPE_TOWN_DOOR: i32 = 6;
/// `info/fieldLimit` bit forbidding movement skills; teleport rocks can't
/// be used there either (v83 FUN_00a0a4aa).
pub const FIELD_LIMIT_MOVEMENT_SKILLS: i32 = 0x2;
/// `info/fieldLimit` bit forbidding Mystic Door (v83 FUN_00969bf9).
pub const FIELD_LIMIT_MYSTIC_DOOR: i32 = 0x8;
/// `info/fieldLimit` bit forbidding every return scroll (v83 FUN_00a1e1ce).
pub const FIELD_LIMIT_RETURN_SCROLLS: i32 = 0x20;
/// `info/fieldLimit` bit keeping teleport rocks from the map, to or from it
/// (v83 FUN_00a0a4aa).
pub const FIELD_LIMIT_TELEPORT_ROCK: i32 = 0x40;
/// `info/fieldLimit` bit forbidding return scrolls that name a map (v83
/// FUN_00a1e1ce).
pub const FIELD_LIMIT_MAP_SCROLLS: i32 = 0x100;
/// `info/fieldLimit` bit leaving only the map's `info/allowedItem` usable
/// (v83 FUN_00a092fb).
pub const FIELD_LIMIT_CONSUMABLES: i32 = 0x400;
/// `info/fieldLimit` bit keeping items and mesos off the ground (v83
/// FUN_00a0900a, FUN_00a23de5).
pub const FIELD_LIMIT_DROPS: i32 = 0x400000;

#[derive(Debug, Error)]
pub enum GameDataError {
    #[error("i/o error")]
    Io(#[from] std::io::Error),
    #[error("nx read error")]
    Nx(#[from] nx_pkg4::NxError),
    #[error("utf-8 error")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("invalid integer cast")]
    InvalidCast(#[from] std::array::TryFromSliceError),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

#[derive(Clone, Debug)]
pub struct PortalTemplate {
    pub id: u32,
    pub name: String,
    pub portal_type: i32,
    pub x: i32,
    pub y: i32,
    pub to_map: Option<i32>,
    pub to_name: String,
    pub script: Option<String>,
}

/// A mob spawn point from a map's `life` (type `m`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapMobTemplate {
    pub mob_id: i32,
    /// Spawn position: `x` and the foothold height `cy`.
    pub x: i16,
    pub y: i16,
    pub foothold: i16,
    pub flip: bool,
    /// Seconds before the mob respawns; negative means never.
    pub mob_time: i32,
    pub hide: bool,
}

/// A reactor (a breakable box and the like) from a map's `reactor` nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapReactorTemplate {
    pub reactor_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    /// Seconds before a broken reactor comes back; negative means never.
    pub reactor_time: i32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapNpcTemplate {
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub foothold: i16,
    pub flip: bool,
    pub rx0: i16,
    pub rx1: i16,
}

/// Asset-backed normal speech. Quest scripts can select a different dialogue
/// definition later; quest completion is never inferred from reading these pages.
#[derive(Clone, Debug)]
pub struct NpcDialogueTemplate {
    pub npc_id: i32,
    pub pages: Vec<String>,
}

/// What a storage NPC (one with `info/trunkPut`) charges, in mesos.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StorageFees {
    /// To store an item (`trunkPut`).
    pub put: i32,
    /// To take one out (`trunkGet`).
    pub get: i32,
}

#[derive(Debug, Default)]
pub struct NpcDialogueData {
    dialogues: BTreeMap<i32, NpcDialogueTemplate>,
    storages: BTreeMap<i32, StorageFees>,
}

impl NpcDialogueData {
    pub fn load_from_nx(
        npc_path: impl AsRef<Path>,
        string_path: impl AsRef<Path>,
    ) -> Result<Self, GameDataError> {
        let npcs = NxDataFile::open(npc_path.as_ref())?;
        let strings = NxDataFile::open(string_path.as_ref())?;
        let names = strings.child_by_name(0, "Npc.img")?.ok_or_else(|| {
            GameDataError::InvalidData("missing root child 'Npc.img'".to_string())
        })?;
        let mut dialogues = BTreeMap::new();
        let mut storages = BTreeMap::new();

        for npc_index in npcs.child_indices(0)? {
            let name = npcs.node_name(npc_index)?;
            let Some(npc_id) = name
                .strip_suffix(".img")
                .and_then(|id| id.parse::<i32>().ok())
            else {
                continue;
            };
            let Some(info) = npcs.child_by_name(npc_index, "info")? else {
                continue;
            };
            if let Some(put) = npcs.int_child(info, "trunkPut")? {
                let get = npcs.int_child(info, "trunkGet")?.unwrap_or(0);
                storages.insert(npc_id, StorageFees { put, get });
            }
            let Some(speak) = npcs.child_by_name(info, "speak")? else {
                continue;
            };
            let Some(texts) = strings.child_by_name(names, &npc_id.to_string())? else {
                continue;
            };
            let mut keys = Vec::new();
            for index in npcs.child_indices(speak)? {
                if let Ok(order) = npcs.node_name(index)?.parse::<u32>() {
                    keys.push((order, npcs.node_name(index)?));
                }
            }
            keys.sort_by_key(|(order, _)| *order);
            let mut pages = Vec::new();
            for (_, key) in keys {
                if let Some(text_key) = npcs.string_child(speak, &key)? {
                    if let Some(text) = strings.string_child(texts, &text_key)? {
                        if !text.trim().is_empty() {
                            pages.push(text);
                        }
                    }
                }
            }
            if !pages.is_empty() {
                dialogues.insert(npc_id, NpcDialogueTemplate { npc_id, pages });
            }
        }
        Ok(Self {
            dialogues,
            storages,
        })
    }

    pub fn dialogue(&self, npc_id: i32) -> Option<&NpcDialogueTemplate> {
        self.dialogues.get(&npc_id)
    }

    /// The fees if `npc_id` keeps a storage.
    pub fn storage_fees(&self, npc_id: i32) -> Option<StorageFees> {
        self.storages.get(&npc_id).copied()
    }
}

/// Whether the v83 client sends `item_id` as USE_ITEM (FUN_004efd25):
/// potions, food, cures, transformations and the like. Return scrolls,
/// summon bags and the rest have opcodes of their own.
pub fn is_use_item(item_id: i32) -> bool {
    matches!(
        item_id / 10_000,
        200 | 201 | 202 | 205 | 221 | 236 | 238 | 245
    )
}

#[derive(Clone, Debug)]
pub struct FieldTemplate {
    pub map_id: i32,
    pub return_map: Option<i32>,
    pub forced_return: Option<i32>,
    /// `info/onUserEnter`: the script run whenever a player enters the map.
    pub on_user_enter: Option<String>,
    pub map_npcs: Vec<MapNpcTemplate>,
    pub mob_spawns: Vec<MapMobTemplate>,
    pub reactors: Vec<MapReactorTemplate>,
    pub mob_rate: f64,
    /// `info/recovery`: how much faster players recover HP and MP at rest,
    /// e.g. 2 in the Sleepywood sauna.
    pub recovery: f64,
    /// `info/town`.
    pub town: bool,
    /// `info/fieldLimit`: what the map forbids (see `FIELD_LIMIT_*`).
    pub field_limit: i32,
    /// `info/allowedItem`: the items `FIELD_LIMIT_CONSUMABLES` still allows.
    pub allowed_items: Vec<i32>,
    /// `info/personalShop`: players may open personal shops here (the Free
    /// Market rooms).
    pub personal_shop: bool,
    /// Whether the map has a wall clock (a `clock` node with a position and
    /// size, as stations do), which the server sets to the time of day.
    pub clock: bool,
    /// `info/timeLimit`: how many seconds a player may stay before being
    /// sent to `forced_return`. The client never reads it; the server shows
    /// the countdown.
    pub time_limit: Option<u32>,
    /// `info/decHP`: the cold, water or curse that hurts players here.
    pub hazard: Option<Hazard>,
    pub footholds: footholds::Footholds,
    portals_by_id: BTreeMap<u32, PortalTemplate>,
    portal_ids_by_name: HashMap<String, Vec<u32>>,
}

/// How often a map's hazard hurts when it has no `info/decInterval`. A
/// v83 Aqua Road item speaks of "the constant HP damage you receive for every
/// 10 seconds underwater" (Cassandra's Magic, 2022187).
pub const HAZARD_INTERVAL: Duration = Duration::from_secs(10);

/// A map's hazard. v83's client reads none of it; the server hurts the
/// player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hazard {
    /// The HP it takes each time (`decHP`).
    pub hp: i32,
    /// How often (`decInterval`, in ms, else `HAZARD_INTERVAL`).
    pub every: Duration,
    /// The equipment that keeps it off (`protectItem`).
    pub protect_item: Option<i32>,
}

impl FieldTemplate {
    /// Whether a player may use `item_id` here. With
    /// `FIELD_LIMIT_CONSUMABLES`, the potions, food, cures and the like that
    /// v83 sends as USE_ITEM work only if `info/allowedItem` lists them.
    pub fn may_use(&self, item_id: i32) -> bool {
        !is_use_item(item_id)
            || self.field_limit & FIELD_LIMIT_CONSUMABLES == 0
            || self.allowed_items.contains(&item_id)
    }

    /// Whether players may drop items and mesos here.
    pub fn may_drop(&self) -> bool {
        self.field_limit & FIELD_LIMIT_DROPS == 0
    }

    pub fn portals(&self) -> impl Iterator<Item = &PortalTemplate> {
        self.portals_by_id.values()
    }

    pub fn portal_by_id(&self, id: u32) -> Option<&PortalTemplate> {
        self.portals_by_id.get(&id)
    }

    pub fn portal_by_name(&self, name: &str) -> Option<&PortalTemplate> {
        let ids = self.portal_ids_by_name.get(name)?;
        let id = ids.first()?;
        self.portals_by_id.get(id)
    }

    /// The player spawn point nearest `(x, y)`: a start or invisible portal
    /// that leads nowhere. Saved on logout so the player returns near where
    /// they stood.
    pub fn closest_spawn_portal(&self, x: i32, y: i32) -> Option<&PortalTemplate> {
        self.portals_by_id
            .values()
            .filter(|portal| (PORTAL_TYPE_START_POINT..=1).contains(&portal.portal_type))
            .filter(|portal| portal.to_map.is_none())
            .min_by_key(|portal| {
                let (dx, dy) = (i64::from(portal.x - x), i64::from(portal.y - y));
                dx * dx + dy * dy
            })
    }

    /// The town's door portals, in the order the client numbers them: the
    /// door of the party member in slot `i` (or of a player alone, 0) stands
    /// at the `i`th.
    pub fn door_portals(&self) -> impl Iterator<Item = &PortalTemplate> {
        self.portals_by_id
            .values()
            .filter(|portal| portal.portal_type == PORTAL_TYPE_TOWN_DOOR)
    }

    pub fn resolve_spawn_portal(&self, preferred_name: &str) -> Option<&PortalTemplate> {
        if !preferred_name.trim().is_empty() {
            // A named link is an exact connection, not a hint. Falling back
            // hides broken/stale asset links and lands players elsewhere.
            return self.portal_by_name(preferred_name);
        }

        if let Some(start) = self
            .portals_by_id
            .values()
            .find(|portal| portal.portal_type == PORTAL_TYPE_START_POINT)
        {
            return Some(start);
        }

        self.portals_by_id
            .get(&0)
            .or_else(|| self.portals_by_id.values().next())
    }
}

#[derive(Debug, Default)]
pub struct GameData {
    fields: BTreeMap<i32, FieldTemplate>,
    /// `Map/AreaCode.img`: the area of each `map_id / 10000000`.
    area_codes: HashMap<i32, i32>,
}

impl GameData {
    pub fn load_from_nx_map(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let map_root = nx
            .child_by_name(0, "Map")?
            .ok_or_else(|| GameDataError::InvalidData("missing root child 'Map'".to_string()))?;

        let mut fields = BTreeMap::new();

        for map_group_idx in nx.child_indices(map_root)? {
            let group_name = nx.node_name(map_group_idx)?;
            if !group_name.starts_with("Map") {
                continue;
            }

            for map_node_idx in nx.child_indices(map_group_idx)? {
                let map_node_name = nx.node_name(map_node_idx)?;
                let Some(map_id) = parse_map_id(map_node_name.as_str()) else {
                    continue;
                };

                let field = build_field_template(&nx, map_node_idx, map_id)?;
                fields.insert(map_id, field);
            }
        }

        let mut area_codes = HashMap::new();
        if let Some(codes) = nx.child_by_name(map_root, "AreaCode.img")? {
            for code in nx.child_indices(codes)? {
                let name = nx.node_name(code)?;
                let Ok(prefix) = name.parse::<i32>() else {
                    continue;
                };
                if let Some(area) = nx.int_child(codes, &name)? {
                    area_codes.insert(prefix, area);
                }
            }
        }

        Ok(Self { fields, area_codes })
    }

    /// Whether a return scroll may take a player from `from` to `to` (v83
    /// FUN_00a26227): both maps' `map_id / 10000000` fall in the same area of
    /// `Map/AreaCode.img`. Event maps (`(id / 1000000) % 100 == 9`) and the
    /// ships (20009xxxx) are in none.
    pub fn same_area(&self, from: i32, to: i32) -> bool {
        let outside = |map_id: i32| (map_id / 1_000_000) % 100 == 9 || map_id / 10_000 == 20_009;
        if outside(from) || outside(to) {
            return false;
        }
        let area = |map_id: i32| self.area_codes.get(&(map_id / 10_000_000));
        area(from).is_some_and(|area_from| area(to) == Some(area_from))
    }

    pub fn field(&self, map_id: i32) -> Option<&FieldTemplate> {
        self.fields.get(&map_id)
    }

    pub fn field_exists(&self, map_id: i32) -> bool {
        self.fields.contains_key(&map_id)
    }
}

fn build_field_template(
    nx: &NxDataFile,
    map_node_idx: u32,
    map_id: i32,
) -> Result<FieldTemplate, GameDataError> {
    let (return_map, forced_return, on_user_enter) =
        if let Some(info_idx) = nx.child_by_name(map_node_idx, "info")? {
            (
                normalize_optional_map_id(nx.int_child(info_idx, "returnMap")?),
                normalize_optional_map_id(nx.int_child(info_idx, "forcedReturn")?),
                normalize_optional_string(nx.string_child(info_idx, "onUserEnter")?),
            )
        } else {
            (None, None, None)
        };

    let map_npcs = build_map_npcs_from_life(nx, map_node_idx)?;
    let mob_spawns = build_mob_spawns_from_life(nx, map_node_idx)?;
    let reactors = build_reactors(nx, map_node_idx)?;
    let footholds = build_footholds(nx, map_node_idx)?;
    let mob_rate = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.real_child(info, "mobRate"))
        .transpose()?
        .flatten()
        .unwrap_or(1.0);
    let recovery = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.real_child(info, "recovery"))
        .transpose()?
        .flatten()
        .unwrap_or(1.0);
    let time_limit = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.int_child(info, "timeLimit"))
        .transpose()?
        .flatten()
        .and_then(|seconds| u32::try_from(seconds).ok())
        .filter(|&seconds| seconds > 0);
    let (town, field_limit, allowed_items, personal_shop, hazard) =
        match nx.child_by_name(map_node_idx, "info")? {
            Some(info) => (
                nx.int_child(info, "town")?.unwrap_or(0) != 0,
                nx.int_child(info, "fieldLimit")?.unwrap_or(0),
                allowed_items(nx, info)?,
                nx.int_child(info, "personalShop")?.unwrap_or(0) != 0,
                hazard(nx, info)?,
            ),
            None => (false, 0, Vec::new(), false, None),
        };
    // v83 FUN_0052bc72 makes the clock only if all four are non-zero.
    let clock = match nx.child_by_name(map_node_idx, "clock")? {
        Some(clock) => {
            let mut all_set = true;
            for key in ["x", "y", "width", "height"] {
                all_set &= nx.int_child(clock, key)?.unwrap_or(0) != 0;
            }
            all_set
        }
        None => false,
    };

    let mut portals_by_id = BTreeMap::new();

    if let Some(portal_root_idx) = nx.child_by_name(map_node_idx, "portal")? {
        for portal_node_idx in nx.child_indices(portal_root_idx)? {
            let portal_id_name = nx.node_name(portal_node_idx)?;
            let Ok(portal_id) = portal_id_name.parse::<u32>() else {
                continue;
            };

            let name = nx
                .string_child(portal_node_idx, "pn")?
                .ok_or_else(|| GameDataError::InvalidData("missing portal pn".to_string()))?;
            let portal_type = nx.int_child(portal_node_idx, "pt")?.unwrap_or(0);
            let x = nx.int_child(portal_node_idx, "x")?.unwrap_or(0);
            let y = nx.int_child(portal_node_idx, "y")?.unwrap_or(0);
            let to_map = normalize_optional_map_id(nx.int_child(portal_node_idx, "tm")?);
            let to_name = normalize_string(
                nx.string_child(portal_node_idx, "tn")?
                    .unwrap_or_default()
                    .as_str(),
            );
            let script = normalize_optional_string(nx.string_child(portal_node_idx, "script")?);

            portals_by_id.insert(
                portal_id,
                PortalTemplate {
                    id: portal_id,
                    name,
                    portal_type,
                    x,
                    y,
                    to_map,
                    to_name,
                    script,
                },
            );
        }
    }

    let mut portal_ids_by_name: HashMap<String, Vec<u32>> = HashMap::new();
    for (id, portal) in &portals_by_id {
        portal_ids_by_name
            .entry(portal.name.clone())
            .or_default()
            .push(*id);
    }

    Ok(FieldTemplate {
        map_id,
        return_map,
        forced_return,
        on_user_enter,
        map_npcs,
        mob_spawns,
        reactors,
        mob_rate,
        recovery,
        town,
        field_limit,
        allowed_items,
        personal_shop,
        clock,
        time_limit,
        hazard,
        footholds,
        portals_by_id,
        portal_ids_by_name,
    })
}

/// A map's `info/decHP`, if positive, with its interval and protection.
fn hazard(nx: &NxDataFile, info: u32) -> Result<Option<Hazard>, GameDataError> {
    let hp = nx.int_child(info, "decHP")?.unwrap_or(0);
    if hp <= 0 {
        return Ok(None);
    }
    let every = nx
        .int_child(info, "decInterval")?
        .and_then(|ms| u64::try_from(ms).ok())
        .filter(|&ms| ms > 0)
        .map_or(HAZARD_INTERVAL, Duration::from_millis);
    let protect_item = nx.int_child(info, "protectItem")?.filter(|&id| id > 0);
    Ok(Some(Hazard {
        hp,
        every,
        protect_item,
    }))
}

/// A map's `info/allowedItem` list.
fn allowed_items(nx: &NxDataFile, info: u32) -> Result<Vec<i32>, GameDataError> {
    let Some(list) = nx.child_by_name(info, "allowedItem")? else {
        return Ok(Vec::new());
    };
    let mut items = Vec::new();
    for child in nx.child_indices(list)? {
        if let Some(item) = nx.int_child(list, &nx.node_name(child)?)? {
            items.push(item);
        }
    }
    Ok(items)
}

fn build_footholds(
    nx: &NxDataFile,
    map_node_idx: u32,
) -> Result<footholds::Footholds, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "foothold")? else {
        return Ok(footholds::Footholds::default());
    };
    let mut lines = Vec::new();
    for layer in nx.child_indices(root)? {
        for group in nx.child_indices(layer)? {
            for node in nx.child_indices(group)? {
                lines.push(footholds::Foothold {
                    x1: read_i16_child(nx, node, "x1", 0)?,
                    y1: read_i16_child(nx, node, "y1", 0)?,
                    x2: read_i16_child(nx, node, "x2", 0)?,
                    y2: read_i16_child(nx, node, "y2", 0)?,
                });
            }
        }
    }
    Ok(footholds::Footholds::new(lines))
}

fn build_reactors(
    nx: &NxDataFile,
    map_node_idx: u32,
) -> Result<Vec<MapReactorTemplate>, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "reactor")? else {
        return Ok(Vec::new());
    };
    let mut reactors = Vec::new();
    for node in nx.child_indices(root)? {
        reactors.push(MapReactorTemplate {
            reactor_id: parse_string_id(nx.string_child(node, "id")?, "reactor id")?,
            x: read_i16_child(nx, node, "x", 0)?,
            y: read_i16_child(nx, node, "y", 0)?,
            flip: nx.int_child(node, "f")?.unwrap_or(0) != 0,
            reactor_time: nx.int_child(node, "reactorTime")?.unwrap_or(0),
            name: nx.string_child(node, "name")?.unwrap_or_default(),
        });
    }
    Ok(reactors)
}

fn build_mob_spawns_from_life(
    nx: &NxDataFile,
    map_node_idx: u32,
) -> Result<Vec<MapMobTemplate>, GameDataError> {
    let Some(life_root_idx) = nx.child_by_name(map_node_idx, "life")? else {
        return Ok(Vec::new());
    };
    let mut spawns = Vec::new();
    for life in nx.child_indices(life_root_idx)? {
        if nx.string_child(life, "type")?.as_deref() != Some("m") {
            continue;
        }
        let x = read_i16_child(nx, life, "x", 0)?;
        let y = read_i16_child(nx, life, "y", 0)?;
        spawns.push(MapMobTemplate {
            mob_id: parse_string_id(nx.string_child(life, "id")?, "mob id")?,
            x,
            y: read_i16_child(nx, life, "cy", i32::from(y))?,
            foothold: read_i16_child(nx, life, "fh", 0)?,
            flip: nx.int_child(life, "f")?.unwrap_or(0) != 0,
            mob_time: nx.int_child(life, "mobTime")?.unwrap_or(0),
            hide: nx.int_child(life, "hide")?.unwrap_or(0) != 0,
        });
    }
    Ok(spawns)
}

fn build_map_npcs_from_life(
    nx: &NxDataFile,
    map_node_idx: u32,
) -> Result<Vec<MapNpcTemplate>, GameDataError> {
    let Some(life_root_idx) = nx.child_by_name(map_node_idx, "life")? else {
        return Ok(Vec::new());
    };

    let mut map_npcs = Vec::new();

    for life_node_idx in nx.child_indices(life_root_idx)? {
        let Some(life_type) = nx.string_child(life_node_idx, "type")? else {
            continue;
        };

        if life_type != "n" {
            continue;
        }

        let npc_id = parse_string_id(nx.string_child(life_node_idx, "id")?, "npc id")?;
        let x = read_i16_child(nx, life_node_idx, "x", 0)?;
        let y = read_i16_child(nx, life_node_idx, "y", 0)?;
        let foothold = read_i16_child(nx, life_node_idx, "fh", 0)?;
        let face_left = nx.int_child(life_node_idx, "f")?.unwrap_or(0) != 0;
        let rx0 = read_i16_child(nx, life_node_idx, "rx0", x as i32)?;
        let rx1 = read_i16_child(nx, life_node_idx, "rx1", x as i32)?;

        map_npcs.push(MapNpcTemplate {
            npc_id,
            x,
            y,
            foothold,
            flip: !face_left,
            rx0,
            rx1,
        });
    }

    Ok(map_npcs)
}

fn parse_map_id(map_node_name: &str) -> Option<i32> {
    map_node_name
        .strip_suffix(".img")
        .and_then(|id| id.parse::<i32>().ok())
}

fn normalize_optional_map_id(value: Option<i32>) -> Option<i32> {
    match value {
        Some(NO_DESTINATION_MAP) | None => None,
        Some(other) => Some(other),
    }
}

fn parse_string_id(value: Option<String>, field_name: &str) -> Result<i32, GameDataError> {
    let value = value.ok_or_else(|| GameDataError::InvalidData(format!("missing {field_name}")))?;
    value
        .parse::<i32>()
        .map_err(|_| GameDataError::InvalidData(format!("invalid {field_name}: '{value}'")))
}

fn read_i16_child(
    nx: &NxDataFile,
    parent_index: u32,
    key: &str,
    default: i32,
) -> Result<i16, GameDataError> {
    let value = nx.int_child(parent_index, key)?.unwrap_or(default);
    i16::try_from(value)
        .map_err(|_| GameDataError::InvalidData(format!("{key} out of i16 range: {value}")))
}

fn normalize_string(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        value.to_string()
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}

#[derive(Clone, Copy, Debug)]
struct NxNodeData {
    name_index: u32,
    children_index: u32,
    child_count: u16,
    data_type: u16,
    data: u64,
}

#[derive(Debug)]
struct NxDataFile {
    mmap: Mmap,
    node_count: u32,
    node_offset: u64,
    string_count: u32,
    string_offset: u64,
}

impl NxDataFile {
    fn open(path: &Path) -> Result<Self, GameDataError> {
        NxFile::open(path)?;

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let magic = read_u32(&mmap, 0)?;
        if magic != 0x3447_4B50 {
            return Err(GameDataError::InvalidData(
                "invalid NX header magic".to_string(),
            ));
        }

        let node_count = read_u32(&mmap, 4)?;
        let node_offset = read_u64(&mmap, 8)?;
        let string_count = read_u32(&mmap, 16)?;
        let string_offset = read_u64(&mmap, 20)?;

        Ok(Self {
            mmap,
            node_count,
            node_offset,
            string_count,
            string_offset,
        })
    }

    fn node_at(&self, index: u32) -> Result<NxNodeData, GameDataError> {
        if index >= self.node_count {
            return Err(GameDataError::InvalidData(format!(
                "node index {index} out of bounds for node_count={}",
                self.node_count
            )));
        }

        let offset = self.node_offset + index as u64 * NODE_SIZE_BYTES;
        Ok(NxNodeData {
            name_index: read_u32(&self.mmap, offset)?,
            children_index: read_u32(&self.mmap, offset + 4)?,
            child_count: read_u16(&self.mmap, offset + 8)?,
            data_type: read_u16(&self.mmap, offset + 10)?,
            data: read_u64(&self.mmap, offset + 12)?,
        })
    }

    fn node_name(&self, index: u32) -> Result<String, GameDataError> {
        let node = self.node_at(index)?;
        self.string_at(node.name_index)
    }

    fn string_at(&self, index: u32) -> Result<String, GameDataError> {
        if index >= self.string_count {
            return Err(GameDataError::InvalidData(format!(
                "string index {index} out of bounds for string_count={}",
                self.string_count
            )));
        }

        let pointer_offset = self.string_offset + index as u64 * 8;
        let string_data_offset = read_u64(&self.mmap, pointer_offset)?;
        let len = read_u16(&self.mmap, string_data_offset)? as usize;
        let start = (string_data_offset + 2) as usize;
        let end = start + len;

        let bytes = self.mmap.get(start..end).ok_or_else(|| {
            GameDataError::InvalidData(format!("string range {start}..{end} out of bounds"))
        })?;

        let value = std::str::from_utf8(bytes)?;
        Ok(value.to_owned())
    }

    fn child_by_name(
        &self,
        parent_index: u32,
        child_name: &str,
    ) -> Result<Option<u32>, GameDataError> {
        let parent = self.node_at(parent_index)?;
        let mut index = parent.children_index;
        let mut count = parent.child_count as u32;

        while count > 0 {
            let middle = count / 2;
            let current_index = index + middle;
            let current_name = self.node_name(current_index)?;

            match current_name.as_str().cmp(child_name) {
                Ordering::Less => {
                    index = current_index + 1;
                    count -= middle + 1;
                }
                Ordering::Equal => return Ok(Some(current_index)),
                Ordering::Greater => count = middle,
            }
        }

        Ok(None)
    }

    fn child_indices(&self, parent_index: u32) -> Result<Vec<u32>, GameDataError> {
        let parent = self.node_at(parent_index)?;
        let start = parent.children_index;
        let end = start + parent.child_count as u32;
        Ok((start..end).collect())
    }

    fn int_child(&self, parent_index: u32, key: &str) -> Result<Option<i32>, GameDataError> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };

        let node = self.node_at(child_index)?;
        let signed = match node.data_type {
            1 => i64::from_le_bytes(node.data.to_le_bytes()),
            3 => match self.numeric_string(node)? {
                Some(value) if (i32::MIN as f64..=i32::MAX as f64).contains(&value) => value as i64,
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        i32::try_from(signed).map(Some).map_err(|_| {
            GameDataError::InvalidData(format!("integer '{key}' out of range: {signed}"))
        })
    }

    /// A vector child's x and y.
    fn vector_child(
        &self,
        parent_index: u32,
        key: &str,
    ) -> Result<Option<(i32, i32)>, GameDataError> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };
        let node = self.node_at(child_index)?;
        if node.data_type != 4 {
            return Ok(None);
        }
        let x = node.data as u32 as i32;
        let y = (node.data >> 32) as u32 as i32;
        Ok(Some((x, y)))
    }

    /// A real (or integer) child as `f64`.
    fn real_child(&self, parent_index: u32, key: &str) -> Result<Option<f64>, GameDataError> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };
        let node = self.node_at(child_index)?;
        Ok(match node.data_type {
            1 => Some(i64::from_le_bytes(node.data.to_le_bytes()) as f64),
            2 => Some(f64::from_bits(node.data)),
            3 => self.numeric_string(node)?,
            _ => None,
        })
    }

    /// Some numbers are stored as strings (Mob.nx has maxHP "1000000"); the
    /// client reads them as numbers, so we do too.
    fn numeric_string(&self, node: NxNodeData) -> Result<Option<f64>, GameDataError> {
        let string_index = u32::try_from(node.data)
            .map_err(|_| GameDataError::InvalidData("string index out of range".to_string()))?;
        let value = self.string_at(string_index)?;
        let value = value.trim();
        Ok(value
            .parse::<i64>()
            .map(|v| v as f64)
            .or_else(|_| value.parse::<f64>())
            .ok()
            .filter(|v| v.is_finite()))
    }

    fn string_child(&self, parent_index: u32, key: &str) -> Result<Option<String>, GameDataError> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };

        let node = self.node_at(child_index)?;
        if node.data_type != 3 {
            return Ok(None);
        }

        let string_index = u32::try_from(node.data)
            .map_err(|_| GameDataError::InvalidData("string index out of range".to_string()))?;
        Ok(Some(self.string_at(string_index)?))
    }
}

fn read_u16(bytes: &[u8], offset: u64) -> Result<u16, GameDataError> {
    let offset = offset as usize;
    let data = bytes.get(offset..offset + 2).ok_or_else(|| {
        GameDataError::InvalidData(format!("u16 out of bounds at offset {offset}"))
    })?;
    Ok(u16::from_le_bytes(data.try_into()?))
}

fn read_u32(bytes: &[u8], offset: u64) -> Result<u32, GameDataError> {
    let offset = offset as usize;
    let data = bytes.get(offset..offset + 4).ok_or_else(|| {
        GameDataError::InvalidData(format!("u32 out of bounds at offset {offset}"))
    })?;
    Ok(u32::from_le_bytes(data.try_into()?))
}

fn read_u64(bytes: &[u8], offset: u64) -> Result<u64, GameDataError> {
    let offset = offset as usize;
    let data = bytes.get(offset..offset + 8).ok_or_else(|| {
        GameDataError::InvalidData(format!("u64 out of bounds at offset {offset}"))
    })?;
    Ok(u64::from_le_bytes(data.try_into()?))
}

/// Locating the (untracked) NX assets from tests.
#[cfg(test)]
pub(crate) mod test_assets {
    use std::path::{Path, PathBuf};

    /// Path to `assets/game-data/<name>`, or `None` (with a note) when the
    /// file is absent, so asset-backed tests skip instead of failing on a
    /// fresh clone. With `RUSTMS_REQUIRE_ASSETS=1` a missing file fails the
    /// test instead, so an asset run cannot pass vacuously. See
    /// assets/game-data/README.md.
    pub fn path(name: &str) -> Option<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/game-data")
            .join(name);
        if path.exists() {
            return Some(path);
        }
        if std::env::var("RUSTMS_REQUIRE_ASSETS").is_ok_and(|value| value == "1") {
            panic!(
                "RUSTMS_REQUIRE_ASSETS=1 but {} is not present",
                path.display()
            );
        }
        eprintln!("skipping: {} is not present", path.display());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_npc_speech_resolves_string_asset_keys() {
        let (Some(npc), Some(string)) =
            (test_assets::path("Npc.nx"), test_assets::path("String.nx"))
        else {
            return;
        };
        let data = NpcDialogueData::load_from_nx(npc, string).expect("load NPC dialogue assets");
        let rain = data.dialogue(12101).expect("Rain normal speech");
        assert_eq!(rain.pages.len(), 3);
        assert!(rain.pages[0].contains("today"));
        assert!(rain.pages[1].contains("Alt"));
        assert!(rain.pages[2].contains("arrow keys"));
        assert!(data.dialogue(-1).is_none());
    }

    #[test]
    fn storage_npcs_charge_their_trunk_fees() {
        let (Some(npc), Some(string)) =
            (test_assets::path("Npc.nx"), test_assets::path("String.nx"))
        else {
            return;
        };
        let data = NpcDialogueData::load_from_nx(npc, string).expect("load NPC assets");
        let fees = |put, get| Some(StorageFees { put, get });
        assert_eq!(data.storage_fees(1012009), fees(100, 0), "Mr. Lee");
        assert_eq!(data.storage_fees(2020004), fees(150, 0), "Mr. Mohammed");
        assert_eq!(data.storage_fees(9030100), fees(500, 1000), "Scrooge");
        assert_eq!(data.storage_fees(12101), None, "Rain keeps no storage");
    }

    #[test]
    fn resolve_spawn_portal_requires_named_links_and_defaults_only_when_unnamed() {
        let field = FieldTemplate {
            map_id: 1,
            return_map: None,
            forced_return: None,
            on_user_enter: None,
            map_npcs: Vec::new(),
            mob_spawns: Vec::new(),
            reactors: Vec::new(),
            mob_rate: 1.0,
            recovery: 1.0,
            town: false,
            field_limit: 0,
            allowed_items: Vec::new(),
            personal_shop: false,
            clock: false,
            time_limit: None,
            hazard: None,
            footholds: footholds::Footholds::default(),
            portals_by_id: BTreeMap::from([
                (
                    0,
                    PortalTemplate {
                        id: 0,
                        name: "sp".to_string(),
                        portal_type: PORTAL_TYPE_START_POINT,
                        x: 0,
                        y: 0,
                        to_map: None,
                        to_name: String::new(),
                        script: None,
                    },
                ),
                (
                    5,
                    PortalTemplate {
                        id: 5,
                        name: "in01".to_string(),
                        portal_type: 2,
                        x: 10,
                        y: 20,
                        to_map: Some(2),
                        to_name: "out01".to_string(),
                        script: None,
                    },
                ),
            ]),
            portal_ids_by_name: HashMap::from([
                ("sp".to_string(), vec![0]),
                ("in01".to_string(), vec![5]),
            ]),
        };

        assert_eq!(
            field.resolve_spawn_portal("in01").expect("named portal").id,
            5
        );
        assert!(field.resolve_spawn_portal("missing").is_none());
        assert_eq!(field.resolve_spawn_portal("").unwrap().id, 0);
    }

    #[test]
    fn closest_spawn_portal_skips_portals_that_lead_elsewhere() {
        let portal = |id, portal_type, x, to_map| PortalTemplate {
            id,
            name: format!("p{id}"),
            portal_type,
            x,
            y: 100,
            to_map,
            to_name: String::new(),
            script: None,
        };
        let field = FieldTemplate {
            map_id: 1,
            return_map: None,
            forced_return: None,
            on_user_enter: None,
            map_npcs: Vec::new(),
            mob_spawns: Vec::new(),
            mob_rate: 1.0,
            recovery: 1.0,
            town: false,
            field_limit: 0,
            allowed_items: Vec::new(),
            personal_shop: false,
            clock: false,
            time_limit: None,
            hazard: None,
            reactors: Vec::new(),
            footholds: footholds::Footholds::default(),
            portals_by_id: BTreeMap::from([
                (0, portal(0, PORTAL_TYPE_START_POINT, -500, None)),
                (1, portal(1, 0, 300, None)),
                (2, portal(2, 2, 900, Some(2))),
                (3, portal(3, 1, 700, None)),
            ]),
            portal_ids_by_name: HashMap::new(),
        };

        let closest = |x| field.closest_spawn_portal(x, 100).map(|p| p.id);
        assert_eq!(closest(-400), Some(0));
        assert_eq!(closest(250), Some(1));
        assert_eq!(closest(950), Some(3));
    }

    #[test]
    fn reads_what_maps_do_to_items() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = |id| game_data.field(id).expect("field");

        let event = field(109090300);
        assert!(!event.may_use(2000000), "a Red Potion isn't listed");
        assert!(event.may_use(2022539), "a listed item works");
        assert!(event.may_use(2030000), "a Return Scroll isn't checked");
        assert!(event.may_drop());
        assert!(!field(910000001).may_drop(), "the Free Market rooms");
        let henesys = field(100000000);
        assert!(henesys.may_use(2000000) && henesys.may_drop());
    }

    #[test]
    fn loads_map_npcs_from_assets_map_nx() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(0).expect("field 0");

        assert!(field.map_npcs.iter().any(|npc| {
            npc.npc_id == 2101
                && npc.x == -46
                && npc.y == 280
                && npc.foothold == 45
                && npc.rx0 <= npc.x
                && npc.rx1 >= npc.x
        }));
    }

    #[test]
    fn loads_footholds_from_assets_map_nx() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(50_000).expect("field 50000");

        assert!(!field.footholds.is_empty());
        // Every snail spawn stands on a foothold.
        for spawn in &field.mob_spawns {
            assert_eq!(
                field.footholds.ground_below(spawn.x, spawn.y - 5),
                Some(spawn.y),
                "{spawn:?}"
            );
        }
    }

    #[test]
    fn return_scrolls_stay_in_the_area_code_area() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        // Henesys and Lith Harbor share Victoria Island; Orbis and El Nath
        // share Ossyria; Ludibrium is an area of its own.
        assert!(game_data.same_area(100_000_000, 104_000_000));
        assert!(game_data.same_area(200_000_000, 211_000_000));
        assert!(!game_data.same_area(100_000_000, 200_000_000));
        assert!(!game_data.same_area(200_000_000, 220_000_000));
        // Event maps and the ships belong to none.
        assert!(!game_data.same_area(109_010_000, 100_000_000));
        assert!(!game_data.same_area(200_090_000, 200_000_000));
    }

    #[test]
    fn towns_list_their_door_portals_in_id_order() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let henesys = game_data.field(100_000_000).expect("Henesys");
        let doors: Vec<_> = henesys.door_portals().map(|portal| portal.id).collect();

        assert!(henesys.town);
        assert_eq!(doors, (28..=33).collect::<Vec<_>>());
        assert_eq!(
            henesys.door_portals().next().map(|p| (p.x, p.y)),
            Some((2786, 333))
        );
        let field = game_data.field(101_010_000).expect("a field near Ellinia");
        assert!(!field.town);
        assert_eq!(field.return_map, Some(101_000_000));
    }

    #[test]
    fn stations_have_a_wall_clock() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let clock = |map_id| game_data.field(map_id).expect("map").clock;
        assert!(clock(101_000_300));
        assert!(clock(200_000_111));
        assert!(!clock(101_000_301));
        assert!(!clock(100_000_000));
    }

    #[test]
    fn timed_maps_have_a_time_limit() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(100_020_100).expect("a mini dungeon");
        assert_eq!(field.time_limit, Some(7200));
        assert_eq!(field.forced_return, Some(100_020_000));
        assert_eq!(
            game_data.field(100_000_000).expect("Henesys").time_limit,
            None
        );
    }

    #[test]
    fn hazardous_maps_hurt() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let hazard = |map_id| game_data.field(map_id).expect("a map").hazard;
        let cold = Hazard {
            hp: 10,
            every: HAZARD_INTERVAL,
            protect_item: Some(1_102_109),
        };
        assert_eq!(hazard(211_040_000), Some(cold));
        let balrogs_tomb = Hazard {
            hp: 50,
            every: Duration::from_secs(15),
            protect_item: None,
        };
        assert_eq!(hazard(105_100_300), Some(balrogs_tomb));
        assert_eq!(hazard(211_000_000), None);
    }
}
