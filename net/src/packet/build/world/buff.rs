//! Buffs, skill cooldowns, and skill effects.

use crate::{error::NetworkError, packet::op::SendOpcode};
use game::{Beacon, BuffStat, Mount, Ride, ShownBuff};
use packet::{io::write::PktWrite, Packet};

/// A temporary stat's bit in the client's 128-bit flags, as the two longs
/// the packet carries (first, second). The flags go out as they lie in
/// memory, with bits 0..31 in the last dword and 96..127 in the first.
fn mask(bit: u8) -> (u64, u64) {
    match bit {
        0..=31 => (0, 1 << (32 + bit)),
        32..=63 => (0, 1 << (bit - 32)),
        64..=95 => (1 << (bit - 32), 0),
        _ => (1 << (bit - 96), 0),
    }
}

/// The two-state stats (energy charge, dash, riding, ...), bits 82..88.
/// A player always carries them; their data has a fixed size.
pub(crate) const TWO_STATE: [u8; 7] = [82, 83, 84, 85, 86, 87, 88];

pub(crate) fn write_mask(
    p: &mut Packet,
    bits: impl IntoIterator<Item = u8>,
) -> Result<(), NetworkError> {
    let (first, second) = bits.into_iter().fold((0, 0), |(first, second), bit| {
        let (f, s) = mask(bit);
        (first | f, second | s)
    });
    p.write_long(first as i64)?;
    p.write_long(second as i64)?;
    Ok(())
}

/// Stats the client reads one more byte for at the end of GIVE_BUFF
/// (FUN_0077dc78): the ones that change how the player moves. Morph (33),
/// 49, and the two-state dash and riding stats (83..85) are among them
/// too.
fn moves(stat: BuffStat) -> bool {
    matches!(
        stat,
        BuffStat::Speed
            | BuffStat::Jump
            | BuffStat::Stun
            | BuffStat::Weakness
            | BuffStat::Slow
            | BuffStat::MapleWarrior
            | BuffStat::Seduce
    )
}

/// GIVE_BUFF: the stats a skill gives, lasting `duration_ms`
/// (FUN_00a202be). Each stat's value, skill, and duration come in the
/// order of their bits. A disease's skill is its mob skill and level,
/// `skill | level << 16`. Speed Infusion's two-state data follows the
/// defense attack and state.
pub fn build_give_buff(
    skill_id: i32,
    duration_ms: i32,
    stats: &[(BuffStat, i16)],
) -> Result<Packet, NetworkError> {
    let mut stats = stats.to_vec();
    stats.sort_by_key(|&(stat, _)| stat.bit());
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, stats.iter().map(|&(stat, _)| stat.bit()))?;
    let infusion = stats
        .iter()
        .find(|&&(stat, _)| stat == BuffStat::SpeedInfusion);
    for &(stat, value) in &stats {
        if stat == BuffStat::SpeedInfusion {
            continue;
        }
        p.write_short(value)?;
        p.write_int(skill_id)?;
        p.write_int(duration_ms)?;
    }
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    if let Some(&(_, value)) = infusion {
        let seconds = (duration_ms / 1000).clamp(0, i32::from(u16::MAX)) as u16;
        write_infusion(&mut p, value, skill_id, seconds)?;
    }
    p.write_short(0)?; // delay
    if stats.iter().any(|&(stat, _)| moves(stat)) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// CANCEL_BUFF: these buffs ended.
pub fn build_cancel_buff(stats: &[BuffStat]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CancelBuff as i16)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    p.write_byte(1)?;
    Ok(p)
}

/// The buffs another player shows (FUN_00788156), as SPAWN_PLAYER and
/// GIVE_FOREIGN_BUFF carry them: the flags, then each one's data in the
/// client's fixed order, then the defense attack and state. SPAWN_PLAYER
/// also sets the two-state stats, whose data the caller writes next.
pub(crate) fn write_shown(
    p: &mut Packet,
    shown: &[ShownBuff],
    two_state: bool,
) -> Result<(), NetworkError> {
    let two_state = two_state.then_some(TWO_STATE).into_iter().flatten();
    write_mask(p, shown.iter().map(|buff| buff.stat.bit()).chain(two_state))?;
    for stat in BuffStat::SHOWN {
        let Some(buff) = shown.iter().find(|buff| buff.stat == stat) else {
            continue;
        };
        match stat {
            BuffStat::Speed | BuffStat::Combo => {
                p.write_byte(buff.value as u8)?;
            }
            BuffStat::Poison => {
                p.write_short(buff.value)?;
                p.write_int(buff.skill_id)?;
            }
            BuffStat::WeaponCharge
            | BuffStat::Stun
            | BuffStat::Darkness
            | BuffStat::Seal
            | BuffStat::Weakness
            | BuffStat::Curse
            | BuffStat::Seduce
            | BuffStat::Undead
            | BuffStat::Confuse
            | BuffStat::StopPotion
            | BuffStat::StopMotion
            | BuffStat::Fear => {
                p.write_int(buff.skill_id)?;
            }
            BuffStat::ShadowStars => {
                p.write_int(i32::from(buff.value))?;
            }
            _ => {}
        }
    }
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    Ok(())
}

