use super::buff::write_mask;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::MobStat;
use packet::{io::write::PktWrite, Packet};
use std::time::Duration;

/// A status on a mob, as the client is told of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobStatusView {
    pub stat: MobStat,
    pub value: i16,
    /// The skill that caused it.
    pub skill_id: i32,
    /// How long it has left.
    pub left: Duration,
}

/// A mob as other players see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobView {
    pub object_id: i32,
    pub mob_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    /// The foothold the mob stands on, 0 if airborne. v83 puts the mob on it.
    pub foothold: i16,
    /// The spawn point's foothold.
    pub home_foothold: i16,
    /// Fade in (a fresh spawn) rather than just appear.
    pub fade_in: bool,
    pub controlled: bool,
    pub statuses: Vec<MobStatusView>,
}

/// The statuses as the client reads them (FUN_0078b0b1): their flags, then
/// each one's value, skill, and time left in half seconds, in the client's
/// order.
fn write_statuses(p: &mut Packet, statuses: &[MobStatusView]) -> Result<(), NetworkError> {
    write_mask(p, statuses.iter().map(|status| status.stat.bit()))?;
    for stat in MobStat::DECODE_ORDER {
        let Some(status) = statuses.iter().find(|status| status.stat == stat) else {
            continue;
        };
        p.write_short(status.value)?;
        p.write_int(status.skill_id)?;
        let halves = status.left.as_millis().div_ceil(500);
        p.write_short(halves.min(i16::MAX as u128) as i16)?;
    }
    Ok(())
}

/// The shared mob body of SPAWN_MONSTER and SPAWN_MONSTER_CONTROL.
fn write_mob(p: &mut Packet, mob: &MobView) -> Result<(), NetworkError> {
    p.write_int(mob.object_id)?;
    p.write_byte(if mob.controlled { 1 } else { 5 })?;
    p.write_int(mob.mob_id)?;
    write_statuses(p, &mob.statuses)?;
    p.write_short(mob.x)?;
    p.write_short(mob.y)?;
    p.write_byte(mob.stance)?;
    p.write_short(mob.foothold)?;
    p.write_short(mob.home_foothold)?;
    p.write_byte(if mob.fade_in { -2i8 as u8 } else { -1i8 as u8 })?;
    p.write_byte(-1i8 as u8)?; // team
    p.write_int(0)?; // item effect
    Ok(())
}

/// SPAWN_MONSTER: show a mob.
pub fn build_spawn_mob(mob: &MobView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonster as i16)?;
    write_mob(&mut p, mob)?;
    Ok(p)
}

/// SPAWN_MONSTER_CONTROL mode 1: this client now moves the mob. Mode 2
/// (`aggro`) also has it chase this client's player.
pub fn build_control_mob(mob: &MobView, aggro: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonsterControl as i16)?;
    p.write_byte(if aggro { 2 } else { 1 })?;
    write_mob(
        &mut p,
        &MobView {
            controlled: true,
            ..mob.clone()
        },
    )?;
    Ok(p)
}

/// SPAWN_MONSTER_CONTROL mode 0: stop moving the mob.
pub fn build_stop_controlling_mob(object_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonsterControl as i16)?;
    p.write_byte(0)?;
    p.write_int(object_id)?;
    Ok(p)
}

/// MOVE_MONSTER: relay the controller's movement to the others.
pub fn build_move_mob(moved: &MobMoveView, movement_bytes: &[u8]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonster as i16)?;
    p.write_int(moved.object_id)?;
    p.write_byte(0)?; // not forced to land
    p.write_byte(u8::from(moved.next_attack))?;
    p.write_byte(moved.action)?;
    p.write_int(moved.skill_data)?;
    p.write_short(moved.start.0)?;
    p.write_short(moved.start.1)?;
    p.write_bytes(movement_bytes)?;
    Ok(p)
}

/// A controller's mob move as the others see it (FUN_0066be61).
pub struct MobMoveView {
    pub object_id: i32,
    /// Whether its controller may attack with it next.
    pub next_attack: bool,
    /// What it did: action << 1 | facing left, or 0xFF for nothing.
    pub action: u8,
    /// The attack's target, or the skill's id and level.
    pub skill_data: i32,
    pub start: (i16, i16),
}

