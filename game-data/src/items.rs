//! Item templates from `Item.nx` (use, setup, etc, cash items) and
//! `Character.nx` (equipment). Looked up on demand: both files are memory
//! mapped and each lookup is a few binary searches.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

/// Stack size when `info/slotMax` is absent (v83: 1 for equipment, else 100).
pub const DEFAULT_SLOT_MAX: i16 = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemInfo {
    pub item_id: i32,
    pub slot_max: i16,
    /// Quest items (`info/quest`).
    pub quest: bool,
    /// One-of-a-kind items (`info/only`).
    pub only: bool,
    /// Items that can't change hands (`info/tradeBlock`).
    pub trade_block: bool,
    pub price: i32,
    /// `spec` of consumable items.
    pub spec: Option<ConsumeSpec>,
    pub equip: Option<EquipInfo>,
}

/// What using a consumable does (`spec`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsumeSpec {
    pub hp: i32,
    pub mp: i32,
    /// Percent of max HP/MP.
    pub hp_r: i32,
    pub mp_r: i32,
}

/// An equipment template (`Character.nx/<category>/<id>.img/info`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipInfo {
    pub upgrade_slots: u8,
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub hp: i16,
    pub mp: i16,
    pub watk: i16,
    pub matk: i16,
    pub wdef: i16,
    pub mdef: i16,
    pub acc: i16,
    pub avoid: i16,
    pub hands: i16,
    pub speed: i16,
    pub jump: i16,
    pub req_level: i16,
    /// Bit flags of job branches (1 warrior, 2 magician, 4 bowman, 8 thief,
    /// 16 pirate); 0 means anyone.
    pub req_job: i16,
    pub req_str: i16,
    pub req_dex: i16,
    pub req_int: i16,
    pub req_luk: i16,
    pub cash: bool,
}

/// Top-level `Item.nx` directory for a non-equipment item id.
fn item_directory(item_id: i32) -> Option<&'static str> {
    match item_id / 1_000_000 {
        2 => Some("Consume"),
        3 => Some("Install"),
        4 => Some("Etc"),
        5 => Some("Cash"),
        _ => None,
    }
}

/// `Character.nx` directories that hold equipment.
const EQUIP_DIRECTORIES: &[&str] = &[
    "Accessory",
    "Cap",
    "Cape",
    "Coat",
    "Glove",
    "Longcoat",
    "Pants",
    "PetEquip",
    "Ring",
    "Shield",
    "Shoes",
    "TamingMob",
    "Weapon",
];

pub struct ItemData {
    items: NxDataFile,
    equipment: NxDataFile,
}

impl ItemData {
    pub fn load(
        items: impl AsRef<Path>,
        equipment: impl AsRef<Path>,
    ) -> Result<Self, GameDataError> {
        Ok(Self {
            items: NxDataFile::open(items.as_ref())?,
            equipment: NxDataFile::open(equipment.as_ref())?,
        })
    }

    /// The template for `item_id`, or `None` if the data has no such item.
    pub fn info(&self, item_id: i32) -> Result<Option<ItemInfo>, GameDataError> {
        if item_id / 1_000_000 == 1 {
            return self.equipment_info(item_id);
        }
        let Some(directory) = item_directory(item_id) else {
            return Ok(None);
        };
        let nx = &self.items;
        let path = format!("{directory}/{:04}.img/{:08}", item_id / 10_000, item_id);
        let Some(node) = path_index(nx, &path)? else {
            return Ok(None);
        };
        let mut info = ItemInfo {
            item_id,
            slot_max: DEFAULT_SLOT_MAX,
            ..ItemInfo::default()
        };
        if let Some(meta) = nx.child_by_name(node, "info")? {
            if let Some(max) = number(nx, meta, "slotMax")? {
                info.slot_max = i16::try_from(max).unwrap_or(i16::MAX).max(1);
            }
            info.quest = number(nx, meta, "quest")? == Some(1);
            info.only = number(nx, meta, "only")? == Some(1);
            info.trade_block = number(nx, meta, "tradeBlock")? == Some(1);
            info.price = number(nx, meta, "price")?.unwrap_or(0);
        }
        if let Some(spec) = nx.child_by_name(node, "spec")? {
            info.spec = Some(ConsumeSpec {
                hp: number(nx, spec, "hp")?.unwrap_or(0),
                mp: number(nx, spec, "mp")?.unwrap_or(0),
                hp_r: number(nx, spec, "hpR")?.unwrap_or(0),
                mp_r: number(nx, spec, "mpR")?.unwrap_or(0),
            });
        }
        Ok(Some(info))
    }

