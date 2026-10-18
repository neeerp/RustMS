//! Game domain model.
//!
//! These types are the in-memory, authoritative representation of game
//! state. They deliberately know nothing about Diesel rows or packet layouts:
//! `db` converts rows to and from them and `net` encodes them for the wire.

pub mod buff;
pub mod character;
pub mod exp;
pub mod guild;
pub mod inventory;
pub mod keymap;
pub mod mob_damage;
pub mod mob_skill;
pub mod mob_stat;
pub mod monster_book;
pub mod quest;
pub mod scroll;
pub mod skill;
pub mod storage;

pub use buff::{Buff, BuffStat, Buffs, Disease, ShownBuff};
pub use character::{ApStat, AvatarLook, CharacterStats, FieldAvatar, MiniRoomBox, Player};
pub use guild::{Guild, GuildMember, GuildTag};
pub use inventory::{EquipStats, Inventory, InventoryChange, InventoryError, InventoryType, Item};
pub use keymap::{KeyBinding, Keymap};
pub use mob_stat::MobStat;
pub use monster_book::MonsterBook;
pub use quest::{QuestLog, QuestRecord, QuestState, QuestStatus};
pub use skill::{SkillBook, SkillRecord};
pub use storage::Storage;
