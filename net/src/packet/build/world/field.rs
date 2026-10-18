use super::buff;
use super::guild::write_emblem;
use crate::codec::encode_avatar_look;
use crate::{error::NetworkError, packet::op::SendOpcode};
use game::FieldAvatar;
use packet::io::read::PktRead;
use packet::{io::write::PktWrite, Packet};
use std::io::Cursor;

/// SPAWN_PLAYER: show `avatar` to the other players on the field
/// (`spawnPlayerMapObject` in HeavenMS, with no pets or shops).
pub fn build_player_enter_field(avatar: &FieldAvatar) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpawnPlayer as i16)?;
    packet.write_int(avatar.id)?;
    packet.write_byte(avatar.level)?;
    packet.write_str_with_length(&avatar.name)?;

    // Guild name and emblem.
    let guild = avatar.guild.as_ref();
    packet.write_str_with_length(guild.map_or("", |guild| &guild.name))?;
    write_emblem(
        &mut packet,
        &guild.map(|guild| guild.emblem).unwrap_or_default(),
    )?;

    write_foreign_buffs(&mut packet, &avatar.buffs)?;

    packet.write_short(avatar.job)?;
    encode_avatar_look(&mut packet, &avatar.look, false)?;
    packet.write_int(0)?; // count of item 5110000
    packet.write_int(0)?; // item effect
    packet.write_int(avatar.chair)?;

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

/// The buffs others see, then the two-state stats' fixed blocks, which
/// are always empty here; the repeated spawn constant mirrors what the
/// reference server sends.
fn write_foreign_buffs(packet: &mut Packet, shown: &[game::ShownBuff]) -> Result<(), NetworkError> {
    const CHAR_MAGIC_SPAWN: i32 = 0x2D0F_5E1A;
    buff::write_shown(packet, shown, true)?;

    packet.write_short(0)?; // energy charge
    packet.write_int(0)?;
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

/// MOVE_PLAYER: relay a player's movement path, which starts at `start`.
pub fn build_player_move(
    character_id: i32,
    start: (i16, i16),
    movement_bytes: &[u8],
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MovePlayer as i16)?;
    packet.write_int(character_id)?;
    packet.write_short(start.0)?;
    packet.write_short(start.1)?;
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

/// SHOW_CHAIR: show the others on the map that a player sat on the portable
/// chair `item_id`, or stood up from it when `item_id` is 0.
pub fn build_show_chair(character_id: i32, item_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::ShowChair as i16)?;
    packet.write_int(character_id)?;
    packet.write_int(item_id)?;
    Ok(packet)
}

/// CANCEL_CHAIR: let the player stand up from a portable chair. (Byte 1
/// followed by a seat id would sit them on a map seat instead.)
pub fn build_cancel_chair() -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::CancelChair as i16)?;
    packet.write_byte(0)?;
    Ok(packet)
}

pub fn parse_movement_state(
    movement_bytes: &[u8],
    current: (i16, i16, u8),
) -> Option<(i16, i16, u8)> {
    parse_movement(movement_bytes, current).map(|(x, y, stance, _)| (x, y, stance))
}

