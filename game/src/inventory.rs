use crate::pet::PetData;
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

    pub const FACE_ACCESSORY: i16 = -2;
    pub const EYE_ACCESSORY: i16 = -3;
    pub const EARRINGS: i16 = -4;
    pub const GLOVES: i16 = -8;
    pub const CAPE: i16 = -9;
    pub const SHIELD: i16 = -10;
    pub const RINGS: [i16; 4] = [-12, -13, -15, -16];
    pub const PENDANT: i16 = -17;
    /// The tamed monster (190xxxx) and its saddle (191xxxx).
    pub const MOUNT: i16 = -18;
    pub const SADDLE: i16 = -19;
    pub const MEDAL: i16 = -49;
    pub const BELT: i16 = -50;

    /// The regular slot an equip id is worn in, from its id range. Rings
    /// may go in any of `RINGS`; this returns the first.
    pub fn for_item(item_id: i32) -> Option<i16> {
        match item_id / 10_000 {
            100 => Some(CAP),
            101 => Some(FACE_ACCESSORY),
            102 => Some(EYE_ACCESSORY),
            103 => Some(EARRINGS),
            104 | 105 => Some(TOP),
            106 => Some(BOTTOM),
            107 => Some(SHOES),
            108 => Some(GLOVES),
            109 => Some(SHIELD),
            110 => Some(CAPE),
            111 => Some(RINGS[0]),
            112 => Some(PENDANT),
            113 => Some(BELT),
            114 => Some(MEDAL),
            190 => Some(MOUNT),
            191 => Some(SADDLE),
            130..=170 => Some(WEAPON),
            _ => None,
        }
    }

    /// The slots a pet equip may be worn in, one for each summoned pet in
    /// list order (v83 FUN_00460358). The HP and MP pouches serve every pet
    /// from one slot.
    pub fn pet_slots(item_id: i32) -> Option<[i16; 3]> {
        let parts = match item_id {
            // The Pet Collar goes where a label ring does.
            1_802_100 => [21, 31, 39],
            1_812_000 => [23, 34, 42],
            1_812_001 => [22, 33, 41],
            1_812_002 => [24; 3],
            1_812_003 => [25; 3],
            1_812_004 => [26, 35, 43],
            1_812_005 => [27, 36, 44],
            1_812_006 => [28, 37, 45],
            1_812_007 => [46, 47, 48],
            _ => match item_id / 10_000 {
                180 => [14, 30, 38],
                182 => [21, 31, 39],
                183 => [29, 32, 40],
                _ => return None,
            },
        };
        Some(parts.map(|part| -part))
    }

    /// The list index of the pet a pet equip slot serves, or `None` for the
    /// pouches every pet shares (v83 FUN_00705e2c).
    pub fn pet_index(slot: i16) -> Option<usize> {
        match -slot {
            14 | 21..=23 | 26..=29 | 46 => Some(0),
            30..=37 | 47 => Some(1),
            38..=45 | 48 => Some(2),
            _ => None,
        }
    }

    /// Whether `item_id` may be worn at `position`. As in v83, an item with
    /// a cash serial is worn at its slot less 100, and any other at its slot.
    pub fn fits(item_id: i32, cash: bool, position: i16) -> bool {
        let position = match (cash, position) {
            (true, -199..=-101) => position + 100,
            (false, -99..=-1) => position,
            _ => return false,
        };
        if let Some(slots) = pet_slots(item_id) {
            return slots.contains(&position);
        }
        match for_item(item_id) {
            Some(slot) if RINGS.contains(&slot) => RINGS.contains(&position),
            Some(slot) => slot == position,
            None => false,
        }
    }

    /// An overall occupies the top slot and leaves no room for pants.
    pub fn is_overall(item_id: i32) -> bool {
        item_id / 10_000 == 105
    }

    /// Two-handed weapons leave no room for a shield. For v83 that is every
    /// 14xxxxx weapon, claws, knuckles and guns too (FUN_004f2f55).
    pub fn is_two_handed(item_id: i32) -> bool {
        item_id / 100_000 == 14
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

/// `Item.flags` bit of equipment that can no longer change hands, which
/// equipment with `equipTradeBlock` gets once worn.
pub const FLAG_UNTRADEABLE: i16 = 0x08;

/// `Item.flags` bits Scissors of Karma set, which let an item that can't
/// change hands do so once: one for equipment, another for other items.
pub const FLAG_EQUIP_KARMA: i16 = 0x10;
pub const FLAG_BUNDLE_KARMA: i16 = 0x02;

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
    /// Present for pets.
    pub pet: Option<PetData>,
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
            pet: None,
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

    fn karma_flag(&self) -> i16 {
        if self.item_id / 1_000_000 == 1 {
            FLAG_EQUIP_KARMA
        } else {
            FLAG_BUNDLE_KARMA
        }
    }

    /// Whether Scissors of Karma let the item change hands once more.
    pub fn has_karma(&self) -> bool {
        self.flags & self.karma_flag() != 0
    }

    /// Use up the item's Karma as it changes hands. Equipment can't change
    /// hands again; other items go back to what their data says.
    pub fn use_karma(&mut self) {
        if self.has_karma() {
            self.flags &= !self.karma_flag();
            if self.item_id / 1_000_000 == 1 {
                self.flags |= FLAG_UNTRADEABLE;
            }
        }
    }
}

