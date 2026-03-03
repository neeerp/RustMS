use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::io::read::PktRead;
use packet::{io::write::PktWrite, Packet};
use std::io::Cursor;

const DEFAULT_STANCE: u8 = 2;
const STARTING_MAP_ID: i32 = 1_000_000;
const STARTING_MAP_SPAWN_X: i16 = 240;
const STARTING_MAP_SPAWN_Y: i16 = 190;
const DEFAULT_OVERALL: i32 = 1052122;
const DEFAULT_SHOES: i32 = 1072318;
const DEFAULT_WEAPON: i32 = 1302000;

#[derive(Clone, Debug)]
pub struct ForeignCharacter {
    pub id: i32,
    pub name: String,
    pub level: i16,
    pub job: i16,
    pub face: i32,
    pub hair: i32,
    pub skin: i32,
    pub gender: i16,
    pub map_id: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
}

pub fn build_player_enter_field(character: &ForeignCharacter) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnPlayer as i16)?;
    packet.write_int(character.id)?;
    packet.write_byte(character.level as u8)?;
    packet.write_str_with_length(&character.name)?;
    packet.write_str_with_length("")?;
    packet.write_short(0)?;
    packet.write_byte(0)?;
    packet.write_short(0)?;
    packet.write_byte(0)?;
    packet.write_bytes(&[0; 8])?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_bytes(&[0; 43])?;
    packet.write_int(0)?;
    packet.write_bytes(&[0; 61])?;
    packet.write_short(character.job)?;
    write_look(&mut packet, character)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;

    packet.write_short(character.x)?;
    packet.write_short(character.y)?;
    packet.write_byte(character.stance)?;
    packet.write_bytes(&[0; 3])?;
    packet.write_byte(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_byte(0)?;
    packet.write_byte(0)?;
    packet.write_bytes(&[0; 3])?;
    packet.write_byte(0)?;
    Ok(packet)
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

fn write_look(packet: &mut Packet, character: &ForeignCharacter) -> Result<(), NetworkError> {
    packet.write_byte((character.gender != 0) as u8)?;
    packet.write_byte(character.skin as u8)?;
    packet.write_int(character.face)?;
    packet.write_byte(0)?;
    packet.write_int(character.hair)?;
    packet.write_byte(5)?;
    packet.write_int(DEFAULT_OVERALL)?;
    packet.write_byte(7)?;
    packet.write_int(DEFAULT_SHOES)?;
    packet.write_byte(0xFF)?;
    packet.write_byte(0xFF)?;
    packet.write_int(DEFAULT_WEAPON)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    packet.write_int(0)?;
    Ok(())
}

pub fn default_spawn_position(map_id: i32) -> (i16, i16, u8) {
    match map_id {
        STARTING_MAP_ID => (STARTING_MAP_SPAWN_X, STARTING_MAP_SPAWN_Y, DEFAULT_STANCE),
        _ => (0, 0, DEFAULT_STANCE),
    }
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