/// GIVE_FOREIGN_BUFF: another player started showing these buffs
/// (0x98385d); the client leaves the others as they were.
pub fn build_give_foreign_buff(
    character_id: i32,
    shown: &[ShownBuff],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_shown(&mut p, shown, false)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// CANCEL_FOREIGN_BUFF: another player's shown buffs ended (0x983921).
pub fn build_cancel_foreign_buff(
    character_id: i32,
    stats: &[BuffStat],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CancelForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    Ok(p)
}

/// A two-state stat's data, as Energy Charge (bit 82) and Dash (83, 84)
/// carry it (v83 0x793ef2, 0x794332): the value and the skill, then when
/// it started and how many seconds it lasts. The client never times it out
/// itself; the seconds only time its buff icon.
pub(crate) fn write_two_state(
    p: &mut Packet,
    buff: Option<ShownBuff>,
    seconds: u16,
) -> Result<(), NetworkError> {
    let (value, skill_id) = buff.map_or((0, 0), |buff| (buff.value, buff.skill_id));
    p.write_int(i32::from(value))?;
    p.write_int(skill_id)?;
    p.write_byte(0)?; // start before (1) or after now
    p.write_int(0)?; // how long
    p.write_short(seconds as i16)?;
    Ok(())
}

/// Speed Infusion's two-state data (PartyBooster, bit 86; v83 0x793e28):
/// as the others', with a second sign and delta for when it was given.
fn write_infusion(
    p: &mut Packet,
    value: i16,
    skill_id: i32,
    seconds: u16,
) -> Result<(), NetworkError> {
    p.write_int(i32::from(value))?;
    p.write_int(skill_id)?;
    p.write_byte(0)?; // start before (1) or after now
    p.write_int(0)?; // how long
    p.write_byte(0)?; // given before (1) or after now
    p.write_int(0)?; // how long
    p.write_short(seconds as i16)?;
    Ok(())
}

/// GIVE_BUFF for Energy Charge: the player's gauge holds `energy` now
/// (0x95f065 opens or closes it). Unlike riding, energy doesn't move the
/// player, so nothing follows the delay.
pub fn build_give_energy(energy: ShownBuff) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, [BuffStat::EnergyCharge.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_two_state(&mut p, Some(energy), 0)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// GIVE_FOREIGN_BUFF for Energy Charge: another player's energy, which
/// glows about them once full (0x9838b7).
pub fn build_give_foreign_energy(
    character_id: i32,
    energy: ShownBuff,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_mask(&mut p, [BuffStat::EnergyCharge.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_two_state(&mut p, Some(energy), 0)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// GIVE_BUFF for Dash: its speed and jump (bits 83 and 84) for `seconds`.
/// Like riding, it moves the player, so a byte follows the delay
/// (FUN_0077dc78).
pub fn build_give_dash(
    speed: ShownBuff,
    jump: ShownBuff,
    seconds: u16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, [speed.stat.bit(), jump.stat.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_two_state(&mut p, Some(speed), seconds)?;
    write_two_state(&mut p, Some(jump), seconds)?;
    p.write_short(0)?; // delay
    p.write_byte(0)?;
    Ok(p)
}

/// GIVE_FOREIGN_BUFF for Dash: another player dashes, raising dust while
/// they're on the ground (0x9310a2).
pub fn build_give_foreign_dash(
    character_id: i32,
    speed: ShownBuff,
    jump: ShownBuff,
    seconds: u16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_mask(&mut p, [speed.stat.bit(), jump.stat.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_two_state(&mut p, Some(speed), seconds)?;
    write_two_state(&mut p, Some(jump), seconds)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// Riding's two-state data (RideVehicle, bit 85; v83 0x793ef2): the mount
/// and the skill riding it, then when it started, which the client never
/// times out.
pub(crate) fn write_ride(p: &mut Packet, ride: Option<Ride>) -> Result<(), NetworkError> {
    let (item_id, skill_id) = ride.map_or((0, 0), |ride| (ride.item_id, ride.skill_id));
    p.write_int(item_id)?;
    p.write_int(skill_id)?;
    p.write_byte(0)?; // start before (1) or after now
    p.write_int(0)?; // how long
    Ok(())
}

/// GIVE_BUFF for riding: the player gets on their mount. A two-state stat
/// has no value, skill and duration of its own: its data follows the
/// defense bytes, and riding moves the player, so one more byte ends it.
pub fn build_give_ride(ride: Ride) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, [BuffStat::Riding.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_ride(&mut p, Some(ride))?;
    p.write_short(0)?; // delay
    p.write_byte(0)?;
    Ok(p)
}

/// GIVE_FOREIGN_BUFF for riding: another player got on their mount.
pub fn build_give_foreign_ride(character_id: i32, ride: Ride) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveForeignBuff as i16)?;
    p.write_int(character_id)?;
    write_mask(&mut p, [BuffStat::Riding.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    write_ride(&mut p, Some(ride))?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// GIVE_BUFF for Homing Beacon: the player's shots seek `beacon.mob`, which
/// the client marks with the skill's `mob` animation. GuidedBullet's
/// two-state data (bit 87; v83 0x77c442) is the others' with no seconds and
/// the mob after them; the client never times it out.
pub fn build_give_beacon(beacon: Beacon) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::GiveBuff as i16)?;
    write_mask(&mut p, [BuffStat::GuidedBullet.bit()])?;
    p.write_byte(0)?; // defense attack
    p.write_byte(0)?; // defense state
    p.write_int(1)?;
    p.write_int(beacon.skill_id)?;
    p.write_byte(0)?; // start before (1) or after now
    p.write_int(0)?; // how long
    p.write_int(beacon.mob)?;
    p.write_short(0)?; // delay
    Ok(p)
}

/// SET_TAMING_MOB_INFO: a player's mount's level, EXP and tiredness;
/// `level_up` plays its level-up effect.
pub fn build_taming_mob_info(
    character_id: i32,
    mount: &Mount,
    level_up: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SetTamingMobInfo as i16)?;
    p.write_int(character_id)?;
    p.write_int(mount.level)?;
    p.write_int(mount.exp)?;
    p.write_int(mount.tiredness)?;
    p.write_byte(u8::from(level_up))?;
    Ok(p)
}

/// COOLDOWN: the skill can't be used for `seconds`.
pub fn build_cooldown(skill_id: i32, seconds: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::Cooldown as i16)?;
    p.write_int(skill_id)?;
    p.write_short(seconds)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT 1: another player used a skill.
pub fn build_skill_effect(
    character_id: i32,
    skill_id: i32,
    character_level: u8,
    skill_level: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(1)?;
    p.write_int(skill_id)?;
    p.write_byte(character_level)?;
    p.write_byte(skill_level)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT 1: the same on the player, as MP Eater shows
/// when it eats (v83 FUN_009377d9 reads the same three fields).
pub fn build_own_skill_effect(
    skill_id: i32,
    character_level: u8,
    skill_level: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(1)?;
    p.write_int(skill_id)?;
    p.write_byte(character_level)?;
    p.write_byte(skill_level)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT 2: a party member's skill reached the player,
/// who plays its `affected` animation (0x93632a).
pub fn build_own_affected(skill_id: i32, skill_level: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(2)?;
    p.write_int(skill_id)?;
    p.write_byte(skill_level)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT 2: the same, on another player.
pub fn build_foreign_affected(
    character_id: i32,
    skill_id: i32,
    skill_level: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(2)?;
    p.write_int(skill_id)?;
    p.write_byte(skill_level)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT 5: the skill's `special` animation on the player,
/// as each drink of Dragon Blood plays (0x937bb8).
pub fn build_own_special(skill_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(5)?;
    p.write_int(skill_id)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT 5: the same, on another player.
pub fn build_foreign_special(character_id: i32, skill_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(5)?;
    p.write_int(skill_id)?;
    Ok(p)
}

/// The skill effect of a Monster Magnet, which also says which way the
/// player faced (0x937b42).
pub fn build_magnet_effect(
    character_id: i32,
    skill_id: i32,
    character_level: u8,
    skill_level: u8,
    toleft: bool,
) -> Result<Packet, NetworkError> {
    let mut p = build_skill_effect(character_id, skill_id, character_level, skill_level)?;
    p.write_byte(u8::from(toleft))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn a_beacon_marks_its_mob_in_seventeen_bytes() {
        let beacon = Beacon {
            skill_id: 5211006,
            mob: 1234,
        };
        let packet = build_give_beacon(beacon).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        // Bit 87, as Cosmic's 0x80000000000000 in the first long.
        expect.long(0x80_0000_0000_0000).long(0);
        expect.byte(0).byte(0);
        expect.int(1).int(5211006).byte(0).int(0).int(1234);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn give_buff_matches_the_client_masks() {
        let packet = build_give_buff(1002, 4000, &[(BuffStat::Speed, 10)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(0x80_0000_0000);
        expect.short(10).int(1002).int(4000);
        // Defense, delay, and the byte for a stat that changes movement.
        expect.byte(0).byte(0).short(0).byte(0);
        expect.end();
    }

    #[test]
    fn give_buff_writes_stats_in_bit_order() {
        // Hyper Body, given out of order; neither changes movement.
        let stats = [(BuffStat::MaxMp, 60), (BuffStat::MaxHp, 50)];
        let packet = build_give_buff(1301007, 300_000, &stats).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(0x6000_0000_0000);
        expect.short(50).int(1301007).int(300_000);
        expect.short(60).int(1301007).int(300_000);
        expect.byte(0).byte(0).short(0);
        expect.end();
    }

    #[test]
    fn speed_infusion_gives_its_two_state_block_after_the_defense() {
        let packet = build_give_buff(5121009, 300_000, &[(BuffStat::SpeedInfusion, -2)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(1 << 54).long(0);
        expect.byte(0).byte(0);
        // Its value and skill, two signs and deltas, then its seconds.
        expect
            .int(-2)
            .int(5121009)
            .byte(0)
            .int(0)
            .byte(0)
            .int(0)
            .short(300);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn hunters_luck_sets_bit_74() {
        let packet = build_give_buff(-2450000, 1_800_000, &[(BuffStat::ExpBuff, 200)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(1 << 42).long(0);
        expect.short(200).int(-2450000).int(1_800_000);
        expect.byte(0).byte(0).short(0);
        expect.end();
    }

    #[test]
    fn diseases_that_stop_movement_carry_the_movement_byte() {
        let seduce = 128 | 3 << 16;
        let packet = build_give_buff(seduce, 5_000, &[(BuffStat::Seduce, 3)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(1 << 7);
        expect.short(3).int(seduce).int(5_000);
        expect.byte(0).byte(0).short(0).byte(0);
        expect.end();

        let seal = 120 | 1 << 16;
        let packet = build_give_buff(seal, 5_000, &[(BuffStat::Seal, 1)]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(0).long(1 << 51);
        expect.short(1).int(seal).int(5_000);
        expect.byte(0).byte(0).short(0);
        expect.end();
    }

    #[test]
    fn masks_cover_all_four_dwords() {
        assert_eq!(mask(0), (0, 1 << 32));
        assert_eq!(mask(34), (0, 4));
        assert_eq!(mask(82), (1 << 50, 0));
        assert_eq!(mask(100), (1 << 4, 0));
    }

    #[test]
    fn foreign_buffs_carry_shown_data_in_the_client_order() {
        let shown = |stat, value, skill_id| ShownBuff {
            stat,
            value,
            skill_id,
        };
        let shown = [
            shown(BuffStat::SoulArrow, 1, 4101000),
            shown(BuffStat::WeaponCharge, 1, 1211004),
            shown(BuffStat::Speed, 40, 1002),
        ];
        let packet = build_give_foreign_buff(7, &shown).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(0).long((1 << 39) | (1 << 54) | (1 << 48));
        expect.byte(40).int(1211004);
        expect.byte(0).byte(0).short(0);
        expect.end();

        // Diseases carry their mob skill; Poison its value first.
        let (stun, poison) = (123 | 1 << 16, 125 | 2 << 16);
        let diseases = [
            ShownBuff {
                stat: BuffStat::Poison,
                value: 15,
                skill_id: poison,
            },
            ShownBuff {
                stat: BuffStat::Stun,
                value: 1,
                skill_id: stun,
            },
        ];
        let packet = build_give_foreign_buff(7, &diseases).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(0).long((1 << 49) | (1 << 50));
        expect.int(stun).short(15).int(poison);
        expect.byte(0).byte(0).short(0);
        expect.end();

        let packet = build_cancel_foreign_buff(7, &[BuffStat::DarkSight]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelForeignBuff as i16).int(7);
        expect.long(0).long(1 << 42);
        expect.end();
    }

    #[test]
    fn cancel_buff_combines_masks() {
        let packet = build_cancel_buff(&[BuffStat::Recovery, BuffStat::Speed]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelBuff as i16);
        expect.long(0).long(0x80_0000_0004).byte(1);
        expect.end();
    }

    #[test]
    fn riding_carries_the_mount_and_skill_in_its_two_state_block() {
        let ride = Ride {
            item_id: 1902000,
            skill_id: 1004,
        };
        let packet = build_give_ride(ride).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(1 << 53).long(0);
        expect.byte(0).byte(0);
        expect.int(1902000).int(1004).byte(0).int(0);
        expect.short(0).byte(0);
        expect.end();

        let packet = build_give_foreign_ride(7, ride).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(1 << 53).long(0);
        expect.byte(0).byte(0);
        expect.int(1902000).int(1004).byte(0).int(0);
        expect.short(0);
        expect.end();

        let packet = build_cancel_buff(&[BuffStat::Riding]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::CancelBuff as i16);
        expect.long(1 << 53).long(0).byte(1);
        expect.end();
    }

    #[test]
    fn energy_carries_its_value_and_skill_in_a_15_byte_block() {
        let energy = ShownBuff {
            stat: BuffStat::EnergyCharge,
            value: 10000,
            skill_id: 5110001,
        };
        let packet = build_give_energy(energy).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(1 << 50).long(0);
        expect.byte(0).byte(0);
        expect.int(10000).int(5110001).byte(0).int(0).short(0);
        expect.short(0);
        expect.end();

        let packet = build_give_foreign_energy(7, energy).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(1 << 50).long(0);
        expect.byte(0).byte(0);
        expect.int(10000).int(5110001).byte(0).int(0).short(0);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn dash_carries_its_speed_and_jump_in_two_15_byte_blocks() {
        let speed = ShownBuff {
            stat: BuffStat::DashSpeed,
            value: 12,
            skill_id: 5001005,
        };
        let jump = ShownBuff {
            stat: BuffStat::DashJump,
            value: 1,
            skill_id: 5001005,
        };
        let packet = build_give_dash(speed, jump, 4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveBuff as i16);
        expect.long(1 << 51 | 1 << 52).long(0);
        expect.byte(0).byte(0);
        expect.int(12).int(5001005).byte(0).int(0).short(4);
        expect.int(1).int(5001005).byte(0).int(0).short(4);
        expect.short(0).byte(0);
        expect.end();

        let packet = build_give_foreign_dash(7, speed, jump, 4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::GiveForeignBuff as i16).int(7);
        expect.long(1 << 51 | 1 << 52).long(0);
        expect.byte(0).byte(0);
        expect.int(12).int(5001005).byte(0).int(0).short(4);
        expect.int(1).int(5001005).byte(0).int(0).short(4);
        expect.short(0);
        expect.end();
    }

    #[test]
    fn taming_mob_info_carries_the_mount() {
        let mount = Mount {
            level: 3,
            exp: 60,
            tiredness: 12,
        };
        let packet = build_taming_mob_info(7, &mount, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SetTamingMobInfo as i16).int(7);
        expect.int(3).int(60).int(12).byte(1);
        expect.end();
    }

    #[test]
    fn skill_effect_carries_both_levels() {
        let packet = build_skill_effect(7, 1001, 12, 3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowForeignEffect as i16).int(7);
        expect.byte(1).int(1001).byte(12).byte(3);
        expect.end();
    }

    #[test]
    fn affected_effects_carry_the_skill_and_its_level() {
        let packet = build_own_affected(2301004, 20).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16);
        expect.byte(2).int(2301004).byte(20);
        expect.end();

        let packet = build_foreign_affected(7, 2301004, 20).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowForeignEffect as i16).int(7);
        expect.byte(2).int(2301004).byte(20);
        expect.end();
    }

    #[test]
    fn special_effects_carry_just_the_skill() {
        let packet = build_own_special(1311008).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16);
        expect.byte(5).int(1311008);
        expect.end();

        let packet = build_foreign_special(7, 1311008).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowForeignEffect as i16).int(7);
        expect.byte(5).int(1311008);
        expect.end();
    }
}
