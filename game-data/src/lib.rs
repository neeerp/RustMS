use memmap2::Mmap;
use nx_pkg4::NxFile;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;
use thiserror::Error;

const NODE_SIZE_BYTES: u64 = 20;
pub const NO_DESTINATION_MAP: i32 = 999_999_999;
pub const PORTAL_TYPE_START_POINT: i32 = 0;

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

#[derive(Clone, Debug)]
pub struct FieldTemplate {
    pub map_id: i32,
    pub return_map: Option<i32>,
    pub forced_return: Option<i32>,
    pub map_npcs: Vec<MapNpcTemplate>,
    portals_by_id: BTreeMap<u32, PortalTemplate>,
    portal_ids_by_name: HashMap<String, Vec<u32>>,
}

impl FieldTemplate {
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

    pub fn resolve_spawn_portal(&self, preferred_name: &str) -> Option<&PortalTemplate> {
        if !preferred_name.trim().is_empty() {
            if let Some(portal) = self.portal_by_name(preferred_name) {
                return Some(portal);
            }
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
}

impl GameData {
    pub fn load_from_nx_map(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxMapFile::open(path.as_ref())?;
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

        Ok(Self { fields })
    }

    pub fn field(&self, map_id: i32) -> Option<&FieldTemplate> {
        self.fields.get(&map_id)
    }

    pub fn field_exists(&self, map_id: i32) -> bool {
        self.fields.contains_key(&map_id)
    }
}

fn build_field_template(
    nx: &NxMapFile,
    map_node_idx: u32,
    map_id: i32,
) -> Result<FieldTemplate, GameDataError> {
    let (return_map, forced_return) =
        if let Some(info_idx) = nx.child_by_name(map_node_idx, "info")? {
            (
                normalize_optional_map_id(nx.int_child(info_idx, "returnMap")?),
                normalize_optional_map_id(nx.int_child(info_idx, "forcedReturn")?),
            )
        } else {
            (None, None)
        };

    let map_npcs = build_map_npcs_from_life(nx, map_node_idx)?;

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
        map_npcs,
        portals_by_id,
        portal_ids_by_name,
    })
}

fn build_map_npcs_from_life(
    nx: &NxMapFile,
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
    nx: &NxMapFile,
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
struct NxMapFile {
    mmap: Mmap,
    node_count: u32,
    node_offset: u64,
    string_count: u32,
    string_offset: u64,
}

impl NxMapFile {
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
        if node.data_type != 1 {
            return Ok(None);
        }

        let signed = i64::from_le_bytes(node.data.to_le_bytes());
        i32::try_from(signed).map(Some).map_err(|_| {
            GameDataError::InvalidData(format!("integer '{key}' out of range: {signed}"))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolve_spawn_portal_prefers_name_then_startpoint() {
        let field = FieldTemplate {
            map_id: 1,
            return_map: None,
            forced_return: None,
            map_npcs: Vec::new(),
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
        assert_eq!(
            field
                .resolve_spawn_portal("missing")
                .expect("fallback startpoint")
                .id,
            0
        );
    }

    #[test]
    fn loads_map_npcs_from_assets_map_nx() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx");
        let game_data = GameData::load_from_nx_map(&path).expect("load map nx");
        let field = game_data.field(10_000).expect("field 10000");

        assert!(field.map_npcs.iter().any(|npc| {
            npc.npc_id == 2101
                && npc.x == 130
                && npc.y == 293
                && npc.foothold == 51
                && npc.rx0 <= npc.x
                && npc.rx1 >= npc.x
        }));
    }
}
