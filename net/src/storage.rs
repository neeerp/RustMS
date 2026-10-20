//! Storage: the account-wide trunk a storage NPC (one with `info/trunkPut`)
//! opens, and what taking out, storing, sorting, and moving mesos
//! (TRUNK_REQUEST) do. The runtime loads the storage, holds it while open,
//! and saves it with the player after every change.

use crate::error::NetworkError;
use crate::packet::build::world::{
    inventory,
    stat::{self, StatUpdate},
    storage::{build_storage_contents, build_storage_result, Tabs},
};
use crate::shop::is_rechargeable;
use crate::{items, npc_data};
use game::inventory::FLAG_UNTRADEABLE;
use game::{InventoryType, Player, Storage};
use game_data::StorageFees;
use packet::Packet;

/// TRUNK_RESULT modes, as the v83 client reads them.
pub mod result {
    pub const TAKEN_OUT: u8 = 0x09;
    /// "Please check if your inventory is full or not."
    pub const INVENTORY_FULL: u8 = 0x0a;
    pub const TAKE_OUT_NO_MESOS: u8 = 0x0b;
    /// The player already has the one-of-a-kind item.
    pub const ONLY_ONE: u8 = 0x0c;
    pub const STORED: u8 = 0x0d;
    pub const SORTED: u8 = 0x0f;
    pub const STORE_NO_MESOS: u8 = 0x10;
    pub const STORAGE_FULL: u8 = 0x11;
    /// Any mode the client doesn't know: "Due to an error, the trade did
    /// not happen."
    pub const ERROR: u8 = 0x12;
    pub const MESO: u8 = 0x13;
    pub const OPEN: u8 = 0x16;
}

/// What the client asked of the open storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageRequest {
    /// The `index`th item listed in the storage's `tab` (an inventory type).
    TakeOut {
        tab: u8,
        index: u8,
    },
    /// `quantity` of the item in `slot` of its bag.
    Store {
        slot: i16,
        item_id: i32,
        quantity: i16,
    },
    Sort,
    /// Mesos to take out, or store if negative.
    Meso(i32),
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenStorage {
    pub npc_id: i32,
    pub fees: StorageFees,
    pub storage: Storage,
}

/// The fees if `npc_id` keeps a storage.
pub fn fees(npc_id: i32) -> Option<StorageFees> {
    npc_data::get().ok()?.storage_fees(npc_id)
}

/// Whether the item may go into storage. The client lists neither cash
/// items nor quest or untradeable ones, unless Scissors of Karma were used
/// on them (FUN_007c6e3a).
pub fn storable(item: &game::Item) -> bool {
    if item.is_cash() {
        return false;
    }
    let bound = item.equip.is_some() && item.flags & FLAG_UNTRADEABLE != 0;
    items::info(item.item_id).is_some_and(|info| {
        !info.equip.is_some_and(|equip| equip.cash)
            && (item.has_karma() || !info.quest && !info.trade_block && !bound)
    })
}

/// The replies to a request, and whether the storage or the player changed
/// and must be saved.
pub struct Outcome {
    pub packets: Vec<Packet>,
    pub changed: bool,
}

impl Outcome {
    fn refused(mode: u8) -> Result<Self, NetworkError> {
        Ok(Self {
            packets: vec![build_storage_result(mode)?],
            changed: false,
        })
    }
}

impl OpenStorage {
    /// Take out an item, for the NPC's `trunkGet` fee.
    fn take_out(
        &mut self,
        player: &mut Player,
        tab: u8,
        index: u8,
    ) -> Result<Outcome, NetworkError> {
        let Some(kind) =
            InventoryType::from_i16(i16::from(tab)).filter(|&kind| kind != InventoryType::Equipped)
        else {
            return Outcome::refused(result::ERROR);
        };
        let Some(mut item) = self.storage.get(kind, usize::from(index)).cloned() else {
            return Outcome::refused(result::ERROR);
        };
        // The client warns that taking an item out uses up its Karma
        // (FUN_007c83af).
        item.use_karma();
        if player.meso < self.fees.get {
            return Outcome::refused(result::TAKE_OUT_NO_MESOS);
        }
        let only = items::info(item.item_id).is_some_and(|info| info.only);
        if only && player.inventory.count(item.item_id) > 0 {
            return Outcome::refused(result::ONLY_ONE);
        }
        let added = if is_rechargeable(item.item_id) || item.equip.is_some() {
            player.inventory_mut().add_alone(item)
        } else {
            let slot_max = items::slot_max(item.item_id);
            player.inventory_mut().add(item, slot_max)
        };
        let Ok(changes) = added else {
            return Outcome::refused(result::INVENTORY_FULL);
        };
        self.storage.take(kind, usize::from(index));
        player.meso -= self.fees.get;
        player.mark_dirty();
        Ok(Outcome {
            packets: vec![
                inventory::build_inventory_operation(true, &changes)?,
                stat::build_stat_update(true, &[StatUpdate::Meso(player.meso)])?,
                build_storage_contents(result::TAKEN_OUT, &self.storage, Tabs::Bag(kind))?,
            ],
            changed: true,
        })
    }

