use std::collections::BTreeMap;
use std::time::SystemTime;

/// The five bags plus the equipped pseudo-inventory, numbered as on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InventoryType {
    Equipped = -1,
    Equip = 1,
    Use = 2,
    Setup = 3,
    Etc = 4,
    Cash = 5,
}

impl InventoryType {
    pub const BAGS: [InventoryType; 5] = [
        InventoryType::Equip,
        InventoryType::Use,
        InventoryType::Setup,
        InventoryType::Etc,
        InventoryType::Cash,
    ];

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            -1 => Some(Self::Equipped),
            1 => Some(Self::Equip),
            2 => Some(Self::Use),
            3 => Some(Self::Setup),
            4 => Some(Self::Etc),
            5 => Some(Self::Cash),
            _ => None,
        }
    }

    /// The bag an item id belongs in, derived from its id range.
    pub fn for_item(item_id: i32) -> Option<Self> {
        match item_id / 1_000_000 {
            1 => Some(Self::Equip),
            2 => Some(Self::Use),
            3 => Some(Self::Setup),
            4 => Some(Self::Etc),
            5 => Some(Self::Cash),
            _ => None,
        }
    }

    fn bag_index(self) -> Option<usize> {
        match self {
            Self::Equipped => None,
            other => Some(other as usize - 1),
        }
    }
}

/// Equipped item positions. Regular equipment uses negative body slots;
/// cash (cosmetic) equipment is offset by a further -100.
pub mod equip_slot {
    pub const CAP: i16 = -1;
    pub const TOP: i16 = -5;
    pub const BOTTOM: i16 = -6;
    pub const SHOES: i16 = -7;
    pub const WEAPON: i16 = -11;
    pub const CASH_WEAPON: i16 = -111;

    /// Positive body slot for a regular equipped position.
    pub fn body_slot(position: i16) -> Option<u8> {
        (-99..=-1).contains(&position).then(|| (-position) as u8)
    }

    /// Positive body slot for a cash equipped position.
    pub fn cash_body_slot(position: i16) -> Option<u8> {
        (position != CASH_WEAPON && (-199..=-101).contains(&position))
            .then(|| (-position - 100) as u8)
    }

    /// The regular slot an equip id is worn in, from its id range.
    pub fn for_item(item_id: i32) -> Option<i16> {
        match item_id / 10_000 {
            100 => Some(CAP),
            104 | 105 => Some(TOP),
            106 => Some(BOTTOM),
            107 => Some(SHOES),
            130..=170 => Some(WEAPON),
            _ => None,
        }
    }
}

/// Stats carried by an equipment instance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipStats {
    pub upgrade_slots: u8,
    pub level: u8,
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
    pub vicious: i32,
    pub item_level: u8,
    pub item_exp: i32,
}

/// One stack (or one equipment instance) occupying an inventory slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub item_id: i32,
    pub quantity: i16,
    pub owner: String,
    pub flags: i16,
    pub expiration: Option<SystemTime>,
    pub cash_serial: Option<i64>,
    /// Present for equipment, absent for stackable items.
    pub equip: Option<EquipStats>,
}

impl Item {
    pub fn stack(item_id: i32, quantity: i16) -> Self {
        Self {
            item_id,
            quantity,
            owner: String::new(),
            flags: 0,
            expiration: None,
            cash_serial: None,
            equip: None,
        }
    }

    pub fn equipment(item_id: i32, stats: EquipStats) -> Self {
        Self {
            equip: Some(stats),
            ..Self::stack(item_id, 1)
        }
    }

    pub fn is_cash(&self) -> bool {
        self.item_id / 1_000_000 == 5 || self.cash_serial.is_some()
    }
}

pub const DEFAULT_SLOT_LIMIT: u8 = 24;
pub const DEFAULT_CASH_SLOT_LIMIT: u8 = 96;

