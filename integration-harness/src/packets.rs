use net::packet::build::world::messaging::{
    WHISPER_RECEIVE_MODE, WHISPER_REQUEST_MODE, WHISPER_RESULT_MODE,
};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use std::net::Ipv4Addr;

const LOGIN_PADDING_LEN: usize = 6;
const LOGIN_HWID: [u8; 4] = [0, 0, 0, 0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginStatusPacket {
    pub status: i32,
    pub account_id: Option<i32>,
    pub gender: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerListPacket {
    WorldDetails,
    EndOfList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharListPacket {
    pub characters: Vec<CharacterSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharNameResponsePacket {
    pub name: String,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacterPacket {
    pub status: u8,
    pub character: CharacterSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerRedirectPacket {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub character_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFieldPacket {
    pub character_id: i32,
    pub character_name: String,
    pub map_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperReceivePacket {
    pub sender_name: String,
    pub channel: u8,
    pub from_admin: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperResultPacket {
    pub target_name: String,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CharacterMeta {
    id: i32,
    name: String,
    map_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterTemplate {
    pub job: i32,
    pub face: i32,
    pub hair: i32,
    pub hair_color: i32,
    pub skin: i32,
    pub top: i32,
    pub bottom: i32,
    pub shoes: i32,
    pub weapon: i32,
    pub gender: u8,
}

impl CharacterTemplate {
    pub fn default_for_gender(gender: u8) -> Self {
        match gender {
            1 => Self {
                job: 1,
                face: 21000,
                hair: 31000,
                hair_color: 0,
                skin: 0,
                top: 0,
                bottom: 0,
                shoes: 0,
                weapon: 0,
                gender,
            },
            _ => Self {
                job: 1,
                face: 20000,
                hair: 30000,
                hair_color: 0,
                skin: 0,
                top: 0,
                bottom: 0,
                shoes: 0,
                weapon: 0,
                gender: 0,
            },
        }
    }
}

pub fn opcode_name(opcode: i16) -> &'static str {
    match opcode {
        x if x == SendOpcode::LoginStatus as i16 => "LoginStatus",
        x if x == SendOpcode::ServerStatus as i16 => "ServerStatus",
        x if x == SendOpcode::ServerList as i16 => "ServerList",
        x if x == SendOpcode::CharList as i16 => "CharList",
        x if x == SendOpcode::ServerIp as i16 => "ServerIp",
        x if x == SendOpcode::LastConnectedWorld as i16 => "LastConnectedWorld",
        x if x == SendOpcode::RecommendedWorlds as i16 => "RecommendedWorlds",
        x if x == SendOpcode::SetField as i16 => "SetField",
        x if x == SendOpcode::Whisper as i16 => "Whisper",
        x if x == SendOpcode::KeyMap as i16 => "KeyMap",
        _ => "Unknown",
    }
}

pub fn build_login_started() -> Packet {
    packet_with_opcode(RecvOpcode::LoginStarted as i16)
}

pub fn build_login_credentials(username: &str, password: &str) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::LoginCredentials as i16);
    packet
        .write_str_with_length(username)
        .map_err(|e| format!("failed to write username: {e}"))?;
    packet
        .write_str_with_length(password)
        .map_err(|e| format!("failed to write password: {e}"))?;
    packet
        .write_bytes(&[0; LOGIN_PADDING_LEN])
        .map_err(|e| format!("failed to write login padding: {e}"))?;
    packet
        .write_bytes(&LOGIN_HWID)
        .map_err(|e| format!("failed to write login hwid: {e}"))?;
    Ok(packet)
}

pub fn build_server_list_request() -> Packet {
    packet_with_opcode(RecvOpcode::ServerListRequest as i16)
}

pub fn build_check_char_name(name: &str) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CheckCharName as i16);
    packet
        .write_str_with_length(name)
        .map_err(|e| format!("failed to write character name for availability check: {e}"))?;
    Ok(packet)
}

pub fn build_accept_tos() -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::AcceptTOS as i16);
    packet
        .write_byte(0x01)
        .map_err(|e| format!("failed to write tos confirmation: {e}"))?;
    Ok(packet)
}

pub fn build_set_gender(gender: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::SetGender as i16);
    packet
        .write_byte(0x01)
        .map_err(|e| format!("failed to write gender confirmation flag: {e}"))?;
    packet
        .write_byte(gender)
        .map_err(|e| format!("failed to write gender value: {e}"))?;
    Ok(packet)
}

pub fn build_char_list_request(world_id: u8, channel_id: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CharListRequest as i16);
    packet
        .write_byte(world_id)
        .map_err(|e| format!("failed to write world id: {e}"))?;
    packet
        .write_byte(channel_id)
        .map_err(|e| format!("failed to write channel id: {e}"))?;
    Ok(packet)
}

pub fn build_char_select(character_id: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CharSelect as i16);
    packet
        .write_int(character_id)
        .map_err(|e| format!("failed to write character id: {e}"))?;
    packet
        .write_str_with_length("")
        .map_err(|e| format!("failed to write character select macs: {e}"))?;
    packet
        .write_str_with_length("")
        .map_err(|e| format!("failed to write character select hwid: {e}"))?;
    Ok(packet)
}

pub fn build_create_char(name: &str, template: CharacterTemplate) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CreateChar as i16);
    packet
        .write_str_with_length(name)
        .map_err(|e| format!("failed to write character name for creation: {e}"))?;
    packet
        .write_int(template.job)
        .map_err(|e| format!("failed to write character job: {e}"))?;
    packet
        .write_int(template.face)
        .map_err(|e| format!("failed to write character face: {e}"))?;
    packet
        .write_int(template.hair)
        .map_err(|e| format!("failed to write character hair: {e}"))?;
    packet
        .write_int(template.hair_color)
        .map_err(|e| format!("failed to write character hair color: {e}"))?;
    packet
        .write_int(template.skin)
        .map_err(|e| format!("failed to write character skin: {e}"))?;
    packet
        .write_int(template.top)
        .map_err(|e| format!("failed to write starter top: {e}"))?;
    packet
        .write_int(template.bottom)
        .map_err(|e| format!("failed to write starter bottom: {e}"))?;
    packet
        .write_int(template.shoes)
        .map_err(|e| format!("failed to write starter shoes: {e}"))?;
    packet
        .write_int(template.weapon)
        .map_err(|e| format!("failed to write starter weapon: {e}"))?;
    packet
        .write_byte(template.gender)
        .map_err(|e| format!("failed to write character gender: {e}"))?;
    Ok(packet)
}