    /// Store `quantity` from a bag slot, for the NPC's `trunkPut` fee. A set
    /// of stars or bullets goes whole.
    fn store(
        &mut self,
        player: &mut Player,
        slot: i16,
        item_id: i32,
        quantity: i16,
    ) -> Result<Outcome, NetworkError> {
        let Some(kind) = InventoryType::for_item(item_id) else {
            return Outcome::refused(result::ERROR);
        };
        let Some(item) = player
            .inventory
            .get(kind, slot)
            .filter(|item| item.item_id == item_id && storable(item))
        else {
            return Outcome::refused(result::ERROR);
        };
        let quantity = if is_rechargeable(item_id) || item.equip.is_some() {
            item.quantity
        } else if (1..=item.quantity).contains(&quantity) {
            quantity
        } else {
            return Outcome::refused(result::ERROR);
        };
        if self.storage.is_full() {
            return Outcome::refused(result::STORAGE_FULL);
        }
        if player.meso < self.fees.put {
            return Outcome::refused(result::STORE_NO_MESOS);
        }
        let stored = game::Item {
            quantity,
            ..item.clone()
        };
        let Ok(changes) = player.inventory_mut().take(kind, slot, quantity) else {
            return Outcome::refused(result::ERROR);
        };
        self.storage.store(stored);
        player.meso -= self.fees.put;
        player.mark_dirty();
        Ok(Outcome {
            packets: vec![
                inventory::build_inventory_operation(true, &changes)?,
                stat::build_stat_update(true, &[StatUpdate::Meso(player.meso)])?,
                build_storage_contents(result::STORED, &self.storage, Tabs::Bag(kind))?,
            ],
            changed: true,
        })
    }

    /// Take out `amount` mesos, or store `-amount`.
    fn meso(&mut self, player: &mut Player, amount: i32) -> Result<Outcome, NetworkError> {
        let carried = i64::from(player.meso) + i64::from(amount);
        let kept = i64::from(self.storage.meso) - i64::from(amount);
        let (Ok(carried), Ok(kept)) = (i32::try_from(carried), i32::try_from(kept)) else {
            return Outcome::refused(result::ERROR);
        };
        if amount == 0 || carried < 0 || kept < 0 {
            return Outcome::refused(result::ERROR);
        }
        player.meso = carried;
        player.mark_dirty();
        self.storage.meso = kept;
        Ok(Outcome {
            packets: vec![
                stat::build_stat_update(true, &[StatUpdate::Meso(player.meso)])?,
                build_storage_contents(result::MESO, &self.storage, Tabs::Meso)?,
            ],
            changed: true,
        })
    }

