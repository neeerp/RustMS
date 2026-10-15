//! Item templates from `Item.nx` (use, setup, etc, cash items) and
//! `Character.nx` (equipment). Looked up on demand: both files are memory
//! mapped and each lookup is a few binary searches.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

/// Stack size when `info/slotMax` is absent (v83: 1 for equipment, else 100).
pub const DEFAULT_SLOT_MAX: i16 = 100;

#[derive(Clone, Debug, Default, PartialEq)]
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
    /// What one throwing star or bullet costs to recharge (`unitPrice`).
    pub unit_price: f64,
    /// What a portable chair heals every 10 seconds (`recoveryHP`/`recoveryMP`).
    pub recovery_hp: i32,
    pub recovery_mp: i32,
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
#[derive(Clone, Debug, Default, PartialEq)]
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
    /// `info/recovery`: how much faster the wearer recovers on a map whose
    /// own recovery is above 1 (the sauna robes, 1.5).
    pub recovery: Option<f64>,
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

    /// Every Monster Book card (`Consume/0238.img`) as `(card, mob)`, in
    /// card order.
    pub fn monster_cards(&self) -> Result<Vec<(i32, i32)>, GameDataError> {
        let nx = &self.items;
        let Some(cards) = path_index(nx, "Consume/0238.img")? else {
            return Ok(Vec::new());
        };
        let mut found = Vec::new();
        for node in nx.child_indices(cards)? {
            let Ok(card) = nx.node_name(node)?.parse::<i32>() else {
                continue;
            };
            let Some(info) = nx.child_by_name(node, "info")? else {
                continue;
            };
            if let Some(mob) = nx.int_child(info, "mob")? {
                found.push((card, mob));
            }
        }
        Ok(found)
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
            if let Some(max) = nx.int_child(meta, "slotMax")? {
                info.slot_max = i16::try_from(max).unwrap_or(i16::MAX).max(1);
            }
            info.quest = nx.int_child(meta, "quest")? == Some(1);
            info.only = nx.int_child(meta, "only")? == Some(1);
            info.trade_block = nx.int_child(meta, "tradeBlock")? == Some(1);
            info.price = nx.int_child(meta, "price")?.unwrap_or(0);
            info.unit_price = nx.real_child(meta, "unitPrice")?.unwrap_or(0.0);
            info.recovery_hp = nx.int_child(meta, "recoveryHP")?.unwrap_or(0);
            info.recovery_mp = nx.int_child(meta, "recoveryMP")?.unwrap_or(0);
        }
        if let Some(spec) = nx.child_by_name(node, "spec")? {
            info.spec = Some(ConsumeSpec {
                hp: nx.int_child(spec, "hp")?.unwrap_or(0),
                mp: nx.int_child(spec, "mp")?.unwrap_or(0),
                hp_r: nx.int_child(spec, "hpR")?.unwrap_or(0),
                mp_r: nx.int_child(spec, "mpR")?.unwrap_or(0),
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
            Ok(nx
                .int_child(meta, key)?
                .map_or(0, |v| i16::try_from(v).unwrap_or(0)))
        };
        let equip = EquipInfo {
            upgrade_slots: u8::try_from(nx.int_child(meta, "tuc")?.unwrap_or(0)).unwrap_or(0),
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
            cash: nx.int_child(meta, "cash")? == Some(1),
            recovery: nx.real_child(meta, "recovery")?,
        };
        Ok(Some(ItemInfo {
            item_id,
            slot_max: 1,
            quest: nx.int_child(meta, "quest")? == Some(1),
            only: nx.int_child(meta, "only")? == Some(1),
            trade_block: nx.int_child(meta, "tradeBlock")? == Some(1),
            price: nx.int_child(meta, "price")?.unwrap_or(0),
            unit_price: 0.0,
            recovery_hp: 0,
            recovery_mp: 0,
            spec: None,
            equip: Some(equip),
        }))
    }
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
        let subi = data.info(2070000).unwrap().unwrap();
        assert_eq!(
            (subi.price, subi.unit_price, subi.slot_max),
            (250, 0.3, 500)
        );
        let relaxer = data.info(3010000).unwrap().unwrap();
        assert_eq!((relaxer.recovery_hp, relaxer.recovery_mp), (50, 0));
    }

    #[test]
    fn reads_monster_cards_and_their_mobs() {
        let Some(data) = data() else {
            return;
        };
        let cards = data.monster_cards().unwrap();
        assert!(cards.len() > 300);
        assert_eq!(cards[0], (2380000, 100100)); // Snail
        assert!(cards.contains(&(2380001, 100101))); // Blue Snail
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
        assert_eq!(sword.recovery, None);
        let robe = data.info(1050018).unwrap().unwrap().equip.unwrap();
        assert_eq!(robe.recovery, Some(1.5));
    }
}