/// Everything a character carries: equipped items plus the five bags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    slot_limits: [u8; 5],
    equipped: BTreeMap<i16, Item>,
    bags: [BTreeMap<i16, Item>; 5],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slot_limits: [
                DEFAULT_SLOT_LIMIT,
                DEFAULT_SLOT_LIMIT,
                DEFAULT_SLOT_LIMIT,
                DEFAULT_SLOT_LIMIT,
                DEFAULT_CASH_SLOT_LIMIT,
            ],
            equipped: BTreeMap::new(),
            bags: Default::default(),
        }
    }
}

impl Inventory {
    pub fn slot_limit(&self, kind: InventoryType) -> u8 {
        kind.bag_index().map_or(0, |index| self.slot_limits[index])
    }

    pub fn set_slot_limit(&mut self, kind: InventoryType, limit: u8) {
        if let Some(index) = kind.bag_index() {
            self.slot_limits[index] = limit;
        }
    }

    /// Items in `kind`, ordered by slot.
    pub fn items(&self, kind: InventoryType) -> impl Iterator<Item = (i16, &Item)> {
        let map = match kind.bag_index() {
            Some(index) => &self.bags[index],
            None => &self.equipped,
        };
        map.iter().map(|(&slot, item)| (slot, item))
    }

    /// Place an item at an explicit slot, replacing whatever was there.
    pub fn insert(&mut self, kind: InventoryType, slot: i16, item: Item) -> Option<Item> {
        match kind.bag_index() {
            Some(index) => self.bags[index].insert(slot, item),
            None => self.equipped.insert(slot, item),
        }
    }

    /// Equipped regular items (slot > -100), ordered by slot.
    pub fn equipped_regular(&self) -> impl Iterator<Item = (i16, &Item)> {
        self.equipped
            .iter()
            .filter(|(&slot, _)| slot > -100)
            .map(|(&slot, item)| (slot, item))
    }

    /// Equipped cash items (slot <= -100), ordered by slot.
    pub fn equipped_cash(&self) -> impl Iterator<Item = (i16, &Item)> {
        self.equipped
            .iter()
            .filter(|(&slot, _)| slot <= -100)
            .map(|(&slot, item)| (slot, item))
    }

    pub fn equipped_ids(&self) -> impl Iterator<Item = (i16, i32)> + '_ {
        self.equipped
            .iter()
            .map(|(&slot, item)| (slot, item.item_id))
    }

    /// Every item with its inventory and slot, for persistence.
    pub fn all(&self) -> impl Iterator<Item = (InventoryType, i16, &Item)> {
        let equipped = self
            .equipped
            .iter()
            .map(|(&slot, item)| (InventoryType::Equipped, slot, item));
        let bags = InventoryType::BAGS
            .into_iter()
            .flat_map(move |kind| self.items(kind).map(move |(slot, item)| (kind, slot, item)));
        equipped.chain(bags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_grouped_by_inventory_and_ordered_by_slot() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Use, 3, Item::stack(2000000, 5));
        inventory.insert(InventoryType::Use, 1, Item::stack(2000001, 1));
        inventory.insert(
            InventoryType::Equipped,
            equip_slot::WEAPON,
            Item::equipment(1302000, EquipStats::default()),
        );

        let slots: Vec<_> = inventory
            .items(InventoryType::Use)
            .map(|(s, _)| s)
            .collect();
        assert_eq!(slots, vec![1, 3]);
        assert_eq!(inventory.all().count(), 3);
        assert_eq!(inventory.equipped_regular().count(), 1);
        assert_eq!(
            inventory.slot_limit(InventoryType::Cash),
            DEFAULT_CASH_SLOT_LIMIT
        );
    }

    #[test]
    fn equip_slots_follow_item_id_ranges() {
        assert_eq!(equip_slot::for_item(1040002), Some(equip_slot::TOP));
        assert_eq!(equip_slot::for_item(1052000), Some(equip_slot::TOP));
        assert_eq!(equip_slot::for_item(1060002), Some(equip_slot::BOTTOM));
        assert_eq!(equip_slot::for_item(1072001), Some(equip_slot::SHOES));
        assert_eq!(equip_slot::for_item(1302000), Some(equip_slot::WEAPON));
        assert_eq!(equip_slot::for_item(2000000), None);
        assert_eq!(InventoryType::for_item(4000000), Some(InventoryType::Etc));
    }
}