    fn equipment_info(&self, item_id: i32) -> Result<Option<ItemInfo>, GameDataError> {
        let nx = &self.equipment;
        let image = format!("{item_id:08}.img");
        let mut found = None;
        for directory in EQUIP_DIRECTORIES {
            if let Some(index) = path_index(nx, &format!("{directory}/{image}"))? {
                found = Some(index);
                break;
            }
        }
        let Some(node) = found else {
            return Ok(None);
        };
        let Some(meta) = nx.child_by_name(node, "info")? else {
            return Ok(None);
        };
        let stat = |key: &str| -> Result<i16, GameDataError> {
            Ok(number(nx, meta, key)?.map_or(0, |v| i16::try_from(v).unwrap_or(0)))
        };
        let equip = EquipInfo {
            upgrade_slots: u8::try_from(number(nx, meta, "tuc")?.unwrap_or(0)).unwrap_or(0),
            str: stat("incSTR")?,
            dex: stat("incDEX")?,
            int: stat("incINT")?,
            luk: stat("incLUK")?,
            hp: stat("incMHP")?,
            mp: stat("incMMP")?,
            watk: stat("incPAD")?,
            matk: stat("incMAD")?,
            wdef: stat("incPDD")?,
            mdef: stat("incMDD")?,
            acc: stat("incACC")?,
            avoid: stat("incEVA")?,
            hands: stat("incCraft")?,
            speed: stat("incSpeed")?,
            jump: stat("incJump")?,
            req_level: stat("reqLevel")?,
            req_job: stat("reqJob")?,
            req_str: stat("reqSTR")?,
            req_dex: stat("reqDEX")?,
            req_int: stat("reqINT")?,
            req_luk: stat("reqLUK")?,
            cash: number(nx, meta, "cash")? == Some(1),
        };
        Ok(Some(ItemInfo {
            item_id,
            slot_max: 1,
            quest: number(nx, meta, "quest")? == Some(1),
            only: number(nx, meta, "only")? == Some(1),
            trade_block: number(nx, meta, "tradeBlock")? == Some(1),
            price: number(nx, meta, "price")?.unwrap_or(0),
            spec: None,
            equip: Some(equip),
        }))
    }
}

/// An integer child, also accepting numbers stored as strings.
fn number(nx: &NxDataFile, parent: u32, key: &str) -> Result<Option<i32>, GameDataError> {
    if let Some(value) = nx.int_child(parent, key)? {
        return Ok(Some(value));
    }
    Ok(nx
        .string_child(parent, key)?
        .and_then(|s| s.trim().parse().ok()))
}

fn path_index(nx: &NxDataFile, path: &str) -> Result<Option<u32>, GameDataError> {
    let mut index = 0;
    for key in path.split('/') {
        match nx.child_by_name(index, key)? {
            Some(child) => index = child,
            None => return Ok(None),
        }
    }
    Ok(Some(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<ItemData> {
        let items = crate::test_assets::path("Item.nx")?;
        let equipment = crate::test_assets::path("Character.nx")?;
        Some(ItemData::load(items, equipment).unwrap())
    }

    #[test]
    fn reads_consumables_quest_items_and_stack_sizes() {
        let Some(data) = data() else {
            return;
        };
        let apple = data.info(2010007).unwrap().unwrap();
        assert!(apple.quest);
        assert_eq!(apple.spec.unwrap().hp, 30);
        assert_eq!(apple.slot_max, DEFAULT_SLOT_MAX);
        let shell = data.info(4000019).unwrap().unwrap();
        assert_eq!(shell.slot_max, 200);
        assert!(!shell.trade_block);
        // Stored as an integer in one and as a string in the other.
        assert!(data.info(2000012).unwrap().unwrap().trade_block);
        assert!(data.info(4031044).unwrap().unwrap().trade_block);
        assert!(data.info(4999999).unwrap().is_none());
    }

    #[test]
    fn reads_equipment_stats_and_requirements() {
        let Some(data) = data() else {
            return;
        };
        let hat = data.info(1002067).unwrap().unwrap();
        let equip = hat.equip.unwrap();
        assert_eq!(
            (equip.upgrade_slots, equip.wdef, equip.req_level),
            (7, 5, 5)
        );
        assert_eq!(hat.slot_max, 1);
        let sword = data.info(1302000).unwrap().unwrap().equip.unwrap();
        assert!(sword.watk > 0);
    }
}