/// One change to an inventory, in the order the client must apply them
/// (INVENTORY_OPERATION). Equipped positions are negative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryChange {
    Add {
        kind: InventoryType,
        slot: i16,
        item: Item,
    },
    Quantity {
        kind: InventoryType,
        slot: i16,
        quantity: i16,
    },
    /// Move, or swap with whatever is at `to`.
    Move {
        kind: InventoryType,
        from: i16,
        to: i16,
    },
    Remove {
        kind: InventoryType,
        slot: i16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryError {
    /// Not enough free slots.
    Full,
    /// Fewer items than requested.
    NotEnough,
    /// The slot is empty, out of range, or otherwise unusable.
    InvalidSlot,
    /// The item can't be worn there.
    CannotEquip,
    /// The item data has no such item.
    UnknownItem,
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Full => "inventory full",
            Self::NotEnough => "not enough items",
            Self::InvalidSlot => "invalid inventory slot",
            Self::CannotEquip => "item cannot be equipped there",
            Self::UnknownItem => "unknown item",
        })
    }
}

impl std::error::Error for InventoryError {}

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

    pub fn get(&self, kind: InventoryType, slot: i16) -> Option<&Item> {
        self.map(kind).get(&slot)
    }

    /// How many of `item_id` are in its bag.
    pub fn count(&self, item_id: i32) -> i32 {
        InventoryType::for_item(item_id).map_or(0, |kind| {
            self.items(kind)
                .filter(|(_, item)| item.item_id == item_id)
                .map(|(_, item)| i32::from(item.quantity))
                .sum()
        })
    }

    /// Add an item, filling partial stacks of the same id before taking
    /// empty slots. `slot_max` is the item's stack size (1 for equipment).
    /// Nothing changes unless all of it fits.
    pub fn add(
        &mut self,
        item: Item,
        slot_max: i16,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        let kind = InventoryType::for_item(item.item_id).ok_or(InventoryError::InvalidSlot)?;
        let slot_max = slot_max.max(1);
        let stackable =
            item.equip.is_none() && item.expiration.is_none() && item.cash_serial.is_none();
        let mut remaining = item.quantity.max(1);

        // Fill a copy so a full inventory changes nothing.
        let mut next = self.clone();
        let mut changes = Vec::new();
        if stackable {
            let partial: Vec<i16> = self
                .items(kind)
                .filter(|(_, other)| Self::stacks_with(other, &item) && other.quantity < slot_max)
                .map(|(slot, _)| slot)
                .collect();
            for slot in partial {
                if remaining == 0 {
                    break;
                }
                let existing = next
                    .map_mut(kind)
                    .get_mut(&slot)
                    .expect("slot listed above");
                let moved = remaining.min(slot_max - existing.quantity);
                existing.quantity += moved;
                remaining -= moved;
                changes.push(InventoryChange::Quantity {
                    kind,
                    slot,
                    quantity: existing.quantity,
                });
            }
        }
        for slot in self.free_slots(kind) {
            if remaining == 0 {
                break;
            }
            let quantity = if stackable {
                remaining.min(slot_max)
            } else {
                1
            };
            let placed = Item {
                quantity,
                ..item.clone()
            };
            next.map_mut(kind).insert(slot, placed.clone());
            remaining -= quantity;
            changes.push(InventoryChange::Add {
                kind,
                slot,
                item: placed,
            });
        }
        if remaining > 0 {
            return Err(InventoryError::Full);
        }
        *self = next;
        Ok(changes)
    }

    /// Remove `quantity` of `item_id`, from the lowest slots first. Nothing
    /// changes unless there are enough.
    pub fn remove(
        &mut self,
        item_id: i32,
        quantity: i16,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        let kind = InventoryType::for_item(item_id).ok_or(InventoryError::InvalidSlot)?;
        if self.count(item_id) < i32::from(quantity) {
            return Err(InventoryError::NotEnough);
        }
        let slots: Vec<i16> = self
            .items(kind)
            .filter(|(_, item)| item.item_id == item_id)
            .map(|(slot, _)| slot)
            .collect();
        let mut remaining = quantity;
        let mut changes = Vec::new();
        for slot in slots {
            if remaining == 0 {
                break;
            }
            let taken = remaining.min(self.map(kind)[&slot].quantity);
            changes.extend(self.take(kind, slot, taken)?);
            remaining -= taken;
        }
        Ok(changes)
    }

    /// Take `quantity` from one slot (e.g. using an item).
    pub fn take(
        &mut self,
        kind: InventoryType,
        slot: i16,
        quantity: i16,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        let item = self
            .map_mut(kind)
            .get_mut(&slot)
            .ok_or(InventoryError::InvalidSlot)?;
        if quantity < 1 || item.quantity < quantity {
            return Err(InventoryError::NotEnough);
        }
        item.quantity -= quantity;
        if item.quantity == 0 {
            self.map_mut(kind).remove(&slot);
            Ok(vec![InventoryChange::Remove { kind, slot }])
        } else {
            Ok(vec![InventoryChange::Quantity {
                kind,
                slot,
                quantity: item.quantity,
            }])
        }
    }

    /// Put an item in the first empty slot of its bag, as it is: a set of
    /// throwing stars or bullets never merges with another.
    pub fn add_alone(&mut self, item: Item) -> Result<Vec<InventoryChange>, InventoryError> {
        let kind = InventoryType::for_item(item.item_id).ok_or(InventoryError::InvalidSlot)?;
        let slot = self.free_slots(kind).next().ok_or(InventoryError::Full)?;
        self.map_mut(kind).insert(slot, item.clone());
        Ok(vec![InventoryChange::Add { kind, slot, item }])
    }

    /// Set how many one slot holds (recharging throwing stars).
    pub fn refill(
        &mut self,
        kind: InventoryType,
        slot: i16,
        quantity: i16,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        let item = self
            .map_mut(kind)
            .get_mut(&slot)
            .ok_or(InventoryError::InvalidSlot)?;
        if quantity < 1 {
            return Err(InventoryError::NotEnough);
        }
        item.quantity = quantity;
        Ok(vec![InventoryChange::Quantity {
            kind,
            slot,
            quantity,
        }])
    }

    /// Put a whole item in its bag at `slot` if that slot is free, else in
    /// the first free slot, without stacking it (an item taken out of the
    /// Cash Inventory).
    pub fn place(&mut self, item: Item, slot: i16) -> Result<(InventoryType, i16), InventoryError> {
        let kind = InventoryType::for_item(item.item_id).ok_or(InventoryError::InvalidSlot)?;
        let slot = if self.in_bag(kind, slot) && !self.map(kind).contains_key(&slot) {
            slot
        } else {
            self.free_slots(kind).next().ok_or(InventoryError::Full)?
        };
        self.map_mut(kind).insert(slot, item);
        Ok((kind, slot))
    }

    /// Take the item with cash serial `serial` out of `kind`, with its slot.
    pub fn pull_cash_item(&mut self, kind: InventoryType, serial: i64) -> Option<(i16, Item)> {
        kind.bag_index()?;
        let slot = self
            .items(kind)
            .find(|(_, item)| item.cash_serial == Some(serial))
            .map(|(slot, _)| slot)?;
        self.map_mut(kind).remove(&slot).map(|item| (slot, item))
    }

    /// Move an item within a bag: into an empty slot, onto a stack of the
    /// same item (up to `slot_max`), or swapping with another item.
    pub fn move_in_bag(
        &mut self,
        kind: InventoryType,
        from: i16,
        to: i16,
        slot_max: i16,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        if kind == InventoryType::Equipped || from == to || !self.in_bag(kind, to) {
            return Err(InventoryError::InvalidSlot);
        }
        let moving = self
            .map(kind)
            .get(&from)
            .ok_or(InventoryError::InvalidSlot)?
            .clone();
        if let Some(target) = self.map(kind).get(&to) {
            let room = slot_max - target.quantity;
            if Self::stacks_with(target, &moving) && room > 0 {
                let moved = room.min(moving.quantity);
                let target_quantity = target.quantity + moved;
                self.map_mut(kind).get_mut(&to).expect("checked").quantity = target_quantity;
                let mut changes = vec![InventoryChange::Quantity {
                    kind,
                    slot: to,
                    quantity: target_quantity,
                }];
                changes.extend(self.take(kind, from, moved)?);
                return Ok(changes);
            }
        }
        let map = self.map_mut(kind);
        let moving = map.remove(&from).expect("checked");
        if let Some(target) = map.remove(&to) {
            map.insert(from, target);
        }
        map.insert(to, moving);
        Ok(vec![InventoryChange::Move { kind, from, to }])
    }

    /// Wear the equipment at `from` (equip bag) at `to` (a negative
    /// position), swapping out what was worn there. Wearing an overall or a
    /// two-handed weapon also takes off the pants or shield, and vice versa;
    /// those need free slots.
    pub fn equip(&mut self, from: i16, to: i16) -> Result<Vec<InventoryChange>, InventoryError> {
        let item = self
            .get(InventoryType::Equip, from)
            .ok_or(InventoryError::InvalidSlot)?
            .clone();
        if item.equip.is_none() || !equip_slot::fits(item.item_id, item.cash_serial.is_some(), to) {
            return Err(InventoryError::CannotEquip);
        }
        let worn = |position: i16| self.equipped.get(&position).map(|i| i.item_id);
        let conflict = match to {
            equip_slot::TOP if equip_slot::is_overall(item.item_id) => {
                worn(equip_slot::BOTTOM).map(|_| equip_slot::BOTTOM)
            }
            equip_slot::BOTTOM => worn(equip_slot::TOP)
                .filter(|&id| equip_slot::is_overall(id))
                .map(|_| equip_slot::TOP),
            equip_slot::WEAPON if equip_slot::is_two_handed(item.item_id) => {
                worn(equip_slot::SHIELD).map(|_| equip_slot::SHIELD)
            }
            equip_slot::SHIELD => worn(equip_slot::WEAPON)
                .filter(|&id| equip_slot::is_two_handed(id))
                .map(|_| equip_slot::WEAPON),
            _ => None,
        };
        let mut changes = Vec::new();
        if let Some(position) = conflict {
            let free = self
                .free_slots(InventoryType::Equip)
                .find(|&slot| slot != from)
                .ok_or(InventoryError::Full)?;
            changes.extend(self.unequip(position, free)?);
        }
        let worn_item = self.equipped.remove(&to);
        let wearing = self.bags[0].remove(&from).expect("checked");
        if let Some(previous) = worn_item {
            self.bags[0].insert(from, previous);
        }
        self.equipped.insert(to, wearing);
        changes.push(InventoryChange::Move {
            kind: InventoryType::Equip,
            from,
            to,
        });
        Ok(changes)
    }

    /// Take off the equipment at `from` (negative) into the empty equip-bag
    /// slot `to`.
    pub fn unequip(&mut self, from: i16, to: i16) -> Result<Vec<InventoryChange>, InventoryError> {
        if !self.in_bag(InventoryType::Equip, to) || self.bags[0].contains_key(&to) {
            return Err(InventoryError::InvalidSlot);
        }
        let item = self
            .equipped
            .remove(&from)
            .ok_or(InventoryError::InvalidSlot)?;
        self.bags[0].insert(to, item);
        Ok(vec![InventoryChange::Move {
            kind: InventoryType::Equip,
            from,
            to,
        }])
    }

    /// Move a bag's items up into its empty slots, keeping their order (the
    /// item window's Gather).
    pub fn gather(&mut self, kind: InventoryType) -> Result<Vec<InventoryChange>, InventoryError> {
        self.rearrange(kind, |_| ())
    }

    /// Order a bag's items by item id from the first slot (the item window's
    /// Sort). Stacks are not merged.
    pub fn sort(&mut self, kind: InventoryType) -> Result<Vec<InventoryChange>, InventoryError> {
        self.rearrange(kind, |item| item.item_id)
    }

    /// Lay a bag's items out from slot 1, ordered by `key` and then by slot,
    /// as the swaps (`Move`s) that get there.
    fn rearrange<K: Ord>(
        &mut self,
        kind: InventoryType,
        key: impl Fn(&Item) -> K,
    ) -> Result<Vec<InventoryChange>, InventoryError> {
        if kind.bag_index().is_none() {
            return Err(InventoryError::InvalidSlot);
        }
        let mut order: Vec<i16> = self.map(kind).keys().copied().collect();
        order.sort_by_key(|slot| key(&self.map(kind)[slot]));

        // Where each of the original slots' items is now, and the reverse.
        let mut now: BTreeMap<i16, i16> = order.iter().map(|&slot| (slot, slot)).collect();
        let mut origin: BTreeMap<i16, i16> = now.clone();
        let mut changes = Vec::new();
        for (target, original) in (1..).zip(order) {
            let from = now[&original];
            if from == target {
                continue;
            }
            if let Some(displaced) = origin.remove(&target) {
                now.insert(displaced, from);
                origin.insert(from, displaced);
            } else {
                origin.remove(&from);
            }
            now.insert(original, target);
            origin.insert(target, original);
            changes.push(InventoryChange::Move {
                kind,
                from,
                to: target,
            });
        }

        let mut items = std::mem::take(self.map_mut(kind));
        *self.map_mut(kind) = origin
            .into_iter()
            .map(|(slot, original)| (slot, items.remove(&original).expect("tracked")))
            .collect();
        Ok(changes)
    }

    fn in_bag(&self, kind: InventoryType, slot: i16) -> bool {
        slot >= 1 && slot <= i16::from(self.slot_limit(kind))
    }

    fn free_slots(&self, kind: InventoryType) -> impl Iterator<Item = i16> + '_ {
        let limit = i16::from(self.slot_limit(kind));
        (1..=limit).filter(move |slot| !self.map(kind).contains_key(slot))
    }

    fn stacks_with(existing: &Item, item: &Item) -> bool {
        existing.item_id == item.item_id
            && existing.equip.is_none()
            && existing.expiration.is_none()
            && existing.cash_serial.is_none()
            && existing.owner == item.owner
            && existing.flags == item.flags
    }

    fn map(&self, kind: InventoryType) -> &BTreeMap<i16, Item> {
        match kind.bag_index() {
            Some(index) => &self.bags[index],
            None => &self.equipped,
        }
    }

    fn map_mut(&mut self, kind: InventoryType) -> &mut BTreeMap<i16, Item> {
        match kind.bag_index() {
            Some(index) => &mut self.bags[index],
            None => &mut self.equipped,
        }
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
    fn karma_is_used_up_once() {
        let mut earrings = Item {
            flags: FLAG_EQUIP_KARMA,
            ..Item::stack(1032031, 1)
        };
        assert!(earrings.has_karma());
        earrings.use_karma();
        assert!(!earrings.has_karma());
        assert_eq!(earrings.flags, FLAG_UNTRADEABLE);

        let mut bundle = Item {
            flags: FLAG_BUNDLE_KARMA | 0x01,
            ..Item::stack(2210005, 3)
        };
        assert!(bundle.has_karma());
        bundle.use_karma();
        assert_eq!(bundle.flags, 0x01);

        // Each kind has its own bit.
        let other = Item {
            flags: FLAG_EQUIP_KARMA,
            ..Item::stack(2210005, 3)
        };
        assert!(!other.has_karma());
    }

    #[test]
    fn cash_items_are_placed_whole_and_pulled_by_serial() {
        let mut inventory = Inventory::default();
        let tickets = Item {
            cash_serial: Some(9),
            ..Item::stack(5220000, 11)
        };
        assert_eq!(
            inventory.place(tickets.clone(), 3),
            Ok((InventoryType::Cash, 3))
        );
        // Slot 3 is taken and 0 is out of the bag, so the first free slot.
        assert_eq!(
            inventory.place(Item::stack(5220000, 1), 3),
            Ok((InventoryType::Cash, 1))
        );
        assert_eq!(
            inventory.place(Item::stack(5220000, 1), 0),
            Ok((InventoryType::Cash, 2))
        );
        assert_eq!(inventory.get(InventoryType::Cash, 3), Some(&tickets));

        assert_eq!(inventory.pull_cash_item(InventoryType::Use, 9), None);
        assert_eq!(
            inventory.pull_cash_item(InventoryType::Cash, 9),
            Some((3, tickets))
        );
        assert_eq!(inventory.pull_cash_item(InventoryType::Cash, 9), None);

        let mut full = Inventory::default();
        full.set_slot_limit(InventoryType::Cash, 1);
        full.place(Item::stack(5220000, 1), 1).unwrap();
        assert_eq!(
            full.place(Item::stack(5220000, 1), 1),
            Err(InventoryError::Full)
        );
    }

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

    fn equip(id: i32) -> Item {
        Item::equipment(id, EquipStats::default())
    }

    #[test]
    fn adding_fills_partial_stacks_then_empty_slots() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Use, 2, Item::stack(2010000, 98));
        let changes = inventory.add(Item::stack(2010000, 5), 100).unwrap();
        assert_eq!(
            changes,
            [
                InventoryChange::Quantity {
                    kind: InventoryType::Use,
                    slot: 2,
                    quantity: 100
                },
                InventoryChange::Add {
                    kind: InventoryType::Use,
                    slot: 1,
                    item: Item::stack(2010000, 3)
                },
            ]
        );
        assert_eq!(inventory.count(2010000), 103);
    }

    #[test]
    fn a_full_bag_changes_nothing() {
        let mut inventory = Inventory::default();
        inventory.set_slot_limit(InventoryType::Etc, 1);
        inventory.insert(InventoryType::Etc, 1, Item::stack(4000019, 199));
        assert!(inventory.add(Item::stack(4000019, 1), 200).is_ok());
        let before = inventory.clone();
        assert_eq!(
            inventory.add(Item::stack(4000019, 1), 200),
            Err(InventoryError::Full)
        );
        assert_eq!(
            inventory.add(Item::stack(4031802, 1), 100),
            Err(InventoryError::Full)
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn star_sets_take_their_own_slot_and_refill_in_place() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Use, 1, Item::stack(2070000, 120));
        let changes = inventory.add_alone(Item::stack(2070000, 500)).unwrap();
        assert_eq!(
            changes,
            [InventoryChange::Add {
                kind: InventoryType::Use,
                slot: 2,
                item: Item::stack(2070000, 500)
            }]
        );
        assert_eq!(
            inventory.refill(InventoryType::Use, 1, 500).unwrap(),
            [InventoryChange::Quantity {
                kind: InventoryType::Use,
                slot: 1,
                quantity: 500
            }]
        );
        assert_eq!(inventory.count(2070000), 1000);
        assert_eq!(
            inventory.refill(InventoryType::Use, 3, 500),
            Err(InventoryError::InvalidSlot)
        );

        inventory.set_slot_limit(InventoryType::Use, 2);
        let before = inventory.clone();
        assert_eq!(
            inventory.add_alone(Item::stack(2070000, 500)),
            Err(InventoryError::Full)
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn removing_takes_from_the_lowest_slots() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Etc, 1, Item::stack(4031792, 4));
        inventory.insert(InventoryType::Etc, 3, Item::stack(4031792, 10));
        assert_eq!(
            inventory.remove(4031792, 20),
            Err(InventoryError::NotEnough)
        );
        let changes = inventory.remove(4031792, 10).unwrap();
        assert_eq!(
            changes,
            [
                InventoryChange::Remove {
                    kind: InventoryType::Etc,
                    slot: 1
                },
                InventoryChange::Quantity {
                    kind: InventoryType::Etc,
                    slot: 3,
                    quantity: 4
                },
            ]
        );
    }

    #[test]
    fn moving_merges_stacks_or_swaps() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Use, 1, Item::stack(2000000, 60));
        inventory.insert(InventoryType::Use, 2, Item::stack(2000000, 60));
        inventory.insert(InventoryType::Use, 3, Item::stack(2000003, 1));
        let merged = inventory
            .move_in_bag(InventoryType::Use, 1, 2, 100)
            .unwrap();
        assert_eq!(
            merged[0],
            InventoryChange::Quantity {
                kind: InventoryType::Use,
                slot: 2,
                quantity: 100
            }
        );
        assert_eq!(inventory.get(InventoryType::Use, 1).unwrap().quantity, 20);
        inventory
            .move_in_bag(InventoryType::Use, 1, 3, 100)
            .unwrap();
        assert_eq!(
            inventory.get(InventoryType::Use, 1).unwrap().item_id,
            2000003
        );
        assert_eq!(
            inventory.get(InventoryType::Use, 3).unwrap().item_id,
            2000000
        );
        assert_eq!(
            inventory.move_in_bag(InventoryType::Use, 1, 99, 100),
            Err(InventoryError::InvalidSlot)
        );
    }

    #[test]
    fn equipping_swaps_with_what_is_worn() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Equipped, equip_slot::CAP, equip(1002067));
        inventory.insert(InventoryType::Equip, 4, equip(1002008));
        inventory.equip(4, equip_slot::CAP).unwrap();
        assert_eq!(
            inventory
                .get(InventoryType::Equipped, equip_slot::CAP)
                .unwrap()
                .item_id,
            1002008
        );
        assert_eq!(
            inventory.get(InventoryType::Equip, 4).unwrap().item_id,
            1002067
        );
        assert_eq!(
            inventory.equip(4, equip_slot::WEAPON),
            Err(InventoryError::CannotEquip)
        );
        inventory.unequip(equip_slot::CAP, 1).unwrap();
        assert!(inventory
            .get(InventoryType::Equipped, equip_slot::CAP)
            .is_none());
        assert_eq!(
            inventory.unequip(equip_slot::CAP, 2),
            Err(InventoryError::InvalidSlot)
        );
    }

    #[test]
    fn cash_equips_are_worn_a_hundred_below_their_slot() {
        let mut inventory = Inventory::default();
        let mut cash_cap = equip(1002186);
        cash_cap.cash_serial = Some(7);
        inventory.insert(InventoryType::Equip, 1, cash_cap);
        inventory.insert(InventoryType::Equip, 2, equip(1002008));
        assert_eq!(
            inventory.equip(1, equip_slot::CAP),
            Err(InventoryError::CannotEquip)
        );
        assert_eq!(
            inventory.equip(2, equip_slot::CAP - 100),
            Err(InventoryError::CannotEquip)
        );
        inventory.equip(1, equip_slot::CAP - 100).unwrap();
        inventory.equip(2, equip_slot::CAP).unwrap();
        assert_eq!(inventory.equipped_regular().count(), 1);
        assert_eq!(inventory.equipped_cash().count(), 1);
    }

    #[test]
    fn pet_equips_go_in_their_pets_slots() {
        // Wings for the second pet, a meso magnet for the third, and an HP
        // pouch, which every pet shares.
        for (item_id, position, index) in [
            (1802063, -130, Some(1)),
            (1812000, -142, Some(2)),
            (1812002, -124, None),
        ] {
            assert!(equip_slot::fits(item_id, true, position), "{item_id}");
            assert_eq!(equip_slot::pet_index(position + 100), index);
        }
        assert!(!equip_slot::fits(1802063, true, -129), "a quote ring's");
        assert!(!equip_slot::fits(1812002, true, -133));
        assert!(!equip_slot::fits(1812008, true, -128), "no such ability");
        assert!(!equip_slot::fits(1832000, true, equip_slot::CAP - 100));
        assert!(equip_slot::fits(1832000, true, -132));
        assert!(
            equip_slot::fits(1802100, true, -139),
            "the collar's a label"
        );
    }

    #[test]
    fn overalls_and_pants_take_each_other_off() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Equipped, equip_slot::TOP, equip(1040002));
        inventory.insert(InventoryType::Equipped, equip_slot::BOTTOM, equip(1060002));
        inventory.insert(InventoryType::Equip, 1, equip(1050000));
        let changes = inventory.equip(1, equip_slot::TOP).unwrap();
        assert_eq!(changes.len(), 2, "pants come off first");
        assert!(inventory
            .get(InventoryType::Equipped, equip_slot::BOTTOM)
            .is_none());
        assert_eq!(
            inventory.count(1060002),
            1,
            "the pants went to the equip bag"
        );
        let pants_slot = inventory
            .items(InventoryType::Equip)
            .find(|(_, item)| item.item_id == 1060002)
            .map(|(slot, _)| slot)
            .unwrap();
        inventory.equip(pants_slot, equip_slot::BOTTOM).unwrap();
        assert!(inventory
            .get(InventoryType::Equipped, equip_slot::TOP)
            .is_none());
    }

    #[test]
    fn a_shield_takes_off_a_claw() {
        let mut inventory = Inventory::default();
        inventory.insert(InventoryType::Equipped, equip_slot::WEAPON, equip(1472000));
        inventory.insert(InventoryType::Equip, 1, equip(1092000));
        inventory.equip(1, equip_slot::SHIELD).unwrap();
        assert!(inventory
            .get(InventoryType::Equipped, equip_slot::WEAPON)
            .is_none());
        assert_eq!(inventory.count(1472000), 1);
    }

    /// Item ids by slot in `kind`.
    fn layout(inventory: &Inventory, kind: InventoryType) -> Vec<(i16, i32)> {
        inventory
            .items(kind)
            .map(|(slot, item)| (slot, item.item_id))
            .collect()
    }

    /// Apply `Move`s as the client does: swap the two slots.
    fn replay(mut slots: BTreeMap<i16, i32>, changes: &[InventoryChange]) -> Vec<(i16, i32)> {
        for change in changes {
            let InventoryChange::Move { from, to, .. } = *change else {
                panic!("only moves: {change:?}");
            };
            let moving = slots.remove(&from);
            if let Some(target) = slots.remove(&to) {
                slots.insert(from, target);
            }
            if let Some(moving) = moving {
                slots.insert(to, moving);
            }
        }
        slots.into_iter().collect()
    }

    fn scattered() -> Inventory {
        let mut inventory = Inventory::default();
        for (slot, item_id, quantity) in [
            (2, 4000019, 30),
            (5, 4000000, 12),
            (6, 4000019, 5),
            (9, 4000001, 7),
            (24, 4000000, 1),
        ] {
            inventory.insert(InventoryType::Etc, slot, Item::stack(item_id, quantity));
        }
        inventory
    }

    #[test]
    fn gather_fills_the_gaps_in_order() {
        let mut inventory = scattered();
        let before: BTreeMap<_, _> = layout(&inventory, InventoryType::Etc).into_iter().collect();
        let changes = inventory.gather(InventoryType::Etc).unwrap();
        let expected = vec![
            (1, 4000019),
            (2, 4000000),
            (3, 4000019),
            (4, 4000001),
            (5, 4000000),
        ];
        assert_eq!(layout(&inventory, InventoryType::Etc), expected);
        assert_eq!(replay(before, &changes), expected);
        assert_eq!(
            inventory.get(InventoryType::Etc, 3).unwrap().quantity,
            5,
            "stacks keep their quantities"
        );
        assert!(inventory.gather(InventoryType::Etc).unwrap().is_empty());
    }

    #[test]
    fn sort_orders_by_item_id_without_merging() {
        let mut inventory = scattered();
        let before: BTreeMap<_, _> = layout(&inventory, InventoryType::Etc).into_iter().collect();
        let changes = inventory.sort(InventoryType::Etc).unwrap();
        let expected = vec![
            (1, 4000000),
            (2, 4000000),
            (3, 4000001),
            (4, 4000019),
            (5, 4000019),
        ];
        assert_eq!(layout(&inventory, InventoryType::Etc), expected);
        assert_eq!(replay(before, &changes), expected);
        let quantities: Vec<_> = inventory
            .items(InventoryType::Etc)
            .map(|(_, item)| item.quantity)
            .collect();
        assert_eq!(
            quantities,
            vec![12, 1, 7, 30, 5],
            "equal ids keep their slot order"
        );
        assert!(inventory.sort(InventoryType::Etc).unwrap().is_empty());
    }

    #[test]
    fn worn_equipment_cannot_be_rearranged() {
        let mut inventory = Inventory::default();
        assert_eq!(
            inventory.sort(InventoryType::Equipped),
            Err(InventoryError::InvalidSlot)
        );
    }
}
