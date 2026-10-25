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
pub mod maker;
pub mod mobs;
pub mod quests;
pub mod reactors;
pub mod server_content;
pub mod skills;
pub mod vega_spell;

const NODE_SIZE_BYTES: u64 = 20;
pub const NO_DESTINATION_MAP: i32 = 999_999_999;
pub const PORTAL_TYPE_START_POINT: i32 = 0;
/// `pt` of a town's door portals (`tp`), where Mystic Doors stand.
pub const PORTAL_TYPE_TOWN_DOOR: i32 = 6;
/// `info/fieldType` of the Mu Lung Dojo's stages, where the Beginner's
/// secret skills work (v83 FUN_00587f5f); `FIELD_TYPE_DOJO_2` too.
pub const FIELD_TYPE_DOJO: i32 = 14;
pub const FIELD_TYPE_DOJO_2: i32 = 18;
/// `info/fieldType` of Nett's Pyramid, where Rage of Pharaoh works.
pub const FIELD_TYPE_PYRAMID: i32 = 23;
/// `info/fieldLimit` bit forbidding movement skills; teleport rocks can't
/// be used there either (v83 FUN_00a0a4aa).
pub const FIELD_LIMIT_MOVEMENT_SKILLS: i32 = 0x2;
/// `info/fieldLimit` bit forbidding summoning sacks unless
/// `info/allowedItem` lists them (v83 FUN_00a0977b).
pub const FIELD_LIMIT_SUMMON_SACKS: i32 = 0x4;
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
/// `info/fieldLimit` bit keeping mounts off the map (v83 FUN_00966f7a).
pub const FIELD_LIMIT_MOUNTS: i32 = 0x200;
/// `info/fieldLimit` bit leaving only the map's `info/allowedItem` usable
/// (v83 FUN_00a092fb).
pub const FIELD_LIMIT_CONSUMABLES: i32 = 0x400;
/// `info/fieldLimit` bit forbidding weather items (v83 FUN_00a0a63f, type
/// 0x10).
pub const FIELD_LIMIT_WEATHER_ITEMS: i32 = 0x4000;
/// `info/fieldLimit` bit keeping Miu Miu's merchant away (v83 FUN_00a0a63f,
/// type 0x25, and FUN_00a0ecab).
pub const FIELD_LIMIT_MIU_MIU: i32 = 0x40000;
/// `info/fieldLimit` bit sparing whoever dies there their EXP (Cosmic's
/// NO_EXP_DECREASE; v83 never reads it). The Dojo and most event maps.
pub const FIELD_LIMIT_NO_EXP_LOSS: i32 = 0x80000;
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
    /// `team`: on a Monster Carnival field, the side (0 red, 1 blue) whose
    /// mob it is.
    pub team: Option<u8>,
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

/// `mobMassacre`: the rules of a Nett's Pyramid stage (and of the Kerning
/// City event rooms 910320xxx and 910330xxx).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Massacre {
    /// `disableSkill`: no skills may be used on it (some of the event rooms).
    pub disable_skill: bool,
    /// `gauge/total`: how much the gauge holds.
    pub total: i32,
    /// `gauge/decrease`: how much it loses each second.
    pub decrease: i32,
    /// `gauge/hitAdd`: how much a kill puts back.
    pub hit_add: i32,
    /// `gauge/missSub`: how much a miss takes off.
    pub miss_sub: i32,
    /// `gauge/coolAdd`: how much a cool kill puts back.
    pub cool_add: i32,
    /// `countEffect`, by the kill count each is reached at, lowest first.
    pub count_effects: Vec<CountEffect>,
}

/// What a player gets on reaching a Nett's Pyramid kill count
/// (`mobMassacre/countEffect/<count>`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CountEffect {
    pub count: i32,
    /// `buff`: the item whose effect they take, or 0 for none.
    pub buff: i32,
    /// `skillUse`: a use of Rage of Pharaoh.
    pub skill_use: bool,
}

/// `monsterCarnival`: the rules of a Monster Carnival battle field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Carnival {
    /// `timeDefault`: the battle's length in seconds.
    pub time_default: i32,
    /// `timeExpand`: how many seconds a tie adds.
    pub time_expand: i32,
    /// `deathCP`: the most CP a player loses on dying.
    pub death_cp: i32,
    /// `mobGenMax`: how many mobs a side may summon, if set.
    pub mob_gen_max: Option<i32>,
    /// `guardianGenMax`: how many guardians a side may place, if set.
    pub guardian_gen_max: Option<i32>,
    /// `mapDivided`: the field is split in two halves, one for each side.
    pub map_divided: bool,
    /// `rewardMapWin` and `rewardMapLose`.
    pub reward_map_win: i32,
    pub reward_map_lose: i32,
    /// `effectWin`, `effectLose`, `soundWin` and `soundLose`.
    pub effect_win: String,
    pub effect_lose: String,
    pub sound_win: String,
    pub sound_lose: String,
    /// `mob`: the mobs that may be summoned, as (mob id, `spendCP`), in the
    /// order of the client's list.
    pub mobs: Vec<(i32, i32)>,
    /// `skill`: the MCSkill ids in the client's list.
    pub skills: Vec<i32>,
    /// `guardian`: the MCGuardian ids in the client's list.
    pub guardians: Vec<i32>,
    /// `guardianGenPos`: where guardians may stand.
    pub guardian_spots: Vec<CarnivalSpot>,
    /// `mobGenPos`: where summoned mobs may appear.
    pub mob_spots: Vec<CarnivalSpot>,
}

