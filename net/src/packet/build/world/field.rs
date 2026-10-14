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

/// FACIAL_EXPRESSION: another player's emote, by FACE_EXPRESSION number.
pub fn build_facial_expression(character_id: i32, emote: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::FacialExpression as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(emote)?;
    Ok(packet)
}

pub fn parse_movement_state(
    movement_bytes: &[u8],
    current: (i16, i16, u8),
) -> Option<(i16, i16, u8)> {
    parse_movement(movement_bytes, current).map(|(x, y, stance, _)| (x, y, stance))
}

/// Like `parse_movement_state`, plus the foothold the mover ended up on:
/// the last nonzero foothold of an absolute or jump-down fragment (0 means
/// airborne), if any fragment had one.
pub fn parse_movement(
    movement_bytes: &[u8],
    current: (i16, i16, u8),
) -> Option<(i16, i16, u8, Option<i16>)> {
    let mut cursor = Cursor::new(movement_bytes);
    let count = cursor.read_byte().ok()?;
    if count == 0 {
        return None;
    }

    let (mut x, mut y, mut stance) = current;
    let mut foothold = None;

    for _ in 0..count {
        let command = cursor.read_byte().ok()?;
        match command {
            0 | 5 | 17 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let _ = cursor.read_short().ok()?;
                let fh = cursor.read_short().ok()?;
                if fh != 0 {
                    foothold = Some(fh);
                }
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
                let fh = cursor.read_short().ok()?;
                if fh != 0 {
                    foothold = Some(fh);
                }
                stance = cursor.read_byte().ok()?;
                let _ = cursor.read_short().ok()?;
            }
            3 | 4 | 7 | 8 | 9 | 10 | 14 => {}
            _ => return None,
        }
    }

    Some((x, y, stance, foothold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::AvatarLook;
    use std::collections::BTreeMap;

    fn avatar() -> FieldAvatar {
        FieldAvatar {
            id: 0x0102_0304,
            name: "Tester".to_string(),
            level: 10,
            job: 100,
            look: AvatarLook {
                gender: 1,
                skin: 2,
                face: 20000,
                hair: 30030,
                visible: BTreeMap::from([(5, 1042003), (11, 1302000)]),
                masked: BTreeMap::from([(5, 1040002)]),
                cash_weapon: Some(1702000),
            },
            x: -120,
            y: 275,
            stance: 4,
        }
    }

    /// `writeForeignBuffs` in HeavenMS with no active buffs; the reference
    /// draws a random "magic spawn" int and repeats it six times.
    fn foreign_buffs(expect: &mut Expect<'_>) {
        expect.int(0).short(0).byte(0xFC).byte(1);
        expect.int(0); // morph
        expect.int(0); // buff mask (high)
        expect.int(0); // buff mask (low)
        expect.int(0).short(0).bytes(&[0; 4]); // energy charge
        expect.int(0).bytes(&[0; 11]).short(0); // dash speed
        expect.bytes(&[0; 9]).int(0).short(0).byte(0); // dash jump
        expect.long(0); // monster riding
        let magic = expect.read_int();
        expect.bytes(&[0; 8]); // speed infusion
        assert_eq!(expect.read_int(), magic);
        expect.byte(0);
        assert_eq!(expect.read_int(), magic);
        expect.short(0);
        expect.bytes(&[0; 9]); // homing beacon
        assert_eq!(expect.read_int(), magic);
        expect.int(0);
        expect.bytes(&[0; 9]); // zombify
        assert_eq!(expect.read_int(), magic);
        expect.short(0).short(0);
    }

    #[test]
    fn spawn_player_matches_reference() {
        let avatar = avatar();
        let packet = build_player_enter_field(&avatar).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnPlayer as i16);
        expect.int(0x0102_0304).byte(10).string("Tester");
        expect.string("").bytes(&[0; 6]); // no guild
        foreign_buffs(&mut expect);
        expect.short(100);
        expect.look(&avatar.look, false);
        expect.int(0).int(0).int(0); // 5110000 count, item effect, chair
        expect.short(-120).short(275).byte(4).short(0).byte(0);
        expect.byte(0); // end of pets
        expect.int(1).long(0); // mount level, exp and tiredness
        expect.byte(0); // no shop or minigame box
        expect.byte(0); // no chalkboard
        expect.byte(0).byte(0).byte(0); // crush, friendship, marriage rings
        expect.byte(0).byte(0).byte(0); // new year cards, two zero bytes
        expect.byte(0); // team
        expect.end();
    }

    #[test]
    fn leave_field_is_just_the_character_id() {
        let packet = build_player_leave_field(0x0102_0304).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::RemovePlayerFromMap as i16)
            .int(0x0102_0304)
            .end();
    }

    #[test]
    fn player_move_rebroadcasts_the_movement_bytes() {
        let movement = [1u8, 0, 0x10, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0x64, 0];
        let packet = build_player_move(0x0102_0304, &movement).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MovePlayer as i16);
        expect.int(0x0102_0304).int(0);
        expect.bytes(&movement);
        expect.end();
    }

    #[test]
    fn local_chat_matches_reference() {
        let packet = build_local_chat(0x0102_0304, "hi all", false, 1).unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::ChatText as i16)
            .int(0x0102_0304)
            .byte(0)
            .string("hi all")
            .byte(1)
            .end();

        let admin = build_local_chat(7, "notice", true, 0).unwrap();
        Expect::new(&admin.bytes)
            .short(SendOpcode::ChatText as i16)
            .int(7)
            .byte(1)
            .string("notice")
            .byte(0)
            .end();
    }
}
