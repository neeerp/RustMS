//! An account's storage (the trunk a storage NPC keeps): mesos and items
//! shared by all its characters.

use crate::inventory::{InventoryType, Item};

/// Slots a new account's storage has; the Cash Shop adds more, up to 48.
pub const DEFAULT_STORAGE_SLOTS: u8 = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Storage {
    pub slots: u8,
    pub meso: i32,
    /// In the order they were stored, or sorted. The client lists each
    /// bag's items in this order and takes one out by its place there.
    items: Vec<Item>,
}

impl Storage {
    pub fn new(slots: u8, meso: i32, items: Vec<Item>) -> Self {
        Self { slots, meso, items }
    }

    pub fn all(&self) -> &[Item] {
        &self.items
    }

    /// The items of one bag, in order.
    pub fn items(&self, kind: InventoryType) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .filter(move |item| InventoryType::for_item(item.item_id) == Some(kind))
    }

    pub fn is_full(&self) -> bool {
        self.items.len() >= usize::from(self.slots)
    }

    /// Store an item after the others; `false`, changing nothing, when every
    /// slot is taken.
    pub fn store(&mut self, item: Item) -> bool {
        if self.is_full() {
            return false;
        }
        self.items.push(item);
        true
    }

    /// The `index`th item of a bag.
    pub fn get(&self, kind: InventoryType, index: usize) -> Option<&Item> {
        self.items(kind).nth(index)
    }

    /// Take out the `index`th item of a bag.
    pub fn take(&mut self, kind: InventoryType, index: usize) -> Option<Item> {
        let position = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| InventoryType::for_item(item.item_id) == Some(kind))
            .nth(index)?
            .0;
        Some(self.items.remove(position))
    }

    /// Order the items by id, keeping items of one id in their order.
    pub fn sort(&mut self) {
        self.items.sort_by_key(|item| item.item_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage::new(
            4,
            0,
            vec![
                Item::stack(4000000, 5),
                Item::stack(2000000, 1),
                Item::stack(4000001, 2),
            ],
        )
    }

    #[test]
    fn a_bags_items_are_counted_in_the_order_stored() {
        let mut storage = storage();
        assert_eq!(storage.get(InventoryType::Etc, 1).unwrap().item_id, 4000001);
        assert_eq!(
            storage.take(InventoryType::Etc, 1).unwrap().item_id,
            4000001
        );
        assert_eq!(storage.take(InventoryType::Etc, 1), None);
        assert_eq!(storage.all().len(), 2);
    }

    #[test]
    fn a_full_storage_takes_nothing() {
        let mut storage = storage();
        assert!(storage.store(Item::stack(4000002, 1)));
        assert!(storage.is_full());
        assert!(!storage.store(Item::stack(4000003, 1)));
        assert_eq!(storage.all().len(), 4);
    }

    #[test]
    fn sorting_orders_by_id() {
        let mut storage = storage();
        storage.store(Item::stack(4000000, 9));
        storage.sort();
        let order: Vec<_> = storage
            .all()
            .iter()
            .map(|i| (i.item_id, i.quantity))
            .collect();
        assert_eq!(
            order,
            [(2000000, 1), (4000000, 5), (4000000, 9), (4000001, 2)]
        );
    }
}