/// `coconut`: the rules of a Coconut Season field (field type 4, b167).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coconut {
    /// `countFalling`, `countBombing` and `countStopped`: how many nuts may
    /// fall, burst or stick. Their sum is the number of nuts the client
    /// knows (501).
    pub count_falling: u32,
    pub count_bombing: u32,
    pub count_stopped: u32,
    /// `timeDefault`: the game's length in seconds.
    pub time_default: u32,
    /// `timeExpand`: the seconds a tie adds, once.
    pub time_expand: u32,
    /// `timeFinish`: the seconds between the end and the warp out.
    pub time_finish: u32,
    /// `effectWin`, `effectLose`, `soundWin` and `soundLose`.
    pub effect_win: String,
    pub effect_lose: String,
    pub sound_win: String,
    pub sound_lose: String,
}

impl Coconut {
    /// How many nuts there are.
    pub fn nuts(&self) -> u32 {
        self.count_falling + self.count_bombing + self.count_stopped
    }
}

/// `snowBall`: the rules of a Snowball field (field type 1, b168). The
/// client reads the lanes and the clamps; the server reads the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snowball {
    /// `dx`: the pixels a ball rolls each step.
    pub dx: i32,
    /// `xMin` and `xMax`: the steps a ball may be at.
    pub x_min: i32,
    pub x_max: i32,
    /// `damageSnowBall`: what a hit on a ball shows.
    pub damage_ball: u16,
    /// `damageSnowMan0` and `damageSnowMan1`: a hit on a snowman, and a
    /// strong one.
    pub damage_snowman: u16,
    pub damage_snowman_strong: u16,
    /// `snowManHP`: a snowman's HP.
    pub snowman_hp: u32,
    /// `snowManWait`: how long, in ms, a snowman brought down stops the
    /// other team's ball.
    pub snowman_wait: u32,
    /// `speed` and `recoveryAmount`.
    pub speed: u32,
    pub recovery: u32,
    /// `section1` to `section3`: the steps whose passing is announced.
    pub sections: [i32; 3],
}

/// A spot in a Monster Carnival field: `x`, `y` (`cy` for mobs), `fh`,
/// `f`, and the side it is for (`team`), if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarnivalSpot {
    pub x: i16,
    pub y: i16,
    pub foothold: i16,
    pub flip: bool,
    pub team: Option<u8>,
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
    /// `info/fixedMobCapacity`: how many mobs the map holds, whoever is on
    /// it (Nett's Pyramid and the Kerning City event rooms, 365 maps).
    pub fixed_mob_capacity: Option<u32>,
    /// `info/createMobInterval`: how often, in milliseconds, the map makes
    /// mobs (the same maps).
    pub create_mob_interval: Option<u32>,
    /// `mobMassacre`, on the 50 maps that have it.
    pub massacre: Option<Massacre>,
    /// `monsterCarnival`, on the Monster Carnival battle fields.
    pub carnival: Option<Carnival>,
    /// `coconut`, on the Coconut Season fields.
    pub coconut: Option<Coconut>,
    /// `snowBall`, on the Snowball fields.
    pub snowball: Option<Snowball>,
    /// `info/VRLeft`, `VRTop`, `VRRight` and `VRBottom` when `info/VRLimit`
    /// is set (only Orbis Tower<B2>, 200082300): the view range that narrows
    /// the map's bounds (`Footholds::bounds_within`).
    pub view_limit: Option<(i32, i32, i32, i32)>,
    /// `info/lvLimit`: the level a player needs to walk in through a portal.
    /// Only the server reads it.
    pub level_limit: Option<u8>,
    /// `info/lvForceMove`: the level at which a player is moved out to
    /// `return_map` (applied on the Training Centers only, see
    /// `net::map_rules`). Only the server reads it.
    pub level_force_move: Option<u8>,
    /// `info/recovery`: how much faster players recover HP and MP at rest,
    /// e.g. 2 in the Sleepywood sauna.
    pub recovery: f64,
    /// `info/town`.
    pub town: bool,
    /// `info/fieldLimit`: what the map forbids (see `FIELD_LIMIT_*`).
    pub field_limit: i32,
    /// `info/fieldType`: the kind of event map it is, e.g.
    /// `FIELD_TYPE_DOJO` (0 for an ordinary map).
    pub field_type: i32,
    /// `info/allowedItem`: the items `FIELD_LIMIT_CONSUMABLES` still allows.
    pub allowed_items: Vec<i32>,
    /// `info/personalShop`: players may open personal shops here (the Free
    /// Market rooms).
    pub personal_shop: bool,
    /// `info/everlast`: drops never vanish here, and players may not throw
    /// items down (the PQ storage rooms).
    pub everlast: bool,
    /// Whether the map has a wall clock (a `clock` node with a position and
    /// size, as stations do), which the server sets to the time of day.
    pub clock: bool,
    /// `info/timeLimit`: how many seconds a player may stay before being
    /// sent to `forced_return`. The client never reads it; the server shows
    /// the countdown.
    pub time_limit: Option<u32>,
    /// `info/timeMob`: a mob that a kill here lets out, and the notice
    /// everyone here then reads (So Gong's trial, where he runs away).
    pub time_mob: Option<(i32, String)>,
    /// `info/decHP`: the cold, water or curse that hurts players here.
    pub hazard: Option<Hazard>,
    /// What the map's traps may do to a player who touches one.
    pub traps: Traps,
    pub footholds: footholds::Footholds,
    /// The `area` boxes, in their order in the map. The client ignores them;
    /// the server checks where players stand in them, as on Kerning City's
    /// party quest's ropes, platforms and barrels.
    pub areas: Vec<Area>,
    portals_by_id: BTreeMap<u32, PortalTemplate>,
    portal_ids_by_name: HashMap<String, Vec<u32>>,
}