/// MOVE_MONSTER_RESPONSE: acknowledge the controller's movement, saying
/// whether it may attack next and the mob's MP (FUN_0066c23b).
pub fn build_move_mob_response(
    object_id: i32,
    move_id: i16,
    next_attack: bool,
    mp: i16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonsterResponse as i16)?;
    p.write_int(object_id)?;
    p.write_short(move_id)?;
    p.write_byte(u8::from(next_attack))?;
    p.write_short(mp)?;
    p.write_byte(0)?; // skill id
    p.write_byte(0)?; // skill level
    Ok(p)
}

/// KILL_MONSTER: remove a mob; `animation` 1 plays its death.
pub fn build_kill_mob(object_id: i32, animation: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::KillMonster as i16)?;
    p.write_int(object_id)?;
    p.write_byte(animation)?;
    p.write_byte(animation)?;
    Ok(p)
}

/// SHOW_MONSTER_HP: the HP bar shown to the player who hit it.
pub fn build_mob_hp(object_id: i32, percent: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowMonsterHp as i16)?;
    p.write_int(object_id)?;
    p.write_byte(percent)?;
    Ok(p)
}

/// SHOW_MAGNET: another player's Monster Magnet caught the mob, or failed
/// to; the client shows the catch over it (0x66d6b9).
pub fn build_show_magnet(object_id: i32, success: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowMagnet as i16)?;
    p.write_int(object_id)?;
    p.write_byte(u8::from(success))?;
    Ok(p)
}

/// MOB_STAT_SET: statuses put on a mob (0x66c301). The client has them
/// take hold at once, and for one that stops the mob reads a byte more.
pub fn build_mob_stat_set(
    object_id: i32,
    statuses: &[MobStatusView],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MobStatSet as i16)?;
    p.write_int(object_id)?;
    write_statuses(&mut p, statuses)?;
    p.write_short(0)?; // delay
    p.write_byte(0)?;
    if statuses.iter().any(|status| status.stat.stops()) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// MOB_STAT_RESET: statuses a mob lost (0x66c424). For one that stopped
/// it, a mob some templates mark reads a byte more; the rest ignore it.
pub fn build_mob_stat_reset(object_id: i32, stats: &[MobStat]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MobStatReset as i16)?;
    p.write_int(object_id)?;
    write_mask(&mut p, stats.iter().map(|stat| stat.bit()))?;
    p.write_byte(0)?;
    if stats.iter().any(|stat| stat.stops()) {
        p.write_byte(0)?;
    }
    Ok(p)
}

/// Touch damage, the `TAKE_DAMAGE` source (`TakeDamagePacket::TOUCH`).
pub const FROM_TOUCH: i8 = -1;

