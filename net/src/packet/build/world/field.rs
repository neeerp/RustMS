use crate::codec::encode_avatar_look;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::FieldAvatar;
use packet::io::read::PktRead;
use packet::{io::write::PktWrite, Packet};
use std::io::Cursor;

/// SPAWN_PLAYER: show `avatar` to the other players on the field
/// (`spawnPlayerMapObject` in HeavenMS, with no guild, buffs, pets, or shops).
pub fn build_player_enter_field(avatar: &FieldAvatar) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnPlayer as i16)?;
    packet.write_int(avatar.id)?;
    packet.write_byte(avatar.level)?;
    packet.write_str_with_length(&avatar.name)?;

    // Guild name and emblem.
    packet.write_str_with_length("")?;
    packet.write_bytes(&[0; 6])?;

    write_foreign_buffs(&mut packet)?;

    packet.write_short(avatar.job)?;
    encode_avatar_look(&mut packet, &avatar.look, false)?;
    packet.write_int(0)?; // count of item 5110000
    packet.write_int(0)?; // item effect
    packet.write_int(0)?; // chair

    packet.write_short(avatar.x)?;
    packet.write_short(avatar.y)?;
    packet.write_byte(avatar.stance)?;
    packet.write_short(0)?; // foothold
    packet.write_byte(0)?;
    packet.write_byte(0)?; // end of pets
    packet.write_int(1)?; // mount level
    packet.write_long(0)?; // mount exp and tiredness
    packet.write_byte(0)?; // no player shop or minigame box
    packet.write_byte(0)?; // no chalkboard
    packet.write_byte(0)?; // crush ring
    packet.write_byte(0)?; // friendship ring
    packet.write_byte(0)?; // marriage ring
    packet.write_byte(0)?; // new year cards
    packet.write_byte(0)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?; // team (only used in event fields)
    Ok(packet)
}

/// The foreign buff block with no active buffs. Its size is fixed by the
/// client (8 + 4 + 4 + 4 + 43 + 65 bytes); the 0xFC/0x01 and the repeated
/// spawn constant mirror what the reference server sends.
fn write_foreign_buffs(packet: &mut Packet) -> Result<(), NetworkError> {
    const CHAR_MAGIC_SPAWN: i32 = 0x2D0F_5E1A;
    packet.write_int(0)?;
    packet.write_short(0)?;
    packet.write_byte(0xFC)?;
    packet.write_byte(1)?;
    packet.write_int(0)?; // morph
    packet.write_int(0)?; // buff mask (high)
    packet.write_int(0)?; // buff mask (low)

    packet.write_int(0)?; // energy charge
    packet.write_short(0)?;
    packet.write_bytes(&[0; 4])?;
    packet.write_int(0)?; // dash speed
    packet.write_bytes(&[0; 11])?;
    packet.write_short(0)?;
    packet.write_bytes(&[0; 9])?; // dash jump
    packet.write_int(0)?;
    packet.write_short(0)?;
    packet.write_byte(0)?;

    packet.write_long(0)?; // monster riding
    packet.write_int(CHAR_MAGIC_SPAWN)?;
    packet.write_bytes(&[0; 8])?; // speed infusion
    packet.write_int(CHAR_MAGIC_SPAWN)?;
    packet.write_byte(0)?;
    packet.write_int(CHAR_MAGIC_SPAWN)?;
    packet.write_short(0)?;
    packet.write_bytes(&[0; 9])?; // homing beacon
    packet.write_int(CHAR_MAGIC_SPAWN)?;
    packet.write_int(0)?;
    packet.write_bytes(&[0; 9])?; // zombify
    packet.write_int(CHAR_MAGIC_SPAWN)?;
    packet.write_short(0)?;
    packet.write_short(0)?;
    Ok(())
}

pub fn build_player_leave_field(character_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::RemovePlayerFromMap as i16)?;
    packet.write_int(character_id)?;
    Ok(packet)
}

pub fn build_player_move(character_id: i32, movement_bytes: &[u8]) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MovePlayer as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(0)?;
    packet.write_bytes(movement_bytes)?;
    Ok(packet)
}

pub fn build_local_chat(
    character_id: i32,
    message: &str,
    from_admin: bool,
    show: u8,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ChatText as i16)?;
    packet.write_int(character_id)?;
    packet.write_byte(if from_admin { 1 } else { 0 })?;
    packet.write_str_with_length(message)?;
    packet.write_byte(show)?;
    Ok(packet)
}

pub fn parse_movement_state(
    movement_bytes: &[u8],
    current: (i16, i16, u8),
) -> Option<(i16, i16, u8)> {
    let mut cursor = Cursor::new(movement_bytes);
    let count = cursor.read_byte().ok()?;
    if count == 0 {
        return None;
    }

    let (mut x, mut y, mut stance) = current;

    for _ in 0..count {
        let command = cursor.read_byte().ok()?;
        match command {
            0 | 5 | 17 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                stance = cursor.read_byte().ok()?;
                let _ = cursor.read_short().ok()?;
            }
            1 | 2 | 6 | 12 | 13 | 16 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                stance = cursor.read_byte().ok()?;
                let _ = cursor.read_short().ok()?;
            }
            11 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                stance = cursor.read_byte().ok()?;
                let _ = cursor.read_short().ok()?;
            }
            15 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                stance = cursor.read_byte().ok()?;
                let _ = cursor.read_short().ok()?;
            }
            3 | 4 | 7 | 8 | 9 | 10 | 14 => {}
            _ => return None,
        }
    }

    Some((x, y, stance))
}
