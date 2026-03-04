use crate::model::{normalize_script, normalize_to_name, MapIndex, MapRecord, PortalRecord};
use anyhow::{bail, Context, Result};
use roxmltree::{Document, Node};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn read_maps(cosmic_map_root: &Path, target_maps: Option<&BTreeSet<i32>>) -> Result<MapIndex> {
    if !cosmic_map_root.is_dir() {
        bail!(
            "Cosmic map root '{}' is not a directory",
            cosmic_map_root.display()
        );
    }

    let mut maps = BTreeMap::new();

    for entry in WalkDir::new(cosmic_map_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !file_name.ends_with(".img.xml") {
            continue;
        }

        let Some(map_id) = parse_map_id(file_name) else {
            continue;
        };

        if let Some(targets) = target_maps {
            if !targets.contains(&map_id) {
                continue;
            }
        }

        let map = read_map_file(path, map_id)
            .with_context(|| format!("failed parsing Cosmic map XML {}", path.display()))?;
        maps.insert(map_id, map);
    }

    Ok(maps)
}

fn parse_map_id(file_name: &str) -> Option<i32> {
    let id_str = file_name.strip_suffix(".img.xml")?;
    id_str.parse().ok()
}

fn read_map_file(path: &Path, map_id: i32) -> Result<MapRecord> {
    let raw = fs::read_to_string(path)?;
    let doc = Document::parse(raw.as_str())?;
    parse_map_doc(&doc, map_id)
}

fn parse_map_doc(doc: &Document<'_>, map_id: i32) -> Result<MapRecord> {
    let mut portals = BTreeMap::new();

    let Some(portal_root) = doc
        .descendants()
        .find(|node| node.has_tag_name("imgdir") && node.attribute("name") == Some("portal"))
    else {
        return Ok(MapRecord { map_id, portals });
    };

    for portal_node in portal_root
        .children()
        .filter(|node| node.is_element() && node.has_tag_name("imgdir"))
    {
        let Some(portal_id_str) = portal_node.attribute("name") else {
            continue;
        };
        let Ok(portal_id) = portal_id_str.parse::<u32>() else {
            continue;
        };

        let portal = parse_portal_node(portal_node, map_id, portal_id)?;
        portals.insert(portal_id, portal);
    }

    Ok(MapRecord { map_id, portals })
}

fn parse_portal_node(
    portal_node: Node<'_, '_>,
    map_id: i32,
    portal_id: u32,
) -> Result<PortalRecord> {
    let name = find_string(portal_node, "pn")
        .with_context(|| format!("map {map_id} portal {portal_id} missing pn"))?;
    let portal_type = find_int(portal_node, "pt")?.unwrap_or(0);
    let x = find_int(portal_node, "x")?.unwrap_or(0);
    let y = find_int(portal_node, "y")?.unwrap_or(0);
    let to_map = find_int(portal_node, "tm")?.unwrap_or(999_999_999);
    let to_name = normalize_to_name(find_string(portal_node, "tn").unwrap_or_default().as_str());
    let script = normalize_script(find_string(portal_node, "script"));

    Ok(PortalRecord {
        id: portal_id,
        name,
        portal_type,
        x,
        y,
        to_map,
        to_name,
        script,
    })
}

fn find_int(portal_node: Node<'_, '_>, key: &str) -> Result<Option<i32>> {
    let Some(node) = find_child_by_name(portal_node, key) else {
        return Ok(None);
    };

    let Some(value) = node.attribute("value") else {
        return Ok(None);
    };

    let parsed = value
        .parse::<i32>()
        .with_context(|| format!("invalid int for key '{key}': '{value}'"))?;
    Ok(Some(parsed))
}

fn find_string(portal_node: Node<'_, '_>, key: &str) -> Option<String> {
    find_child_by_name(portal_node, key)
        .and_then(|node| node.attribute("value"))
        .map(ToOwned::to_owned)
}

fn find_child_by_name<'a>(portal_node: Node<'a, 'a>, key: &str) -> Option<Node<'a, 'a>> {
    portal_node.children().find(|child| {
        child.is_element()
            && matches!(child.tag_name().name(), "int" | "string")
            && child.attribute("name") == Some(key)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_portal_block() {
        let xml = r#"
<imgdir name="100000001.img">
  <imgdir name="portal">
    <imgdir name="1">
      <string name="pn" value="out02"/>
      <int name="pt" value="2"/>
      <int name="x" value="202"/>
      <int name="y" value="242"/>
      <int name="tm" value="100000000"/>
      <string name="tn" value="in02"/>
    </imgdir>
  </imgdir>
</imgdir>
"#;

        let doc = Document::parse(xml).expect("xml parses");
        let map = parse_map_doc(&doc, 100000001).expect("map parses");
        let portal = map.portals.get(&1).expect("portal exists");

        assert_eq!(portal.name, "out02");
        assert_eq!(portal.portal_type, 2);
        assert_eq!(portal.to_map, 100000000);
        assert_eq!(portal.to_name, "in02");
    }
}
