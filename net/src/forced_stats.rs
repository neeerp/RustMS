//! Forced stats: what FORCED_STAT_SET (0x22) pins a player's stats to while
//! they are on a map, and FORCED_STAT_RESET (0x23) lets go of. v83 keeps
//! them in `CWvsContext`'s ForcedStat (decoded by FUN_0077bf9a), so they
//! outlast a change of map until reset.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The thirteen stats v83 can pin, in its mask order; 0 leaves one alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForcedStats {
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub pad: i16,
    pub pdd: i16,
    pub mad: i16,
    pub mdd: i16,
    pub acc: i16,
    pub eva: i16,
    pub speed: u8,
    pub jump: u8,
    pub speed_max: u8,
}

/// The Black Road's Burning Forests (914000200, 210 and 220), where the
/// Aran of the tutorial fights at a hero's strength: 999 of each stat,
/// accuracy and avoid, 255 attack, speed 120 and jump 140 (Cosmic's
/// `aranGodlyStats`; nothing in the data says).
pub const BURNING_FOREST: ForcedStats = ForcedStats {
    str: 999,
    dex: 999,
    int: 999,
    luk: 999,
    pad: 255,
    pdd: 0,
    mad: 0,
    mdd: 0,
    acc: 999,
    eva: 999,
    speed: 120,
    jump: 140,
    speed_max: 0,
};

/// The stats a map pins, if it pins any.
pub fn on_map(map_id: i32) -> Option<ForcedStats> {
    match map_id {
        914000200 | 914000210 | 914000220 => Some(BURNING_FOREST),
        _ => None,
    }
}

impl ForcedStats {
    /// The ten two-byte stats, by their mask bits 0 to 9.
    fn shorts(&self) -> [i16; 10] {
        [
            self.str, self.dex, self.int, self.luk, self.pad, self.pdd, self.mad, self.mdd,
            self.acc, self.eva,
        ]
    }

    /// The three one-byte stats, by their mask bits 10 to 12.
    fn bytes(&self) -> [u8; 3] {
        [self.speed, self.jump, self.speed_max]
    }

    fn mask(&self) -> u32 {
        let shorts = self.shorts().into_iter().map(|value| value != 0);
        let bytes = self.bytes().into_iter().map(|value| value != 0);
        shorts
            .chain(bytes)
            .enumerate()
            .fold(0, |mask, (bit, set)| mask | u32::from(set) << bit)
    }
}

/// FORCED_STAT_SET: the mask, then each pinned stat in its order.
pub fn build_set(stats: &ForcedStats) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ForcedStatSet as i16)?;
    packet.write_int(stats.mask() as i32)?;
    for value in stats.shorts().into_iter().filter(|&value| value != 0) {
        packet.write_short(value)?;
    }
    for value in stats.bytes().into_iter().filter(|&value| value != 0) {
        packet.write_byte(value)?;
    }
    Ok(packet)
}

/// FORCED_STAT_RESET: no body.
pub fn build_reset() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ForcedStatReset as i16)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_burning_forests_pin_the_heros_stats_as_cosmic_writes_them() {
        assert_eq!(on_map(914000210), Some(BURNING_FOREST));
        assert_eq!(on_map(914000300), None);
        let packet = build_set(&BURNING_FOREST).unwrap();
        let body = &packet.bytes[2..];
        assert_eq!(
            body,
            [
                0x1F, 0x0F, 0, 0, 0xE7, 3, 0xE7, 3, 0xE7, 3, 0xE7, 3, 0xFF, 0, 0xE7, 3, 0xE7, 3,
                0x78, 0x8C
            ]
        );
        assert_eq!(build_reset().unwrap().bytes.len(), 2);
    }
}
