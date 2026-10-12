use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// A mob as other players see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobView {
    pub object_id: i32,
    pub mob_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub foothold: i16,
    /// Fade in (a fresh spawn) rather than just appear.
    pub fade_in: bool,
    pub controlled: bool,
}

/// The shared mob body of SPAWN_MONSTER and SPAWN_MONSTER_CONTROL.
fn write_mob(p: &mut Packet, mob: &MobView) -> Result<(), NetworkError> {
    p.write_int(mob.object_id)?;
    p.write_byte(if mob.controlled { 1 } else { 5 })?;
    p.write_int(mob.mob_id)?;
    // Temporary status mask (four ints); no statuses yet.
    p.write_bytes(&[0; 16])?;
    p.write_short(mob.x)?;
    p.write_short(mob.y)?;
    p.write_byte(mob.stance)?;
    p.write_short(0)?; // origin foothold
    p.write_short(mob.foothold)?;
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

/// SPAWN_MONSTER_CONTROL mode 1: this client now moves the mob.
pub fn build_control_mob(mob: &MobView) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnMonsterControl as i16)?;
    p.write_byte(1)?;
    write_mob(
        &mut p,
        &MobView {
            controlled: true,
            ..*mob
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
pub fn build_move_mob(
    object_id: i32,
    start: (i16, i16),
    movement_bytes: &[u8],
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonster as i16)?;
    p.write_int(object_id)?;
    p.write_byte(0)?;
    p.write_byte(0)?; // skill possible
    p.write_byte(0)?; // skill
    p.write_byte(0)?; // skill id
    p.write_byte(0)?; // skill level
    p.write_short(0)?; // option
    p.write_short(start.0)?;
    p.write_short(start.1)?;
    p.write_bytes(movement_bytes)?;
    Ok(p)
}

/// MOVE_MONSTER_RESPONSE: acknowledge the controller's movement.
pub fn build_move_mob_response(
    object_id: i32,
    move_id: i16,
    mp: i16,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::MoveMonsterResponse as i16)?;
    p.write_int(object_id)?;
    p.write_short(move_id)?;
    p.write_byte(0)?; // use skills
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

/// Touch damage, the `TAKE_DAMAGE` source (`TakeDamagePacket::TOUCH`).
pub const FROM_TOUCH: i8 = -1;

/// DAMAGE_PLAYER: show others that a player was hurt by a mob's touch.
pub fn build_damage_player(
    character_id: i32,
    mob_id: i32,
    damage: i32,
    direction: u8,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::DamagePlayer as i16)?;
    p.write_int(character_id)?;
    p.write_byte(FROM_TOUCH as u8)?;
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
            fade_in: true,
            controlled: false,
        }
    }

    #[test]
    fn spawn_matches_the_v83_layout() {
        let packet = build_spawn_mob(&snail()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonster as i16);
        expect.int(2_000_000_001).byte(5).int(100100);
        expect.bytes(&[0; 16]);
        expect.short(147).short(-25).byte(5).short(0).short(45);
        expect.byte(-2i8 as u8).byte(-1i8 as u8).int(0);
        expect.end();
    }

    #[test]
    fn control_has_a_mode_and_marks_the_mob_controlled() {
        let packet = build_control_mob(&snail()).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnMonsterControl as i16).byte(1);
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
        let moved = build_move_mob(7, (10, 20), &[1, 2]).unwrap();
        let mut expect = Expect::new(&moved.bytes);
        expect.short(SendOpcode::MoveMonster as i16).int(7);
        expect.byte(0).byte(0).byte(0).byte(0).byte(0).short(0);
        expect.short(10).short(20).byte(1).byte(2);
        expect.end();
        let response = build_move_mob_response(7, 3, 0).unwrap();
        let mut expect = Expect::new(&response.bytes);
        expect
            .short(SendOpcode::MoveMonsterResponse as i16)
            .int(7)
            .short(3)
            .byte(0)
            .short(0)
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