/// One of a map's `area` boxes: from `x1` to `x2` and `y1` to `y2`, the
/// second edge left out of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
}

impl Area {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        (self.x1..self.x2).contains(&x) && (self.y1..self.y2).contains(&y)
    }
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
    /// Whether it is the water's (a `swim` map) rather than the cold's.
    pub water: bool,
    /// Whether it is the cold's, which an equip scrolled with ADD PREVENT
    /// COLDNESS keeps off: El Nath's and the internet cafe's maps (by
    /// `returnMap`), as Cosmic has it, and those the cold cape (1102109)
    /// protects.
    pub cold: bool,
}

/// What a map's traps, objs marked `obstacle`, may do to a player. The client
/// tests them and reports the hits (v83 FUN_00642710); this is what bounds
/// its reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Traps {
    /// The most any of them hurts (`damage`), before the client's roll of a
    /// tenth either way.
    pub damage: i32,
    /// Whether one leaves the player on `forceHP`, which can take all their
    /// HP.
    pub force_hp: bool,
    /// Where touching one sends a player (`targetField`), such as the Witch
    /// Tower's entrance.
    pub target_fields: Vec<i32>,
}

impl Traps {
    /// The most a hit the client reports may be: a trap's `damage` over 10
    /// rolls up to a tenth more (0x6427c8), and a `forceHP` trap takes up to
    /// all of `hp`.
    pub fn max_hit(&self, hp: i32) -> i32 {
        let rolled = if self.damage > 10 {
            self.damage + self.damage / 10
        } else {
            self.damage
        };
        if self.force_hp {
            rolled.max(hp)
        } else {
            rolled
        }
    }

    fn add(&mut self, trap: &Traps) {
        self.damage = self.damage.max(trap.damage);
        self.force_hp |= trap.force_hp;
        for &map_id in &trap.target_fields {
            if !self.target_fields.contains(&map_id) {
                self.target_fields.push(map_id);
            }
        }
    }
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

    /// Whether a player may open the summoning sack `item_id` here.
    pub fn may_summon(&self, item_id: i32) -> bool {
        self.field_limit & FIELD_LIMIT_SUMMON_SACKS == 0 || self.allowed_items.contains(&item_id)
    }

    /// Whether a player may use a weather item here. The client also keeps
    /// them out of the wedding halls (fieldType 60).
    pub fn may_blow_weather(&self) -> bool {
        self.field_limit & FIELD_LIMIT_WEATHER_ITEMS == 0
    }

    /// Whether Miu Miu's merchant comes here.
    pub fn may_call_merchant(&self) -> bool {
        self.field_limit & FIELD_LIMIT_MIU_MIU == 0
    }

    /// Whether players may drop items and mesos here.
    pub fn may_drop(&self) -> bool {
        self.field_limit & FIELD_LIMIT_DROPS == 0
    }