/// DAMAGE_PLAYER: show others that a player was hurt by a mob's touch
/// ([`FROM_TOUCH`]) or attack `from` (FUN_009832e3).
pub fn build_damage_player(
    character_id: i32,
    from: i8,
    mob_id: i32,
    damage: i32,
    direction: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DamagePlayer as i16)?;
    p.write_int(character_id)?;
    p.write_byte(from as u8)?;
    p.write_int(damage)?;
    p.write_int(mob_id)?;
    p.write_byte(direction)?;
    p.write_short(0)?;
    p.write_int(damage)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    fn snail() -> MobView {
        MobView {
            object_id: 2_000_000_001,
            mob_id: 100100,
            x: 147,
            y: -25,
            stance: 5,
            foothold: 45,
            home_foothold: 44,
            fade_in: true,
            controlled: false,
            statuses: Vec::new(),
        }
    }

    #[test]
    fn spawn_carries_the_mob_statuses() {
        let mut mob = snail();
        mob.statuses = vec![MobStatusView {
            stat: MobStat::Speed,
            value: -40,
            skill_id: 2101003,
            left: Duration::from_millis(39_800),
        }];
        let packet = build_spawn_mob(&mob).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.long(0).long(1 << (32 + 6));
        expect.short(-40).int(2101003).short(80);
        expect.short(147);
    }

    #[test]
    fn stat_set_follows_the_client_order_and_stops_with_a_byte() {
        let status = |stat, value, skill_id| MobStatusView {
            stat,
            value,
            skill_id,
            left: Duration::from_secs(10),
        };
        let statuses = [
            status(MobStat::Doom, 1, 2311005),
            status(MobStat::PImmune, 1, 140 | 1 << 16),
            status(MobStat::Pad, -20, 1201006),
        ];
        let packet = build_mob_stat_set(5, &statuses).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatSet as i16).int(5);
        expect
            .long(0)
            .long(1 << 32 | 1 << (32 + 16) | 1 << (32 + 18));
        expect.short(-20).int(1201006).short(20);
        expect.short(1).int(140 | 1 << 16).short(20);
        expect.short(1).int(2311005).short(20);
        expect.short(0).byte(0).byte(0);
        expect.end();
    }

    #[test]
    fn stat_reset_names_the_lost_statuses() {
        let packet = build_mob_stat_reset(5, &[MobStat::Seal]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatReset as i16).int(5);
        expect.long(0).long(1 << (32 + 10));
        expect.byte(0);
        expect.end();
        let packet = build_mob_stat_reset(5, &[MobStat::Doom]).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MobStatReset as i16).int(5);
        expect.long(0).long(1 << (32 + 16));
        expect.byte(0).byte(0);
        expect.end();
    }

    #[test]
    fn spawn_matches_the_v83_layout() {
        let packet = build_spawn_mob(&snail()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.bytes(&[0; 16]);
        expect.short(147).short(-25).byte(5).short(45).short(44);
        expect.byte(-2i8 as u8).byte(-1i8 as u8).int(0);
        expect.end();
    }

    #[test]
    fn control_has_a_mode_and_marks_the_mob_controlled() {
        let packet = build_control_mob(&snail(), false).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonsterControl as i16).byte(1);
        expect.int(2_000_000_001).byte(1).int(100100);
        let aggro = build_control_mob(&snail(), true).unwrap();
        let mut expect = Expect::new(&aggro.bytes);
        expect.short(SendOpcode::SpawnMonsterControl as i16).byte(2);
        expect.int(2_000_000_001).byte(1).int(100100);
        let stop = build_stop_controlling_mob(2_000_000_001).unwrap();
        let mut expect = Expect::new(&stop.bytes);
        expect
            .short(SendOpcode::SpawnMonsterControl as i16)
            .byte(0)
            .int(2_000_000_001);
        expect.end();
    }

    #[test]
    fn movement_kill_and_hp() {
        let view = MobMoveView {
            object_id: 7,
            next_attack: true,
            action: 12 << 1 | 1,
            skill_data: 0x0050_0040,
            start: (10, 20),
        };
        let moved = build_move_mob(&view, &[1, 2]).unwrap();
        let mut expect = Expect::new(&moved.bytes);
        expect.short(SendOpcode::MoveMonster as i16).int(7);
        expect.byte(0).byte(1).byte(25).int(0x0050_0040);
        expect.short(10).short(20).byte(1).byte(2);
        expect.end();
        let response = build_move_mob_response(7, 3, true, 40).unwrap();
        let mut expect = Expect::new(&response.bytes);
        expect
            .short(SendOpcode::MoveMonsterResponse as i16)
            .int(7)
            .short(3)
            .byte(1)
            .short(40)
            .byte(0)
            .byte(0);
        expect.end();
        let kill = build_kill_mob(7, 1).unwrap();
        let mut expect = Expect::new(&kill.bytes);
        expect
            .short(SendOpcode::KillMonster as i16)
            .int(7)
            .byte(1)
            .byte(1);
        expect.end();
        let hp = build_mob_hp(7, 40).unwrap();
        let mut expect = Expect::new(&hp.bytes);
        expect
            .short(SendOpcode::ShowMonsterHp as i16)
            .int(7)
            .byte(40);
        expect.end();
    }
}
