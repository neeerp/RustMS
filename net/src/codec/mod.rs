//! Encoders for structures that recur across many v83 packets.
//!
//! Character stats, avatar looks, items, and the full character data block
//! appear in the character list, SET_FIELD, spawns, shops, trades, and the
//! cash shop. Builders must use these instead of writing the bytes by hand,
//! so every packet stays consistent with the reference layout (HeavenMS).

pub mod character;
pub mod item;
pub mod look;
pub mod stats;
pub mod time;
#[cfg(test)]
pub(crate) mod testing;

pub use character::encode_character_data;
pub use item::{encode_inventory, encode_item};
pub use look::encode_avatar_look;
pub use stats::encode_character_stats;
