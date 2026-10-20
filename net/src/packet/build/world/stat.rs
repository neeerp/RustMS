use crate::{error::NetworkError, packet::op::SendOpcode};
use game::CharacterStats;
use packet::{io::write::PktWrite, Packet};

/// One changed stat in a STAT_CHANGED packet. Each variant knows its mask bit
/// and wire width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatUpdate {
    Skin(u8),
    Face(i32),
    Hair(i32),
    /// The cash serials of the summoned pets, 0 for an empty place.
    Pets([i64; 3]),
    Level(u8),
    Job(i16),
    Str(i16),
    Dex(i16),
    Int(i16),
    Luk(i16),
    Hp(i16),
    MaxHp(i16),
    Mp(i16),
    MaxMp(i16),
    Ap(i16),
    Sp(i16),
    Exp(i32),
    Fame(i16),
    Meso(i32),
    /// EXP from Writs of Solomon not yet taken.
    GachaExp(i32),
}

impl StatUpdate {
    pub fn mask(self) -> i32 {
        match self {
            StatUpdate::Skin(_) => 0x1,
            StatUpdate::Face(_) => 0x2,
            StatUpdate::Hair(_) => 0x4,
            StatUpdate::Pets(_) => 0x8 | 0x80000 | 0x100000,
            StatUpdate::Level(_) => 0x10,
            StatUpdate::Job(_) => 0x20,
            StatUpdate::Str(_) => 0x40,
            StatUpdate::Dex(_) => 0x80,
            StatUpdate::Int(_) => 0x100,
            StatUpdate::Luk(_) => 0x200,
            StatUpdate::Hp(_) => 0x400,
            StatUpdate::MaxHp(_) => 0x800,
            StatUpdate::Mp(_) => 0x1000,
            StatUpdate::MaxMp(_) => 0x2000,
            StatUpdate::Ap(_) => 0x4000,
            StatUpdate::Sp(_) => 0x8000,
            StatUpdate::Exp(_) => 0x10000,
            StatUpdate::Fame(_) => 0x20000,
            StatUpdate::Meso(_) => 0x40000,
            StatUpdate::GachaExp(_) => 0x200000,
        }
    }

    fn write(self, packet: &mut Packet) -> Result<(), NetworkError> {
        match self {
            StatUpdate::Skin(v) | StatUpdate::Level(v) => {
                packet.write_byte(v)?;
            }
            StatUpdate::Face(v)
            | StatUpdate::Hair(v)
            | StatUpdate::Exp(v)
            | StatUpdate::Meso(v)
            | StatUpdate::GachaExp(v) => packet.write_int(v)?,
            StatUpdate::Job(v)
            | StatUpdate::Str(v)
            | StatUpdate::Dex(v)
            | StatUpdate::Int(v)
            | StatUpdate::Luk(v)
            | StatUpdate::Hp(v)
            | StatUpdate::MaxHp(v)
            | StatUpdate::Mp(v)
            | StatUpdate::MaxMp(v)
            | StatUpdate::Ap(v)
            | StatUpdate::Sp(v)
            | StatUpdate::Fame(v) => packet.write_short(v)?,
            StatUpdate::Pets(serials) => {
                for serial in serials {
                    packet.write_long(serial)?;
                }
            }
        }
        Ok(())
    }
}

/// STAT_CHANGED. Values are written in mask order regardless of input order;
/// the pets go by their lowest bit, right after the hair, as the client reads
/// them. `enable_actions` unlocks the client UI (an empty update does only
/// that).
pub fn build_stat_update(
    enable_actions: bool,
    updates: &[StatUpdate],
) -> Result<Packet, NetworkError> {
    let mut sorted = updates.to_vec();
    sorted.sort_by_key(|update| update.mask().trailing_zeros());

    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::StatChange as i16)?;
    packet.write_byte(enable_actions as u8)?;
    packet.write_int(sorted.iter().fold(0, |mask, update| mask | update.mask()))?;
    for update in sorted {
        update.write(&mut packet)?;
    }
    Ok(packet)
}

/// HP and MP, e.g. after `Player::fit_hp_mp` brought them down.
pub fn build_hp_mp(stats: &CharacterStats) -> Result<Packet, NetworkError> {
    build_stat_update(false, &[StatUpdate::Hp(stats.hp), StatUpdate::Mp(stats.mp)])
}

/// The stats an EXP gain changes: EXP, and after a level-up the level, AP,
/// SP, and the (restored) HP and MP.
pub fn build_exp_stats(stats: &CharacterStats, levelled: bool) -> Result<Packet, NetworkError> {
    let mut updates = vec![StatUpdate::Exp(stats.exp)];
    if levelled {
        updates.extend([
            StatUpdate::Level(stats.level),
            StatUpdate::Ap(stats.ap),
            StatUpdate::Sp(stats.sp),
            StatUpdate::Hp(stats.hp),
            StatUpdate::MaxHp(stats.max_hp),
            StatUpdate::Mp(stats.mp),
            StatUpdate::MaxMp(stats.max_mp),
        ]);
    }
    build_stat_update(false, &updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::stats::sample_stats;
    use crate::codec::testing::Expect;

    #[test]
    fn values_follow_mask_order_with_their_wire_widths() {
        let packet = build_stat_update(
            true,
            &[StatUpdate::Exp(15), StatUpdate::Ap(5), StatUpdate::Level(2)],
        )
        .unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(1);
        expect.int(0x10 | 0x4000 | 0x10000);
        expect.byte(2).short(5).int(15);
        expect.end();
    }

    #[test]
    fn pet_serials_follow_the_hair() {
        let packet = build_stat_update(
            true,
            &[
                StatUpdate::Level(9),
                StatUpdate::Pets([7, 0, 0]),
                StatUpdate::Hair(30000),
            ],
        )
        .unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(1);
        expect.int(0x4 | 0x8 | 0x10 | 0x80000 | 0x100000);
        expect.int(30000).long(7).long(0).long(0).byte(9);
        expect.end();
    }

    #[test]
    fn gacha_exp_is_an_int_after_the_meso() {
        let updates = [StatUpdate::GachaExp(5000), StatUpdate::Meso(10)];
        let packet = build_stat_update(true, &updates).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(1);
        expect.int(0x40000 | 0x200000).int(10).int(5000);
        expect.end();
    }

    #[test]
    fn empty_update_only_enables_actions() {
        let packet = build_stat_update(true, &[]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::StatChange as i16)
            .byte(1)
            .int(0)
            .end();
    }

    #[test]
    fn exp_stats_add_level_ap_sp_hp_and_mp_after_a_level_up() {
        let stats = sample_stats();
        let packet = build_exp_stats(&stats, false).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(0);
        expect.int(0x10000).int(1234);
        expect.end();

        let packet = build_exp_stats(&stats, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::StatChange as i16).byte(0);
        expect.int(0x10 | 0x400 | 0x800 | 0x1000 | 0x2000 | 0x4000 | 0x8000 | 0x10000);
        expect.byte(10).short(300).short(310).short(50).short(55);
        expect.short(3).short(1).int(1234);
        expect.end();
    }
}
