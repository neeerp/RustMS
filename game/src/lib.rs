//! Game domain model.
//!
//! These types are the in-memory, authoritative representation of game
//! state. They deliberately know nothing about Diesel rows or packet layouts:
//! `db` converts rows to and from them and `net` encodes them for the wire.

pub mod character;
pub mod inventory;
pub mod keymap;
pub mod quest;
pub mod skill;

pub use character::{AvatarLook, CharacterStats, FieldAvatar, Player};
pub use inventory::{EquipStats, Inventory, InventoryType, Item};
pub use keymap::{KeyBinding, Keymap};
pub use quest::{QuestLog, QuestRecord, QuestStatus};
pub use skill::{SkillBook, SkillRecord};