    /// Answer a request; `Close` is the runtime's to handle.
    pub fn respond(
        &mut self,
        player: &mut Player,
        request: StorageRequest,
    ) -> Result<Outcome, NetworkError> {
        match request {
            StorageRequest::TakeOut { tab, index } => self.take_out(player, tab, index),
            StorageRequest::Store {
                slot,
                item_id,
                quantity,
            } => self.store(player, slot, item_id, quantity),
            StorageRequest::Sort => {
                self.storage.sort();
                Ok(Outcome {
                    packets: vec![build_storage_contents(
                        result::SORTED,
                        &self.storage,
                        Tabs::All,
                    )?],
                    changed: true,
                })
            }
            StorageRequest::Meso(amount) => self.meso(player, amount),
            StorageRequest::Close => Ok(Outcome {
                packets: Vec::new(),
                changed: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Item;

    const RED_POTION: i32 = 2000000;
    const SUBI: i32 = 2070000;
    const SWORD: i32 = 1302000;

    fn open(items: Vec<Item>) -> OpenStorage {
        OpenStorage {
            npc_id: 1012009,
            fees: StorageFees { put: 100, get: 0 },
            storage: Storage::new(4, 1000, items),
        }
    }

    fn player(meso: i32) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        player
    }

    fn mode(outcome: &Outcome) -> u8 {
        outcome.packets.last().unwrap().bytes[2]
    }

    #[test]
    fn storing_takes_the_fee_and_the_items() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(150);
        items::give(player.inventory_mut(), RED_POTION, 30).unwrap();
        let mut open = open(Vec::new());
        let stored = open
            .respond(
                &mut player,
                StorageRequest::Store {
                    slot: 1,
                    item_id: RED_POTION,
                    quantity: 10,
                },
            )
            .unwrap();
        assert!(stored.changed);
        assert_eq!(mode(&stored), result::STORED);
        assert_eq!(player.meso, 50);
        assert_eq!(player.inventory.count(RED_POTION), 20);
        assert_eq!(
            open.storage.get(InventoryType::Use, 0).unwrap().quantity,
            10
        );

        let broke = open
            .respond(
                &mut player,
                StorageRequest::Store {
                    slot: 1,
                    item_id: RED_POTION,
                    quantity: 10,
                },
            )
            .unwrap();
        assert_eq!(mode(&broke), result::STORE_NO_MESOS);
        assert_eq!(player.inventory.count(RED_POTION), 20);
        let wrong = open
            .respond(
                &mut player,
                StorageRequest::Store {
                    slot: 1,
                    item_id: RED_POTION,
                    quantity: 21,
                },
            )
            .unwrap();
        assert_eq!(mode(&wrong), result::ERROR);
    }

    #[test]
    fn a_full_storage_refuses_and_stars_go_whole() {
        if items::data().is_err() {
            return;
        }
        let mut player = player(1000);
        player
            .inventory_mut()
            .add_alone(Item::stack(SUBI, 400))
            .unwrap();
        let mut open = open(Vec::new());
        open.respond(
            &mut player,
            StorageRequest::Store {
                slot: 1,
                item_id: SUBI,
                quantity: 1,
            },
        )
        .unwrap();
        assert_eq!(
            open.storage.get(InventoryType::Use, 0).unwrap().quantity,
            400
        );
        assert_eq!(player.inventory.count(SUBI), 0);

        let mut full = OpenStorage {
            storage: Storage::new(1, 0, vec![Item::stack(4000000, 1)]),
            ..open
        };
        items::give(player.inventory_mut(), RED_POTION, 1).unwrap();
        let refused = full
            .respond(
                &mut player,
                StorageRequest::Store {
                    slot: 1,
                    item_id: RED_POTION,
                    quantity: 1,
                },
            )
            .unwrap();
        assert_eq!(mode(&refused), result::STORAGE_FULL);
    }

    #[test]
    fn taking_out_puts_the_item_back_with_its_stats() {
        if items::data().is_err() {
            return;
        }
        let sword = items::create(SWORD, 1).unwrap();
        let mut open = OpenStorage {
            fees: StorageFees {
                put: 500,
                get: 1000,
            },
            ..open(vec![Item::stack(RED_POTION, 5), sword.clone()])
        };
        let mut player = player(999);
        let broke = open
            .respond(&mut player, StorageRequest::TakeOut { tab: 1, index: 0 })
            .unwrap();
        assert_eq!(mode(&broke), result::TAKE_OUT_NO_MESOS);

        player.meso = 1500;
        let taken = open
            .respond(&mut player, StorageRequest::TakeOut { tab: 1, index: 0 })
            .unwrap();
        assert_eq!(mode(&taken), result::TAKEN_OUT);
        assert_eq!(player.meso, 500);
        let (_, carried) = player.inventory.items(InventoryType::Equip).next().unwrap();
        assert_eq!(carried, &sword);
        assert_eq!(open.storage.all().len(), 1);

        player.inventory.set_slot_limit(InventoryType::Use, 0);
        player.meso = 1000;
        let full = open
            .respond(&mut player, StorageRequest::TakeOut { tab: 2, index: 0 })
            .unwrap();
        assert_eq!(mode(&full), result::INVENTORY_FULL);
        assert_eq!(open.storage.all().len(), 1);
    }

    #[test]
    fn karma_lets_untradeable_items_in_and_is_used_up_coming_out() {
        if items::data().is_err() {
            return;
        }
        // Tigun Transformation Bundle, a quest item that can't be traded.
        const TIGUN: i32 = 2210005;
        let karma = Item {
            flags: game::inventory::FLAG_BUNDLE_KARMA,
            ..Item::stack(TIGUN, 3)
        };
        assert!(!storable(&Item::stack(TIGUN, 3)) && storable(&karma));
        let bound = Item {
            flags: FLAG_UNTRADEABLE,
            ..items::create(SWORD, 1).unwrap()
        };
        assert!(!storable(&bound));

        let mut player = player(100);
        player.inventory.insert(InventoryType::Use, 1, karma);
        let mut open = open(Vec::new());
        let stored = open
            .respond(
                &mut player,
                StorageRequest::Store {
                    slot: 1,
                    item_id: TIGUN,
                    quantity: 3,
                },
            )
            .unwrap();
        assert_eq!(mode(&stored), result::STORED);
        assert!(open.storage.get(InventoryType::Use, 0).unwrap().has_karma());
        let taken = open
            .respond(&mut player, StorageRequest::TakeOut { tab: 2, index: 0 })
            .unwrap();
        assert_eq!(mode(&taken), result::TAKEN_OUT);
        let (_, carried) = player.inventory.items(InventoryType::Use).next().unwrap();
        assert_eq!(carried.flags, 0);
        assert!(!storable(carried));
    }

    #[test]
    fn mesos_move_both_ways_within_what_there_is() {
        let mut open = open(Vec::new());
        let mut player = player(500);
        open.respond(&mut player, StorageRequest::Meso(-200))
            .unwrap();
        assert_eq!((player.meso, open.storage.meso), (300, 1200));
        open.respond(&mut player, StorageRequest::Meso(1200))
            .unwrap();
        assert_eq!((player.meso, open.storage.meso), (1500, 0));
        let refused = open.respond(&mut player, StorageRequest::Meso(1)).unwrap();
        assert_eq!(mode(&refused), result::ERROR);
        assert!(!refused.changed);
        let refused = open
            .respond(&mut player, StorageRequest::Meso(-1501))
            .unwrap();
        assert_eq!(mode(&refused), result::ERROR);
        assert_eq!((player.meso, open.storage.meso), (1500, 0));
    }
}
