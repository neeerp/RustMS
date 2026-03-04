use crate::model::{normalize_script, normalize_to_name, MapIndex, MapRecord, PortalRecord};
use anyhow::{anyhow, bail, Context, Result};
use memmap2::Mmap;
use nx_pkg4::NxFile;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;

const NODE_SIZE_BYTES: u64 = 20;

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
    fn open(path: &Path) -> Result<Self> {
        NxFile::open(path).context("nx-pkg4 failed to open file")?;

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let magic = read_u32(&mmap, 0)?;
        if magic != 0x3447_4B50 {
            bail!("invalid NX header magic");
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

    fn node_at(&self, index: u32) -> Result<NxNodeData> {
        if index >= self.node_count {
            bail!(
                "node index {index} out of bounds for node_count={}",
                self.node_count
            );
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

    fn node_name(&self, index: u32) -> Result<String> {
        let node = self.node_at(index)?;
        self.string_at(node.name_index)
    }

    fn string_at(&self, index: u32) -> Result<String> {
        if index >= self.string_count {
            bail!(
                "string index {index} out of bounds for string_count={}",
                self.string_count
            );
        }

        let pointer_offset = self.string_offset + index as u64 * 8;
        let string_data_offset = read_u64(&self.mmap, pointer_offset)?;
        let len = read_u16(&self.mmap, string_data_offset)? as usize;
        let start = (string_data_offset + 2) as usize;
        let end = start + len;

        let bytes = self
            .mmap
            .get(start..end)
            .ok_or_else(|| anyhow!("string byte range {start}..{end} out of bounds"))?;

        let value = std::str::from_utf8(bytes)?;
        Ok(value.to_owned())
    }

    fn child_by_name(&self, parent_index: u32, child_name: &str) -> Result<Option<u32>> {
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

    fn child_indices(&self, parent_index: u32) -> Result<Vec<u32>> {
        let parent = self.node_at(parent_index)?;
        let start = parent.children_index;
        let end = start + parent.child_count as u32;
        Ok((start..end).collect())
    }

    fn int_child(&self, parent_index: u32, key: &str) -> Result<Option<i32>> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };

        let node = self.node_at(child_index)?;
        if node.data_type != 1 {
            return Ok(None);
        }

        let signed = i64::from_le_bytes(node.data.to_le_bytes());
        Ok(Some(i32::try_from(signed).with_context(|| {
            format!("integer '{key}' value out of i32 range: {signed}")
        })?))
    }

    fn string_child(&self, parent_index: u32, key: &str) -> Result<Option<String>> {
        let Some(child_index) = self.child_by_name(parent_index, key)? else {
            return Ok(None);
        };

        let node = self.node_at(child_index)?;
        if node.data_type != 3 {
            return Ok(None);
        }

        let string_index = u32::try_from(node.data).context("string index out of u32 range")?;
        Ok(Some(self.string_at(string_index)?))
    }
}

pub fn read_maps(nx_map_path: &Path, target_maps: Option<&BTreeSet<i32>>) -> Result<MapIndex> {
    let nx = NxMapFile::open(nx_map_path)?;

    let map_root = nx
        .child_by_name(0, "Map")?
        .ok_or_else(|| anyhow!("missing root child 'Map'"))?;

    let mut maps = BTreeMap::new();

    for map_group_idx in nx.child_indices(map_root)? {
        let map_group_name = nx.node_name(map_group_idx)?;
        if !map_group_name.starts_with("Map") {
            continue;
        }

        for map_node_idx in nx.child_indices(map_group_idx)? {
            let map_node_name = nx.node_name(map_node_idx)?;
            let Some(map_id) = parse_map_id(map_node_name.as_str()) else {
                continue;
            };

            if let Some(targets) = target_maps {
                if !targets.contains(&map_id) {
                    continue;
                }
            }

            let map_record = read_map_record(&nx, map_node_idx, map_id)
                .with_context(|| format!("failed reading NX map node {map_node_name}"))?;
            maps.insert(map_id, map_record);
        }
    }

    Ok(maps)
}

fn parse_map_id(map_node_name: &str) -> Option<i32> {
    map_node_name
        .strip_suffix(".img")
        .and_then(|id| id.parse::<i32>().ok())
}

fn read_map_record(nx: &NxMapFile, map_node_idx: u32, map_id: i32) -> Result<MapRecord> {
    let Some(portal_root_idx) = nx.child_by_name(map_node_idx, "portal")? else {
        return Ok(MapRecord {
            map_id,
            portals: BTreeMap::new(),
        });
    };

    let mut portals = BTreeMap::new();

    for portal_node_idx in nx.child_indices(portal_root_idx)? {
        let portal_id_name = nx.node_name(portal_node_idx)?;
        let Ok(portal_id) = portal_id_name.parse::<u32>() else {
            continue;
        };

        let name = nx
            .string_child(portal_node_idx, "pn")?
            .ok_or_else(|| anyhow!("missing pn"))?;
        let portal_type = nx.int_child(portal_node_idx, "pt")?.unwrap_or(0);
        let x = nx.int_child(portal_node_idx, "x")?.unwrap_or(0);
        let y = nx.int_child(portal_node_idx, "y")?.unwrap_or(0);
        let to_map = nx.int_child(portal_node_idx, "tm")?.unwrap_or(999_999_999);
        let to_name = normalize_to_name(
            nx.string_child(portal_node_idx, "tn")?
                .unwrap_or_default()
                .as_str(),
        );
        let script = normalize_script(nx.string_child(portal_node_idx, "script")?);

        portals.insert(
            portal_id,
            PortalRecord {
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

    Ok(MapRecord { map_id, portals })
}

fn read_u16(bytes: &[u8], offset: u64) -> Result<u16> {
    let offset = offset as usize;
    let data = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow!("u16 out of bounds at offset {offset}"))?;
    Ok(u16::from_le_bytes(data.try_into()?))
}

fn read_u32(bytes: &[u8], offset: u64) -> Result<u32> {
    let offset = offset as usize;
    let data = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("u32 out of bounds at offset {offset}"))?;
    Ok(u32::from_le_bytes(data.try_into()?))
}

fn read_u64(bytes: &[u8], offset: u64) -> Result<u64> {
    let offset = offset as usize;
    let data = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| anyhow!("u64 out of bounds at offset {offset}"))?;
    Ok(u64::from_le_bytes(data.try_into()?))
}