pub fn build_player_logged_in(character_id: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::PlayerLoggedIn as i16);
    packet
        .write_int(character_id)
        .map_err(|e| format!("failed to write logged-in character id: {e}"))?;
    Ok(packet)
}

pub fn build_whisper(target_name: &str, message: &str) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::Whisper as i16);
    packet
        .write_byte(WHISPER_REQUEST_MODE)
        .map_err(|e| format!("failed to write whisper request mode: {e}"))?;
    packet
        .write_str_with_length(target_name)
        .map_err(|e| format!("failed to write whisper target name: {e}"))?;
    packet
        .write_str_with_length(message)
        .map_err(|e| format!("failed to write whisper message: {e}"))?;
    Ok(packet)
}

pub fn decode_login_status(packet: &Packet) -> Result<LoginStatusPacket, String> {
    expect_opcode(packet, SendOpcode::LoginStatus as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read login status opcode: {e}"))?;
    let status = cursor
        .read_int()
        .map_err(|e| format!("failed to read login status code: {e}"))?;

    if status != 0 {
        return Ok(LoginStatusPacket {
            status,
            account_id: None,
            gender: None,
        });
    }

    cursor
        .read_short()
        .map_err(|e| format!("failed to read login status flags: {e}"))?;
    let account_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read login account id: {e}"))?;
    let gender = cursor
        .read_byte()
        .map_err(|e| format!("failed to read login gender: {e}"))?;

    Ok(LoginStatusPacket {
        status,
        account_id: Some(account_id),
        gender: Some(gender),
    })
}

pub fn decode_server_list(packet: &Packet) -> Result<ServerListPacket, String> {
    expect_opcode(packet, SendOpcode::ServerList as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read server list opcode: {e}"))?;
    let marker = cursor
        .read_byte()
        .map_err(|e| format!("failed to read server list marker: {e}"))?;
    if marker == 0xFF {
        Ok(ServerListPacket::EndOfList)
    } else {
        Ok(ServerListPacket::WorldDetails)
    }
}

pub fn decode_last_connected_world(packet: &Packet) -> Result<i32, String> {
    expect_opcode(packet, SendOpcode::LastConnectedWorld as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read last connected world opcode: {e}"))?;
    cursor
        .read_int()
        .map_err(|e| format!("failed to read last connected world: {e}"))
}

pub fn decode_recommended_worlds(packet: &Packet) -> Result<(), String> {
    expect_opcode(packet, SendOpcode::RecommendedWorlds as i16)?;
    Ok(())
}

pub fn decode_char_list(packet: &Packet) -> Result<CharListPacket, String> {
    expect_opcode(packet, SendOpcode::CharList as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read char list opcode: {e}"))?;
    cursor
        .read_byte()
        .map_err(|e| format!("failed to read account status: {e}"))?;
    let count = cursor
        .read_byte()
        .map_err(|e| format!("failed to read char count: {e}"))?;

    let mut characters = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let meta = decode_character_meta(&mut cursor)?;
        skip(&mut cursor, 41)?;
        characters.push(CharacterSummary {
            id: meta.id,
            name: meta.name,
        });
    }

    Ok(CharListPacket { characters })
}

pub fn decode_char_name_response(packet: &Packet) -> Result<CharNameResponsePacket, String> {
    expect_opcode(packet, SendOpcode::CharNameResponse as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read char-name response opcode: {e}"))?;
    let name = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read char-name response name: {e}"))?;
    let unavailable_flag = cursor
        .read_byte()
        .map_err(|e| format!("failed to read char-name availability flag: {e}"))?;
    Ok(CharNameResponsePacket {
        name,
        available: unavailable_flag == 0,
    })
}

pub fn decode_new_character(packet: &Packet) -> Result<NewCharacterPacket, String> {
    expect_opcode(packet, SendOpcode::NewCharacter as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read new-character opcode: {e}"))?;
    let status = cursor
        .read_byte()
        .map_err(|e| format!("failed to read new-character status: {e}"))?;
    let meta = decode_character_meta(&mut cursor)?;
    skip(&mut cursor, 41)?;

    Ok(NewCharacterPacket {
        status,
        character: CharacterSummary {
            id: meta.id,
            name: meta.name,
        },
    })
}

pub fn decode_server_redirect(packet: &Packet) -> Result<ServerRedirectPacket, String> {
    expect_opcode(packet, SendOpcode::ServerIp as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read redirect opcode: {e}"))?;
    cursor
        .read_short()
        .map_err(|e| format!("failed to read redirect padding: {e}"))?;
    let ip_bytes = cursor
        .read_bytes(4)
        .map_err(|e| format!("failed to read redirect ip: {e}"))?;
    let port = cursor
        .read_short()
        .map_err(|e| format!("failed to read redirect port: {e}"))?;
    let character_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read redirect character id: {e}"))?;

    Ok(ServerRedirectPacket {
        ip: Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]),
        port: port as u16,
        character_id,
    })
}

pub fn decode_set_field(packet: &Packet) -> Result<SetFieldPacket, String> {
    expect_opcode(packet, SendOpcode::SetField as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read set field opcode: {e}"))?;
    skip(&mut cursor, 20)?;
    skip(&mut cursor, 9)?;
    let meta = decode_character_meta(&mut cursor)?;

    Ok(SetFieldPacket {
        character_id: meta.id,
        character_name: meta.name,
        map_id: meta.map_id,
    })
}

pub fn decode_whisper_receive(packet: &Packet) -> Result<WhisperReceivePacket, String> {
    expect_opcode(packet, SendOpcode::Whisper as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read whisper opcode: {e}"))?;
    let mode = cursor
        .read_byte()
        .map_err(|e| format!("failed to read whisper receive mode: {e}"))?;
    if mode != WHISPER_RECEIVE_MODE {
        return Err(format!(
            "expected whisper receive mode {WHISPER_RECEIVE_MODE:#04x} but got {mode:#04x}"
        ));
    }

    let sender_name = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read whisper sender name: {e}"))?;
    let channel = cursor
        .read_byte()
        .map_err(|e| format!("failed to read whisper channel: {e}"))?;
    let from_admin = cursor
        .read_byte()
        .map_err(|e| format!("failed to read whisper admin flag: {e}"))?
        != 0;
    let message = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read whisper message: {e}"))?;

    Ok(WhisperReceivePacket {
        sender_name,
        channel,
        from_admin,
        message,
    })
}

pub fn decode_whisper_result(packet: &Packet) -> Result<WhisperResultPacket, String> {
    expect_opcode(packet, SendOpcode::Whisper as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read whisper opcode: {e}"))?;
    let mode = cursor
        .read_byte()
        .map_err(|e| format!("failed to read whisper result mode: {e}"))?;
    if mode != WHISPER_RESULT_MODE {
        return Err(format!(
            "expected whisper result mode {WHISPER_RESULT_MODE:#04x} but got {mode:#04x}"
        ));
    }

    let target_name = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read whisper target name: {e}"))?;
    let success = cursor
        .read_byte()
        .map_err(|e| format!("failed to read whisper success flag: {e}"))?
        != 0;

    Ok(WhisperResultPacket {
        target_name,
        success,
    })
}

fn packet_with_opcode(opcode: i16) -> Packet {
    let mut packet = Packet::new_empty();
    packet
        .write_short(opcode)
        .expect("packet opcode write cannot fail");
    packet
}

fn expect_opcode(packet: &Packet, expected: i16) -> Result<(), String> {
    let actual = packet.opcode();
    if actual != expected {
        return Err(format!(
            "expected opcode {} ({}) but got {} ({})",
            expected,
            opcode_name(expected),
            actual,
            opcode_name(actual)
        ));
    }
    Ok(())
}

fn decode_character_meta(cursor: &mut Cursor<&[u8]>) -> Result<CharacterMeta, String> {
    let id = cursor
        .read_int()
        .map_err(|e| format!("failed to read character id: {e}"))?;
    let name_bytes = cursor
        .read_bytes(13)
        .map_err(|e| format!("failed to read character name: {e}"))?;
    let end = name_bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name_bytes.len());
    let name = String::from_utf8(name_bytes[..end].to_vec())
        .map_err(|e| format!("invalid utf-8 in character name: {e}"))?;

    skip(cursor, 1 + 1 + 4 + 4 + 8 + 8 + 8 + 1 + 2)?;
    skip(cursor, 2 * 9)?;
    skip(cursor, 2)?;
    skip(cursor, 4 + 2 + 4)?;
    let map_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read character map id: {e}"))?;
    skip(cursor, 1 + 4)?;

    Ok(CharacterMeta { id, name, map_id })
}

fn skip(cursor: &mut Cursor<&[u8]>, length: usize) -> Result<(), String> {
    cursor
        .read_bytes(length)
        .map(|_| ())
        .map_err(|e| format!("failed to skip {length} bytes: {e}"))
}