    /// Whether players may throw items down here. v83 refuses to drag an
    /// item onto an everlast map (FUN_004f31a5 reads `CField+0x164`).
    pub fn may_drop_items(&self) -> bool {
        self.may_drop() && !self.everlast
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
        let trap_objs = trap_objs(&nx)?;

        let mut map_nodes = BTreeMap::new();
        for map_group_idx in nx.child_indices(map_root)? {
            let group_name = nx.node_name(map_group_idx)?;
            if !group_name.starts_with("Map") {
                continue;
            }

            for map_node_idx in nx.child_indices(map_group_idx)? {
                let map_node_name = nx.node_name(map_node_idx)?;
                if let Some(map_id) = parse_map_id(map_node_name.as_str()) {
                    map_nodes.insert(map_id, map_node_idx);
                }
            }
        }

        for (&map_id, &map_node_idx) in &map_nodes {
            // A copy of a map (`info/link`, as the mini-dungeons' and the
            // Dojo's are) has only its own `info`; the rest is the linked
            // map's.
            let body_idx = match nx.child_by_name(map_node_idx, "info")? {
                Some(info) => nx
                    .string_child(info, "link")?
                    .and_then(|link| link.parse::<i32>().ok())
                    .and_then(|link| map_nodes.get(&link).copied())
                    .unwrap_or(map_node_idx),
                None => map_node_idx,
            };
            let field = build_field_template(&nx, map_node_idx, body_idx, map_id, &trap_objs)?;
            fields.insert(map_id, field);
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

    pub fn field_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.fields.keys().copied()
    }

    pub fn field_exists(&self, map_id: i32) -> bool {
        self.fields.contains_key(&map_id)
    }
}

/// The template of the map whose node is `map_node_idx`, with its `info`
/// from that node and the rest from `body_idx` (the node itself, or the map
/// it links to).
fn build_field_template(
    nx: &NxDataFile,
    map_node_idx: u32,
    body_idx: u32,
    map_id: i32,
    trap_objs: &TrapObjs,
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

    let map_npcs = build_map_npcs_from_life(nx, body_idx)?;
    let mob_spawns = build_mob_spawns_from_life(nx, body_idx)?;
    let reactors = build_reactors(nx, body_idx)?;
    let footholds = build_footholds(nx, body_idx)?;
    let traps = traps(nx, body_idx, trap_objs)?;
    let mob_rate = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.real_child(info, "mobRate"))
        .transpose()?
        .flatten()
        .unwrap_or(1.0);
    let positive_key = |key| -> Result<Option<u32>, GameDataError> {
        Ok(match nx.child_by_name(map_node_idx, "info")? {
            Some(info) => nx
                .int_child(info, key)?
                .and_then(|value| u32::try_from(value).ok())
                .filter(|&value| value > 0),
            None => None,
        })
    };
    let (fixed_mob_capacity, create_mob_interval) = (
        positive_key("fixedMobCapacity")?,
        positive_key("createMobInterval")?,
    );
    let massacre = build_massacre(nx, body_idx)?;
    let carnival = build_carnival(nx, body_idx)?;
    let coconut = build_coconut(nx, body_idx)?;
    let snowball = build_snowball(nx, body_idx)?;
    let recovery = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.real_child(info, "recovery"))
        .transpose()?
        .flatten()
        .unwrap_or(1.0);
    let view_limit = match nx.child_by_name(map_node_idx, "info")? {
        Some(info) if nx.int_child(info, "VRLimit")?.unwrap_or(0) != 0 => {
            let edge =
                |key| -> Result<i32, GameDataError> { Ok(nx.int_child(info, key)?.unwrap_or(0)) };
            Some((
                edge("VRLeft")?,
                edge("VRTop")?,
                edge("VRRight")?,
                edge("VRBottom")?,
            ))
        }
        _ => None,
    };
    let level_key = |key| -> Result<Option<u8>, GameDataError> {
        Ok(match nx.child_by_name(map_node_idx, "info")? {
            Some(info) => nx
                .int_child(info, key)?
                .and_then(|level| u8::try_from(level).ok())
                .filter(|&level| level > 0),
            None => None,
        })
    };
    let (level_limit, level_force_move) = (level_key("lvLimit")?, level_key("lvForceMove")?);
    let time_limit = nx
        .child_by_name(map_node_idx, "info")?
        .map(|info| nx.int_child(info, "timeLimit"))
        .transpose()?
        .flatten()
        .and_then(|seconds| u32::try_from(seconds).ok())
        .filter(|&seconds| seconds > 0);
    let time_mob = match nx.child_by_name(map_node_idx, "info")? {
        Some(info) => match nx.child_by_name(info, "timeMob")? {
            Some(time_mob) => nx
                .int_child(time_mob, "id")?
                .zip(nx.string_child(time_mob, "message")?),
            None => None,
        },
        None => None,
    };
    let (town, field_limit, field_type, allowed_items, personal_shop, everlast, hazard) =
        match nx.child_by_name(map_node_idx, "info")? {
            Some(info) => (
                nx.int_child(info, "town")?.unwrap_or(0) != 0,
                nx.int_child(info, "fieldLimit")?.unwrap_or(0),
                nx.int_child(info, "fieldType")?.unwrap_or(0),
                allowed_items(nx, info)?,
                nx.int_child(info, "personalShop")?.unwrap_or(0) != 0,
                nx.int_child(info, "everlast")?.unwrap_or(0) != 0,
                hazard(nx, info, return_map)?,
            ),
            None => (false, 0, 0, Vec::new(), false, false, None),
        };
    // v83 FUN_0052bc72 makes the clock only if all four are non-zero.
    let clock = match nx.child_by_name(body_idx, "clock")? {
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

    if let Some(portal_root_idx) = nx.child_by_name(body_idx, "portal")? {
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

    let mut areas = Vec::new();
    if let Some(area_root) = nx.child_by_name(body_idx, "area")? {
        for area in nx.child_indices(area_root)? {
            let edge =
                |key| -> Result<i32, GameDataError> { Ok(nx.int_child(area, key)?.unwrap_or(0)) };
            areas.push(Area {
                x1: edge("x1")?,
                x2: edge("x2")?,
                y1: edge("y1")?,
                y2: edge("y2")?,
            });
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
        fixed_mob_capacity,
        create_mob_interval,
        massacre,
        carnival,
        coconut,
        snowball,
        view_limit,
        level_limit,
        level_force_move,
        recovery,
        town,
        field_limit,
        field_type,
        allowed_items,
        personal_shop,
        everlast,
        clock,
        time_limit,
        time_mob,
        hazard,
        traps,
        footholds,
        areas,
        portals_by_id,
        portal_ids_by_name,
    })
}

/// The traps among `Map/Obj`'s objs, by `oS` and then `l0`, `l1` and `l2`.
type TrapObjs = HashMap<String, HashMap<(String, String, String), Traps>>;

/// The objs marked `obstacle` (v83 FUN_0063c741), whether on the obj's own
/// node or on one of its states (`s0`, `s1`, ...), which the server may show
/// in turn.
fn trap_objs(nx: &NxDataFile) -> Result<TrapObjs, GameDataError> {
    let mut traps = TrapObjs::new();
    let Some(obj_root) = nx.child_by_name(0, "Obj")? else {
        return Ok(traps);
    };
    for set in nx.child_indices(obj_root)? {
        let set_name = nx.node_name(set)?;
        let Some(o_s) = set_name.strip_suffix(".img") else {
            continue;
        };
        for l0 in nx.child_indices(set)? {
            for l1 in nx.child_indices(l0)? {
                for l2 in nx.child_indices(l1)? {
                    let mut obj = Traps::default();
                    let mut any = trap(nx, l2, &mut obj)?;
                    for state in nx.child_indices(l2)? {
                        let name = nx.node_name(state)?;
                        if name.len() > 1
                            && name.starts_with('s')
                            && name[1..].bytes().all(|b| b.is_ascii_digit())
                        {
                            any |= trap(nx, state, &mut obj)?;
                        }
                    }
                    if any {
                        traps.entry(o_s.to_string()).or_default().insert(
                            (nx.node_name(l0)?, nx.node_name(l1)?, nx.node_name(l2)?),
                            obj,
                        );
                    }
                }
            }
        }
    }
    Ok(traps)
}

/// Adds `node`'s trap to `traps`, if it is one.
fn trap(nx: &NxDataFile, node: u32, traps: &mut Traps) -> Result<bool, GameDataError> {
    if nx.int_child(node, "obstacle")?.unwrap_or(0) == 0 {
        return Ok(false);
    }
    traps.add(&Traps {
        damage: nx.int_child(node, "damage")?.unwrap_or(0),
        force_hp: nx.int_child(node, "forceHP")?.is_some_and(|hp| hp >= 0),
        target_fields: nx
            .int_child(node, "targetField")?
            .filter(|&map_id| map_id != 0)
            .into_iter()
            .collect(),
    });
    Ok(true)
}

/// The traps placed on the map's layers.
fn traps(nx: &NxDataFile, map_node_idx: u32, trap_objs: &TrapObjs) -> Result<Traps, GameDataError> {
    let mut traps = Traps::default();
    for layer in 0..8 {
        let Some(layer) = nx.child_by_name(map_node_idx, &layer.to_string())? else {
            continue;
        };
        let Some(objs) = nx.child_by_name(layer, "obj")? else {
            continue;
        };
        for obj in nx.child_indices(objs)? {
            let Some(set) = nx
                .string_child(obj, "oS")?
                .and_then(|o_s| trap_objs.get(&o_s))
            else {
                continue;
            };
            let key = (
                nx.string_child(obj, "l0")?.unwrap_or_default(),
                nx.string_child(obj, "l1")?.unwrap_or_default(),
                nx.string_child(obj, "l2")?.unwrap_or_default(),
            );
            if let Some(trap) = set.get(&key) {
                traps.add(trap);
            }
        }
    }
    Ok(traps)
}

/// A map's `info/decHP`, if positive, with its interval and protection.
fn hazard(
    nx: &NxDataFile,
    info: u32,
    return_map: Option<i32>,
) -> Result<Option<Hazard>, GameDataError> {
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
    let water = nx.int_child(info, "swim")?.unwrap_or(0) != 0;
    let cold = !water
        && (protect_item == Some(COLD_CAPE) || matches!(return_map, Some(EL_NATH | INTERNET_CAFE)));
    Ok(Some(Hazard {
        hp,
        every,
        protect_item,
        water,
        cold,
    }))
}

const COLD_CAPE: i32 = 1_102_109;
const EL_NATH: i32 = 211_000_000;
const INTERNET_CAFE: i32 = 193_000_000;

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

fn build_massacre(nx: &NxDataFile, map_node_idx: u32) -> Result<Option<Massacre>, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "mobMassacre")? else {
        return Ok(None);
    };
    let int =
        |node, key| -> Result<i32, GameDataError> { Ok(nx.int_child(node, key)?.unwrap_or(0)) };
    let mut massacre = Massacre {
        disable_skill: int(root, "disableSkill")? != 0,
        ..Massacre::default()
    };
    if let Some(gauge) = nx.child_by_name(root, "gauge")? {
        massacre.total = int(gauge, "total")?;
        massacre.decrease = int(gauge, "decrease")?;
        massacre.hit_add = int(gauge, "hitAdd")?;
        massacre.miss_sub = int(gauge, "missSub")?;
        massacre.cool_add = int(gauge, "coolAdd")?;
    }
    if let Some(effects) = nx.child_by_name(root, "countEffect")? {
        for node in nx.child_indices(effects)? {
            let Ok(count) = nx.node_name(node)?.parse() else {
                continue;
            };
            massacre.count_effects.push(CountEffect {
                count,
                buff: int(node, "buff")?,
                skill_use: int(node, "skillUse")? != 0,
            });
        }
        massacre.count_effects.sort_by_key(|effect| effect.count);
    }
    Ok(Some(massacre))
}