/// Like `parse_movement_state`, plus the foothold the mover ended up on:
/// that of the last element with a position (0 means airborne), if any.
///
/// The elements follow a path's start position (`current`'s x and y), as
/// v83 reads them (FUN_0068a33c): each is a type byte and its fields, then a
/// stance byte and a duration short, except type 10, which has neither.
/// Elements that carry only a velocity keep the position before them.
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
            // Position, velocity, foothold (and for 15, the foothold it
            // fell from).
            0 | 5 | 15 | 17 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                cursor.read_short().ok()?;
                cursor.read_short().ok()?;
                foothold = Some(cursor.read_short().ok()?);
                if command == 15 {
                    cursor.read_short().ok()?;
                }
            }
            // Velocity.
            1 | 2 | 6 | 12 | 13 | 16 | 18 | 19 | 20 | 22 => {
                cursor.read_short().ok()?;
                cursor.read_short().ok()?;
            }
            // Position and foothold.
            3 | 4 | 7 | 8 | 9 | 11 => {
                x = cursor.read_short().ok()?;
                y = cursor.read_short().ok()?;
                foothold = Some(cursor.read_short().ok()?);
            }
            // Velocity and the foothold fallen from.
            14 => {
                cursor.read_short().ok()?;
                cursor.read_short().ok()?;
                cursor.read_short().ok()?;
            }
            10 => {
                cursor.read_byte().ok()?;
                continue;
            }
            _ => {}
        }
        stance = cursor.read_byte().ok()?;
        cursor.read_short().ok()?;
    }

    Some((x, y, stance, foothold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;
    use game::guild::{Emblem, GuildTag};
    use game::{AvatarLook, BuffStat, ShownBuff};
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
            chair: 3010000,
            buffs: Vec::new(),
            guild: None,
        }
    }

    /// `writeForeignBuffs` in HeavenMS with no active buffs; the reference
    /// draws a random "magic spawn" int and repeats it six times.
    fn foreign_buffs(expect: &mut Expect<'_>) {
        expect.int(0).short(0).byte(0xFC).byte(1);
        expect.int(0); // morph
        expect.int(0); // buff mask (high)
        empty_two_state(expect);
    }

    fn empty_two_state(expect: &mut Expect<'_>) {
        expect.int(0); // defense attack and state, then two-state data
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
        expect.int(0).int(0).int(3010000); // 5110000 count, item effect, chair
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
    fn spawn_player_carries_the_buffs_others_see() {
        let mut avatar = avatar();
        let shown = |stat, value, skill_id| ShownBuff {
            stat,
            value,
            skill_id,
        };
        avatar.buffs = vec![
            shown(BuffStat::Speed, 40, 4101004),
            shown(BuffStat::DarkSight, 1, 4001003),
        ];
        let packet = build_player_enter_field(&avatar).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnPlayer as i16);
        expect.int(0x0102_0304).byte(10).string("Tester");
        expect.string("").bytes(&[0; 6]);
        expect
            .long(0x01FC_0000_0000_0000)
            .long((1 << 39) | (1 << 42));
        expect.byte(40); // speed; Dark Sight has no data
        empty_two_state(&mut expect);
        expect.short(100);
    }

    #[test]
    fn spawn_player_carries_the_guild_name_and_emblem() {
        let mut avatar = avatar();
        avatar.guild = Some(GuildTag {
            id: 4,
            name: "Testers".into(),
            emblem: Emblem {
                background: 1001,
                background_color: 2,
                mark: 2003,
                mark_color: 4,
            },
        });
        let packet = build_player_enter_field(&avatar).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::SpawnPlayer as i16);
        expect.int(0x0102_0304).byte(10).string("Tester");
        expect
            .string("Testers")
            .short(1001)
            .byte(2)
            .short(2003)
            .byte(4);
        foreign_buffs(&mut expect);
        expect.short(100);
    }

    #[test]
    fn chair_packets_match_reference() {
        let shown = build_show_chair(0x0102_0304, 3010000).unwrap();
        Expect::new(&shown.bytes)
            .short(SendOpcode::ShowChair as i16)
            .int(0x0102_0304)
            .int(3010000)
            .end();
        let cancel = build_cancel_chair().unwrap();
        Expect::new(&cancel.bytes)
            .short(SendOpcode::CancelChair as i16)
            .byte(0)
            .end();
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
        let packet = build_player_move(0x0102_0304, (-40, 300), &movement).unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MovePlayer as i16);
        expect.int(0x0102_0304).short(-40).short(300);
        expect.bytes(&movement);
        expect.end();
    }

    #[test]
    fn movement_reads_each_element_type() {
        let movement = [
            6u8, //
            // A walk: position (16, 32), velocity, foothold 7.
            0, 0x10, 0, 0x20, 0, 0x05, 0, 0, 0, 7, 0, 4, 0x64, 0, //
            // A jump: velocity only, so the position stays.
            1, 0xF0, 0xFF, 0x9C, 0xFF, 6, 0x5A, 0, //
            // Type 10: one byte, no stance or duration.
            10, 3, //
            // A down jump: foothold 9, fallen from 8.
            15, 0x30, 0, 0x40, 0, 0, 0, 0, 0, 9, 0, 8, 0, 7, 0x10, 0, //
            // A teleport: position and foothold 11.
            4, 0x50, 0, 0x60, 0, 11, 0, 2, 0, 0, //
            // An unknown type: stance and duration only.
            21, 5, 0x10, 0,
        ];
        assert_eq!(
            parse_movement(&movement, (1, 2, 3)),
            Some((0x50, 0x60, 5, Some(11)))
        );

        // Velocity-only elements keep the start and its stance until theirs.
        let jump = [
            2u8, 1, 0, 0, 0, 0, 6, 0x5A, 0, 14, 0, 0, 0, 0, 3, 0, 7, 0x10, 0,
        ];
        assert_eq!(
            parse_movement(&jump, (100, -50, 2)),
            Some((100, -50, 7, None))
        );
        assert_eq!(parse_movement(&jump[..jump.len() - 1], (100, -50, 2)), None);
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