fn build_coconut(nx: &NxDataFile, map_node_idx: u32) -> Result<Option<Coconut>, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "coconut")? else {
        return Ok(None);
    };
    let count = |key| -> Result<u32, GameDataError> {
        Ok(nx
            .int_child(root, key)?
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0))
    };
    let string = |key| -> Result<String, GameDataError> {
        Ok(nx.string_child(root, key)?.unwrap_or_default())
    };
    Ok(Some(Coconut {
        count_falling: count("countFalling")?,
        count_bombing: count("countBombing")?,
        count_stopped: count("countStopped")?,
        time_default: count("timeDefault")?,
        time_expand: count("timeExpand")?,
        time_finish: count("timeFinish")?,
        effect_win: string("effectWin")?,
        effect_lose: string("effectLose")?,
        sound_win: string("soundWin")?,
        sound_lose: string("soundLose")?,
    }))
}

fn build_snowball(nx: &NxDataFile, map_node_idx: u32) -> Result<Option<Snowball>, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "snowBall")? else {
        return Ok(None);
    };
    let int = |key| -> Result<i32, GameDataError> { Ok(nx.int_child(root, key)?.unwrap_or(0)) };
    let amount = |key| -> Result<u32, GameDataError> { Ok(u32::try_from(int(key)?).unwrap_or(0)) };
    let damage = |key| -> Result<u16, GameDataError> { Ok(u16::try_from(int(key)?).unwrap_or(0)) };
    Ok(Some(Snowball {
        dx: int("dx")?,
        x_min: int("xMin")?,
        x_max: int("xMax")?,
        damage_ball: damage("damageSnowBall")?,
        damage_snowman: damage("damageSnowMan0")?,
        damage_snowman_strong: damage("damageSnowMan1")?,
        snowman_hp: amount("snowManHP")?,
        snowman_wait: amount("snowManWait")?,
        speed: amount("speed")?,
        recovery: amount("recoveryAmount")?,
        sections: [int("section1")?, int("section2")?, int("section3")?],
    }))
}

fn build_carnival(nx: &NxDataFile, map_node_idx: u32) -> Result<Option<Carnival>, GameDataError> {
    let Some(root) = nx.child_by_name(map_node_idx, "monsterCarnival")? else {
        return Ok(None);
    };
    let int = |key| -> Result<i32, GameDataError> { Ok(nx.int_child(root, key)?.unwrap_or(0)) };
    let string = |key| -> Result<String, GameDataError> {
        Ok(nx.string_child(root, key)?.unwrap_or_default())
    };
    // A list's entries are named 0, 1, 2 ..., which the file sorts as text
    // (10 before 2); the client takes them by number.
    let ordered = |list| -> Result<Vec<u32>, GameDataError> {
        let mut children = Vec::new();
        for child in nx.child_indices(list)? {
            if let Ok(order) = nx.node_name(child)?.parse::<u32>() {
                children.push((order, child));
            }
        }
        children.sort_by_key(|&(order, _)| order);
        Ok(children.into_iter().map(|(_, child)| child).collect())
    };
    let ints = |key| -> Result<Vec<i32>, GameDataError> {
        let Some(list) = nx.child_by_name(root, key)? else {
            return Ok(Vec::new());
        };
        let mut values = Vec::new();
        for child in ordered(list)? {
            if let Some(value) = nx.int_child(list, &nx.node_name(child)?)? {
                values.push(value);
            }
        }
        Ok(values)
    };
    let spots = |key, y_key| -> Result<Vec<CarnivalSpot>, GameDataError> {
        let Some(list) = nx.child_by_name(root, key)? else {
            return Ok(Vec::new());
        };
        let mut spots = Vec::new();
        for node in ordered(list)? {
            let y = read_i16_child(nx, node, "y", 0)?;
            spots.push(CarnivalSpot {
                x: read_i16_child(nx, node, "x", 0)?,
                y: read_i16_child(nx, node, y_key, i32::from(y))?,
                foothold: read_i16_child(nx, node, "fh", 0)?,
                flip: nx.int_child(node, "f")?.unwrap_or(0) != 0,
                team: nx
                    .int_child(node, "team")?
                    .and_then(|team| u8::try_from(team).ok()),
            });
        }
        Ok(spots)
    };
    let mut mobs = Vec::new();
    if let Some(list) = nx.child_by_name(root, "mob")? {
        for node in ordered(list)? {
            let id = nx.int_child(node, "id")?.unwrap_or(0);
            mobs.push((id, nx.int_child(node, "spendCP")?.unwrap_or(0)));
        }
    }
    Ok(Some(Carnival {
        time_default: int("timeDefault")?,
        time_expand: int("timeExpand")?,
        death_cp: int("deathCP")?,
        mob_gen_max: nx.int_child(root, "mobGenMax")?,
        guardian_gen_max: nx.int_child(root, "guardianGenMax")?,
        map_divided: int("mapDivided")? != 0,
        reward_map_win: int("rewardMapWin")?,
        reward_map_lose: int("rewardMapLose")?,
        effect_win: string("effectWin")?,
        effect_lose: string("effectLose")?,
        sound_win: string("soundWin")?,
        sound_lose: string("soundLose")?,
        mobs,
        skills: ints("skill")?,
        guardians: ints("guardian")?,
        guardian_spots: spots("guardianGenPos", "y")?,
        mob_spots: spots("mobGenPos", "cy")?,
    }))
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
                    id: nx.node_name(node)?.parse().unwrap_or(0),
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
            team: nx
                .int_child(life, "team")?
                .and_then(|team| u8::try_from(team).ok()),
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
            fixed_mob_capacity: None,
            create_mob_interval: None,
            massacre: None,
            carnival: None,
            coconut: None,
            snowball: None,
            view_limit: None,
            level_limit: None,
            level_force_move: None,
            recovery: 1.0,
            town: false,
            field_limit: 0,
            field_type: 0,
            allowed_items: Vec::new(),
            personal_shop: false,
            everlast: false,
            clock: false,
            time_limit: None,
            time_mob: None,
            hazard: None,
            traps: Traps::default(),
            footholds: footholds::Footholds::default(),
            areas: Vec::new(),
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
            fixed_mob_capacity: None,
            create_mob_interval: None,
            massacre: None,
            carnival: None,
            coconut: None,
            snowball: None,
            view_limit: None,
            level_limit: None,
            level_force_move: None,
            recovery: 1.0,
            town: false,
            field_limit: 0,
            field_type: 0,
            allowed_items: Vec::new(),
            personal_shop: false,
            everlast: false,
            clock: false,
            time_limit: None,
            time_mob: None,
            hazard: None,
            traps: Traps::default(),
            reactors: Vec::new(),
            footholds: footholds::Footholds::default(),
            areas: Vec::new(),
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
    fn level_limits_come_from_the_map_info() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = |map_id| game_data.field(map_id).expect("map");
        // Leafre's forest, past the town's west00.
        assert_eq!(field(240010000).level_limit, Some(15));
        assert_eq!(field(240000000).level_limit, None);
        let center = field(910060000);
        assert_eq!(center.level_force_move, Some(20));
        assert_eq!(center.level_limit, None);
        assert_eq!(field(103040100).level_force_move, Some(51));
    }

    #[test]
    fn orbis_towers_view_range_narrows_its_bounds() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let tower = game_data.field(200082300).expect("Orbis Tower<B2>");
        assert_eq!(tower.view_limit, Some((-400, -2239, 400, 285)));
        // Only the top moves in, to 65 px below the view's.
        assert_eq!(tower.footholds.bounds(), Some((-335, -2610, 335, 160)));
        assert_eq!(
            tower.footholds.bounds_within(tower.view_limit),
            Some((-335, -2174, 335, 160))
        );
        assert_eq!(
            game_data.field(100000000).expect("Henesys").view_limit,
            None
        );
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
    fn loads_kerning_party_quest_areas_from_assets_map_nx() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let areas = |map_id| game_data.field(map_id).expect("field").areas.len();

        // Its ropes, platforms and barrels.
        assert_eq!(
            [areas(103000801), areas(103000802), areas(103000803)],
            [4, 5, 6]
        );
        assert_eq!(areas(103000800), 0);
        let rope = game_data.field(103000801).unwrap().areas[0];
        assert_eq!(
            rope,
            Area {
                x1: -770,
                x2: -742,
                y1: -132,
                y2: 46
            }
        );
        assert!(rope.contains(-770, -132) && rope.contains(-743, 45));
        assert!(!rope.contains(-742, 0) && !rope.contains(-750, 46));
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
    fn coconut_fields_have_their_rules() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(109_080_000).expect("Coconut Season");
        assert_eq!(field.field_type, 4);
        let coconut = field.coconut.as_ref().expect("coconut");
        assert_eq!(
            (
                coconut.count_falling,
                coconut.count_bombing,
                coconut.count_stopped
            ),
            (401, 80, 20)
        );
        assert_eq!(coconut.nuts(), 501);
        assert_eq!(
            (
                coconut.time_default,
                coconut.time_expand,
                coconut.time_finish
            ),
            (300, 120, 12)
        );
        assert_eq!(coconut.effect_win, "event/coconut/victory");
        assert_eq!(coconut.sound_lose, "Coconut/Failed");
        let short = game_data.field(109_080_003).expect("the G variant");
        let short = short.coconut.as_ref().expect("coconut");
        assert_eq!((short.time_default, short.time_expand), (180, 60));
        assert!(game_data
            .field(100_000_000)
            .expect("Henesys")
            .coconut
            .is_none());
    }

    #[test]
    fn snowball_fields_have_their_rules() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(109_060_000).expect("Snowball");
        assert_eq!(field.field_type, 1);
        let snowball = field.snowball.as_ref().expect("snowBall");
        assert_eq!((snowball.dx, snowball.x_min, snowball.x_max), (3, -60, 900));
        assert_eq!(
            (
                snowball.damage_ball,
                snowball.damage_snowman,
                snowball.damage_snowman_strong
            ),
            (10, 15, 45)
        );
        assert_eq!((snowball.snowman_hp, snowball.snowman_wait), (7500, 10000));
        assert_eq!((snowball.speed, snowball.recovery), (150, 400));
        assert_eq!(snowball.sections, [45, 290, 560]);
        let other = game_data.field(109_060_002).expect("the second field");
        assert_eq!(other.snowball.as_ref().expect("snowBall").snowman_hp, 3000);
        // The entrances have none.
        assert!(game_data
            .field(109_060_001)
            .expect("the entrance")
            .snowball
            .is_none());
    }

    #[test]
    fn monster_carnival_fields_have_their_rules_and_sides() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data
            .field(980_000_101)
            .expect("Monster Carnival field 1");
        let carnival = field.carnival.as_ref().expect("monsterCarnival");
        assert_eq!((carnival.time_default, carnival.time_expand), (610, 120));
        assert_eq!(carnival.death_cp, 10);
        assert_eq!(
            (carnival.mob_gen_max, carnival.guardian_gen_max),
            (Some(10), Some(8))
        );
        assert!(!carnival.map_divided);
        assert_eq!(
            (carnival.reward_map_win, carnival.reward_map_lose),
            (980_000_103, 980_000_104)
        );
        assert_eq!(carnival.effect_win, "quest/carnival/win");
        assert_eq!(carnival.sound_lose, "MobCarnival/Lose");
        assert_eq!(carnival.mobs.len(), 10);
        assert_eq!(carnival.mobs[0], (9_300_127, 7));
        assert_eq!(carnival.mobs[9], (9_300_136, 30));
        assert_eq!(carnival.skills, [1, 2, 4, 5, 6, 0, 3, 7]);
        assert_eq!(carnival.guardians, (0..=8).collect::<Vec<_>>());
        assert_eq!(carnival.mob_spots.len(), 15);
        assert_eq!(carnival.mob_spots[0].x, -988);
        assert_eq!(carnival.mob_spots[0].y, 162);
        // The fifteenth spot is the last, not the third.
        assert_eq!(
            (carnival.mob_spots[14].x, carnival.mob_spots[2].x),
            (622, -777)
        );
        assert_eq!(carnival.guardian_spots[0].y, -135);
        // Its own mobs are on one side or the other.
        let teams: Vec<_> = field.mob_spawns.iter().map(|spawn| spawn.team).collect();
        assert!(teams.contains(&Some(0)) && teams.contains(&Some(1)));
        // The divided fields mark the sides of their guardians' spots.
        let divided = game_data.field(980_000_301).expect("field 3");
        let carnival = divided.carnival.as_ref().expect("monsterCarnival");
        assert!(carnival.map_divided);
        assert!(carnival
            .guardian_spots
            .iter()
            .all(|spot| spot.team.is_some()));
        assert!(game_data
            .field(980_000_100)
            .expect("waiting room")
            .carnival
            .is_none());
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
    fn pyramid_stages_fix_their_mob_count_and_spawn_interval() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        for map_id in [926_010_100, 926_010_101, 926_023_500] {
            let stage = game_data.field(map_id).expect("a Pyramid stage");
            assert_eq!(stage.field_type, FIELD_TYPE_PYRAMID, "{map_id}");
            assert_eq!(stage.fixed_mob_capacity, Some(70), "{map_id}");
            assert_eq!(stage.create_mob_interval, Some(3000), "{map_id}");
        }
        let henesys = game_data.field(100_000_000).expect("Henesys");
        assert_eq!(henesys.fixed_mob_capacity, None);
        assert_eq!(henesys.create_mob_interval, None);
        assert_eq!(henesys.massacre, None);
    }

    #[test]
    fn pyramid_stages_set_their_gauge_and_kill_rewards() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let massacre = |map_id| {
            game_data
                .field(map_id)
                .and_then(|field| field.massacre.clone())
                .expect("a Pyramid stage")
        };
        // Solo easy, through a copy's link to its first room.
        let easy = massacre(926_010_101);
        assert_eq!(
            (
                easy.total,
                easy.decrease,
                easy.hit_add,
                easy.miss_sub,
                easy.cool_add
            ),
            (100, 1, 1, 4, 5)
        );
        assert!(!easy.disable_skill);
        let counts: Vec<_> = easy.count_effects.iter().map(|e| e.count).collect();
        assert_eq!(counts, [250, 500, 1000, 1500, 2000, 2500, 3000]);
        assert_eq!(
            easy.count_effects[0],
            CountEffect {
                count: 250,
                buff: 2022585,
                skill_use: false
            }
        );
        assert_eq!(
            easy.count_effects[1],
            CountEffect {
                count: 500,
                buff: 2022586,
                skill_use: true
            }
        );
        assert_eq!(
            easy.count_effects[3],
            CountEffect {
                count: 1500,
                buff: 0,
                skill_use: true
            }
        );
        // Party hell.
        let hell = massacre(926_023_500);
        assert_eq!(
            (
                hell.total,
                hell.decrease,
                hell.hit_add,
                hell.miss_sub,
                hell.cool_add
            ),
            (200, 8, 1, 15, 7)
        );
        // A Kerning City event room bars skills.
        assert!(massacre(910_320_100).disable_skill);
    }

    #[test]
    fn a_maps_copies_take_its_body_through_their_link() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let pig_farm = game_data.field(100_020_100).expect("a mini dungeon");
        // Its copy has only an `info` node, linking it.
        let copy = game_data.field(100_020_101).expect("a copy of it");
        assert_eq!(copy.map_id, 100_020_101);
        assert_eq!(copy.time_limit, Some(7200));
        assert_eq!(copy.forced_return, Some(100_020_000));
        let names = |field: &FieldTemplate| {
            let portals = field.portals_by_id.values();
            portals
                .map(|portal| portal.name.clone())
                .collect::<Vec<_>>()
        };
        assert!(names(copy).contains(&"out00".to_string()));
        assert_eq!(names(copy), names(pig_farm));
        assert_eq!(copy.footholds.bounds(), pig_farm.footholds.bounds());
        assert!(copy.footholds.bounds().is_some());
        assert_eq!(copy.mob_spawns.len(), pig_farm.mob_spawns.len());
        assert!(!copy.mob_spawns.is_empty());
    }

    #[test]
    fn pq_storage_rooms_are_everlast() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = |map_id| game_data.field(map_id).expect("map");
        // OPQ's Jail Storage and GPQ's Center Banquet.
        assert!(field(920_010_911).everlast && !field(920_010_911).may_drop_items());
        assert!(field(990_000_501).everlast);
        assert!(!field(100_000_000).everlast && field(100_000_000).may_drop_items());
    }

    #[test]
    fn maps_have_their_traps() {
        let Some(path) = test_assets::path("Map.nx") else {
            return;
        };
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let traps = |map_id| game_data.field(map_id).expect("a map").traps.clone();

        // The Forest of Patience's thorns hurt for 1.
        let patience = traps(101_000_100);
        assert_eq!((patience.damage, patience.max_hit(500)), (1, 1));
        assert!(!patience.force_hp && patience.target_fields.is_empty());

        // Adobis's Mission I's are rolled within a tenth.
        assert_eq!(traps(280_010_010).max_hit(500), 220);

        // Test of Agility's menhirs leave the player on 100 HP.
        let agility = traps(610_030_300);
        assert!(agility.force_hp);
        assert_eq!(agility.max_hit(80_000), 80_000);

        // The Witch Tower's snacks send players back to its entrance.
        let tower = traps(980_041_000);
        assert_eq!(
            (tower.damage, tower.target_fields.as_slice()),
            (0, &[980_040_000][..])
        );

        assert_eq!(traps(100_000_000), Traps::default());
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
            water: false,
            cold: true,
        };
        assert_eq!(hazard(211_040_000), Some(cold));
        // Ice Valley's quest maps have no cape, but are El Nath's.
        assert!(hazard(921_100_100).is_some_and(|h| h.cold));
        // The Chilly Hill is the internet cafe's.
        assert!(hazard(196_000_000).is_some_and(|h| h.cold));
        // The Other Dimension hurts, but is not cold.
        assert!(hazard(108_010_101).is_some_and(|h| !h.cold));
        let balrogs_tomb = Hazard {
            hp: 50,
            every: Duration::from_secs(15),
            protect_item: None,
            water: false,
            cold: false,
        };
        assert_eq!(hazard(105_100_300), Some(balrogs_tomb));
        // The tower's foot is under water, though El Nath's.
        let seabed = Hazard {
            hp: 6,
            every: HAZARD_INTERVAL,
            protect_item: Some(1_102_061),
            water: true,
            cold: false,
        };
        assert_eq!(hazard(200_082_300), Some(seabed));
        assert_eq!(hazard(211_000_000), None);
    }
}
