use net::packet::build::world::messaging::{
    WHISPER_RECEIVE_MODE, WHISPER_REQUEST_MODE, WHISPER_RESULT_MODE,
};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::net::Ipv4Addr;

const LOGIN_PADDING_LEN: usize = 6;
const LOGIN_HWID: [u8; 4] = [0, 0, 0, 0];
const MOVEMENT_HEADER_LEN: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub id: i32,
    pub name: String,
    /// Visible equipment by body slot, as shown in the character list.
    pub visible_equips: BTreeMap<u8, i32>,
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
    /// 0 available, 1 taken, 2 not allowed.
    pub result: u8,
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

pub struct ChannelChangePacket {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFieldPacket {
    pub character_id: i32,
    pub character_name: String,
    pub map_id: i32,
    /// The portal the character appears at.
    pub spawn_point: u8,
    pub level: u8,
    pub exp: i32,
    pub job: i16,
    pub str: i16,
    pub max_hp: i16,
    pub fame: i16,
    pub hair: i32,
    pub ap: i16,
    pub sp: i16,
    /// Skill levels by skill id.
    pub skills: BTreeMap<i32, i32>,
    pub quests: std::collections::BTreeMap<i16, u8>,
    /// Progress strings (kill counts) of started quests.
    pub quest_progress: BTreeMap<i16, String>,
    /// Equipped item ids by (positive) equipped slot.
    pub equipped: BTreeMap<i16, i32>,
    /// How many of each item the bags hold (not counting equipped items).
    pub items: BTreeMap<i32, i32>,
    pub meso: i32,
    /// Slots in the Equip, Use, Set-up, Etc and Cash bags.
    pub slot_limits: [u8; 5],
    /// The Monster Book's cover card and each card's copies, by card id.
    pub monster_book_cover: i32,
    pub monster_book: BTreeMap<i32, u8>,
    /// The name Blessing of the Fairy's tooltip shows, if any.
    pub linked_name: Option<String>,
    /// The crush rings, then the friendship rings, the character carries.
    pub couple_rings: Vec<CoupleRingRecord>,
}

/// A crush or friendship ring record of the character data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoupleRingRecord {
    pub partner_id: i32,
    pub partner_name: String,
    pub ring_id: i64,
    pub partner_ring_id: i64,
    /// A friendship ring's item; crush records carry none.
    pub item_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFieldWarpPacket {
    pub map_id: i32,
    pub spawn_point: u8,
    /// Where the player appears instead of at the spawn point.
    pub chase: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPlayerPacket {
    pub character_id: i32,
    pub level: u8,
    pub character_name: String,
    /// The guild name under theirs, or "" for none.
    pub guild: String,
    pub visible_equips: BTreeMap<u8, i32>,
    /// The effect item they show, or 0.
    pub item_effect: i32,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    /// The pets out: (item id, name, serial), in list order.
    pub pets: Vec<(i32, String, i64)>,
    /// The note on the chalkboard they hold up, if any.
    pub chalkboard: Option<String>,
    /// Whether their Berserk aura shows.
    pub berserk: bool,
    /// Their ring blocks.
    pub rings: RingLooks,
}

/// The ring blocks that end a look: the crush and friendship rings'
/// (serial, partner's serial, item) and the marriage ring's (self, partner,
/// ring), each if shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RingLooks {
    pub crush: Option<(i64, i64, i32)>,
    pub friendship: Option<(i64, i64, i32)>,
    pub marriage: Option<(i32, i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnNpcPacket {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTextPacket {
    pub character_id: i32,
    pub from_admin: bool,
    pub message: String,
    pub show: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlayerPacket {
    pub character_id: i32,
    pub movement_bytes: Vec<u8>,
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
    spawn_point: u8,
    level: u8,
    exp: i32,
    job: i16,
    str: i16,
    max_hp: i16,
    fame: i16,
    hair: i32,
    ap: i16,
    sp: i16,
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
                top: 1041002,
                bottom: 1061002,
                shoes: 1072001,
                weapon: 1302000,
                gender,
            },
            _ => Self {
                job: 1,
                face: 20000,
                hair: 30000,
                hair_color: 0,
                skin: 0,
                top: 1040002,
                bottom: 1060002,
                shoes: 1072001,
                weapon: 1302000,
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
        x if x == SendOpcode::ChangeChannel as i16 => "ChangeChannel",
        x if x == SendOpcode::LastConnectedWorld as i16 => "LastConnectedWorld",
        x if x == SendOpcode::RecommendedWorlds as i16 => "RecommendedWorlds",
        x if x == SendOpcode::SetField as i16 => "SetField",
        x if x == SendOpcode::SetCashShop as i16 => "SetCashShop",
        x if x == SendOpcode::QueryCashResult as i16 => "QueryCashResult",
        x if x == SendOpcode::CashShopOperation as i16 => "CashShopOperation",
        x if x == SendOpcode::CheckNewNameResult as i16 => "CheckNewNameResult",
        x if x == SendOpcode::NameChangeRulesResult as i16 => "NameChangeRulesResult",
        x if x == SendOpcode::CancelNameChangeResult as i16 => "CancelNameChangeResult",
        x if x == SendOpcode::NoNameChange as i16 => "NoNameChange",
        x if x == SendOpcode::Whisper as i16 => "Whisper",
        x if x == SendOpcode::SpawnPlayer as i16 => "SpawnPlayer",
        x if x == SendOpcode::SpawnNpc as i16 => "SpawnNpc",
        x if x == SendOpcode::RemovePlayerFromMap as i16 => "RemovePlayerFromMap",
        x if x == SendOpcode::ChatText as i16 => "ChatText",
        x if x == SendOpcode::MovePlayer as i16 => "MovePlayer",
        x if x == SendOpcode::KeyMap as i16 => "KeyMap",
        x if x == SendOpcode::MacroSysDataInit as i16 => "MacroSysDataInit",
        x if x == SendOpcode::AutoHpPot as i16 => "AutoHpPot",
        x if x == SendOpcode::AutoMpPot as i16 => "AutoMpPot",
        x if x == SendOpcode::QuickSlotInit as i16 => "QuickSlotInit",
        x if x == SendOpcode::FieldEffect as i16 => "FieldEffect",
        x if x == SendOpcode::StatChange as i16 => "StatChange",
        x if x == SendOpcode::ShowStatusInfo as i16 => "ShowStatusInfo",
        x if x == SendOpcode::UpdateSkills as i16 => "UpdateSkills",
        x if x == SendOpcode::UpdateQuestInfo as i16 => "UpdateQuestInfo",
        x if x == SendOpcode::PlayerHint as i16 => "PlayerHint",
        x if x == SendOpcode::ShowItemGainInChat as i16 => "ShowItemGainInChat",
        x if x == SendOpcode::InventoryOperation as i16 => "InventoryOperation",
        x if x == SendOpcode::UpdateCharLook as i16 => "UpdateCharLook",
        x if x == SendOpcode::SpawnMonster as i16 => "SpawnMonster",
        x if x == SendOpcode::SpawnMonsterControl as i16 => "SpawnMonsterControl",
        x if x == SendOpcode::MoveMonster as i16 => "MoveMonster",
        x if x == SendOpcode::KillMonster as i16 => "KillMonster",
        x if x == SendOpcode::DropItemFromMapObject as i16 => "DropItemFromMapObject",
        x if x == SendOpcode::RemoveItemFromMap as i16 => "RemoveItemFromMap",
        x if x == SendOpcode::MonsterBookSetCard as i16 => "MonsterBookSetCard",
        x if x == SendOpcode::MonsterBookSetCover as i16 => "MonsterBookSetCover",
        x if x == SendOpcode::SpawnReactor as i16 => "SpawnReactor",
        x if x == SendOpcode::HitReactor as i16 => "HitReactor",
        x if x == SendOpcode::RemoveReactor as i16 => "RemoveReactor",
        x if x == SendOpcode::FamilyInfo as i16 => "FamilyInfo",
        x if x == SendOpcode::FamilyPrivilegeList as i16 => "FamilyPrivilegeList",
        x if x == SendOpcode::FamilySetPrivilege as i16 => "FamilySetPrivilege",
        x if x == SendOpcode::MarriageRequest as i16 => "MarriageRequest",
        x if x == SendOpcode::MarriageResult as i16 => "MarriageResult",
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
        .write_byte(0)
        .map_err(|e| format!("failed to write char list padding byte: {e}"))?;
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

/// PLAYER_LOGGEDIN as a v83 client sends it: just the character id. The
/// server knows the channel from the port the connection arrived on.
pub fn build_player_logged_in(character_id: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::PlayerLoggedIn as i16);
    packet
        .write_int(character_id)
        .map_err(|e| format!("failed to write logged-in character id: {e}"))?;
    Ok(packet)
}

pub fn build_change_channel(channel_id: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::ChangeChannel as i16);
    packet
        .write_byte(channel_id)
        .map_err(|e| format!("failed to write change-channel id: {e}"))?;
    packet
        .write_int(0)
        .map_err(|e| format!("failed to write change-channel tick: {e}"))?;
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

pub fn build_all_chat(message: &str, show: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::AllChat as i16);
    packet
        .write_str_with_length(message)
        .map_err(|e| format!("failed to write all-chat message: {e}"))?;
    packet
        .write_byte(show)
        .map_err(|e| format!("failed to write all-chat show flag: {e}"))?;
    Ok(packet)
}

pub fn build_face_expression(emote: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::FaceExpression as i16);
    packet
        .write_int(emote)
        .map_err(|e| format!("failed to write face-expression emote: {e}"))?;
    Ok(packet)
}

/// USE_CHAIR: sit on the portable chair `item_id`.
pub fn build_use_chair(item_id: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::UseChair as i16);
    packet
        .write_int(item_id)
        .map_err(|e| format!("failed to write chair item: {e}"))?;
    Ok(packet)
}

/// CANCEL_CHAIR: stand up from map seat `seat`, or -1 for a portable chair.
pub fn build_cancel_chair(seat: i16) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CancelChair as i16);
    packet
        .write_short(seat)
        .map_err(|e| format!("failed to write chair seat: {e}"))?;
    Ok(packet)
}

pub fn build_player_move(movement_bytes: &[u8]) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::PlayerMove as i16);
    packet
        .write_bytes(&[0; MOVEMENT_HEADER_LEN])
        .map_err(|e| format!("failed to write movement header: {e}"))?;
    packet
        .write_bytes(movement_bytes)
        .map_err(|e| format!("failed to write movement bytes: {e}"))?;
    Ok(packet)
}

/// SHOP on the status bar.
pub fn build_enter_cash_shop() -> Packet {
    packet_with_opcode(RecvOpcode::EnterCashShop as i16)
}

/// The Cash Shop's exit button: CHANGE_MAP with no body.
pub fn build_leave_cash_shop() -> Packet {
    packet_with_opcode(RecvOpcode::ChangeMap as i16)
}

/// CASHSHOP_OPERATION buy: pay with `cash_type` (1 NX Credit, 2 Maple
/// Points, 4 NX Prepaid) for the commodity `sn`.
pub fn build_cash_shop_buy(cash_type: i32, sn: i32) -> Result<Packet, String> {
    cash_shop_purchase(0x03, cash_type, sn)
}

/// CHECK_CASH (0xE4): the Cash Shop's Check button asks for the balances.
pub fn build_check_cash() -> Packet {
    packet_with_opcode(RecvOpcode::CheckCash as i16)
}

/// COUPON_CODE (0xE6): the Coupon dialog's code, after an empty string.
pub fn build_coupon_code(code: &str) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CouponCode as i16);
    packet
        .write_str_with_length("")
        .and_then(|_| packet.write_str_with_length(code))
        .map_err(|e| e.to_string())?;
    Ok(packet)
}

/// CASHSHOP_OPERATION 0x1E: buy package `sn`, paying with `cash_type`.
pub fn build_cash_shop_buy_package(cash_type: i32, sn: i32) -> Result<Packet, String> {
    cash_shop_purchase(0x1E, cash_type, sn)
}

fn cash_shop_purchase(action: u8, cash_type: i32, sn: i32) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    packet
        .write_bytes(&[action, 0])
        .map_err(|e| format!("failed to write purchase action: {e}"))?;
    packet
        .write_int(cash_type)
        .map_err(|e| format!("failed to write purchase cash type: {e}"))?;
    packet
        .write_int(sn)
        .map_err(|e| format!("failed to write purchase serial number: {e}"))?;
    Ok(packet)
}

/// CASHSHOP_OPERATION 0x06, or 0x07 for the storage (`bag` `None`): buy four
/// slots with a slot button, paying with `cash_type`.
pub fn build_cash_shop_expand(cash_type: i32, bag: Option<u8>) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    let action = if bag.is_some() { 0x06 } else { 0x07 };
    packet
        .write_bytes(&[action, 0])
        .map_err(|e| format!("failed to write expansion action: {e}"))?;
    packet
        .write_int(cash_type)
        .map_err(|e| format!("failed to write expansion cash type: {e}"))?;
    packet
        .write_byte(0)
        .map_err(|e| format!("failed to write expansion mode: {e}"))?;
    if let Some(bag) = bag {
        packet
            .write_byte(bag)
            .map_err(|e| format!("failed to write expansion bag: {e}"))?;
    }
    Ok(packet)
}

/// CASHSHOP_OPERATION gift: the birthday code, the serial number, and the
/// receiver's name and message.
pub fn build_cash_shop_gift(
    birthday: i32,
    sn: i32,
    to: &str,
    message: &str,
) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    packet
        .write_byte(0x04)
        .map_err(|e| format!("failed to write gift action: {e}"))?;
    packet
        .write_int(birthday)
        .map_err(|e| format!("failed to write gift birthday code: {e}"))?;
    packet
        .write_int(sn)
        .map_err(|e| format!("failed to write gift serial number: {e}"))?;
    packet
        .write_str_with_length(to)
        .map_err(|e| format!("failed to write gift receiver: {e}"))?;
    packet
        .write_str_with_length(message)
        .map_err(|e| format!("failed to write gift message: {e}"))?;
    Ok(packet)
}

/// CASHSHOP_OPERATION 0x1D (a crush ring) or 0x23 (a friendship ring):
/// buy a pair with the balance `cash_type`, one for `to`.
pub fn build_cash_shop_buy_ring(
    friendship: bool,
    birthday: i32,
    cash_type: i32,
    sn: i32,
    to: &str,
    message: &str,
) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    let write = |packet: &mut Packet| -> std::io::Result<()> {
        packet.write_byte(if friendship { 0x23 } else { 0x1D })?;
        packet.write_int(birthday)?;
        packet.write_int(cash_type)?;
        packet.write_int(sn)?;
        packet.write_str_with_length(to)?;
        packet.write_str_with_length(message)?;
        Ok(())
    };
    write(&mut packet).map_err(|e| format!("failed to write ring purchase: {e}"))?;
    Ok(packet)
}

/// CASHSHOP_OPERATION wish list: ten serial numbers, 0 for an empty slot.
pub fn build_cash_shop_wish_list(sns: &[i32; 10]) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    packet
        .write_byte(0x05)
        .map_err(|e| format!("failed to write wish list action: {e}"))?;
    for &sn in sns {
        packet
            .write_int(sn)
            .map_err(|e| format!("failed to write wish list slot: {e}"))?;
    }
    Ok(packet)
}

/// CASHSHOP_OPERATION take out (0x0D): move Cash Inventory item `cash_id`
/// to `slot` of bag `kind`.
pub fn build_cash_shop_take_out(cash_id: i64, kind: u8, slot: i16) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    let write = |e: std::io::Error| format!("failed to write take out: {e}");
    packet.write_byte(0x0D).map_err(write)?;
    packet.write_long(cash_id).map_err(write)?;
    packet.write_byte(kind).map_err(write)?;
    packet.write_short(slot).map_err(write)?;
    Ok(packet)
}

/// CASHSHOP_OPERATION put back (0x0E): return item `cash_id` from bag
/// `kind` to the Cash Inventory.
pub fn build_cash_shop_put_back(cash_id: i64, kind: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::CashShopOperation as i16);
    let write = |e: std::io::Error| format!("failed to write put back: {e}");
    packet.write_byte(0x0E).map_err(write)?;
    packet.write_long(cash_id).map_err(write)?;
    packet.write_byte(kind).map_err(write)?;
    Ok(packet)
}

pub fn build_change_map(target_map_id: i32, portal_name: &str) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::ChangeMap as i16);
    packet
        .write_byte(0)
        .map_err(|e| format!("failed to write change-map death flag: {e}"))?;
    packet
        .write_int(target_map_id)
        .map_err(|e| format!("failed to write change-map target id: {e}"))?;
    packet
        .write_str_with_length(portal_name)
        .map_err(|e| format!("failed to write change-map portal name: {e}"))?;
    packet
        .write_byte(0)
        .map_err(|e| format!("failed to write change-map padding byte: {e}"))?;
    packet
        .write_short(0)
        .map_err(|e| format!("failed to write change-map wheel flag: {e}"))?;
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
        let look = decode_look(&mut cursor)?;
        // View-all flag and ranking flag (ranking disabled).
        skip(&mut cursor, 2)?;
        characters.push(CharacterSummary {
            id: meta.id,
            name: meta.name,
            visible_equips: look,
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
    let result = cursor
        .read_byte()
        .map_err(|e| format!("failed to read char-name result: {e}"))?;
    Ok(CharNameResponsePacket { name, result })
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
    let look = decode_look(&mut cursor)?;
    skip(&mut cursor, 2)?;

    Ok(NewCharacterPacket {
        status,
        character: CharacterSummary {
            id: meta.id,
            name: meta.name,
            visible_equips: look,
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

pub fn decode_channel_change(packet: &Packet) -> Result<ChannelChangePacket, String> {
    expect_opcode(packet, SendOpcode::ChangeChannel as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read change-channel opcode: {e}"))?;
    cursor
        .read_byte()
        .map_err(|e| format!("failed to read change-channel flag: {e}"))?;
    let octets = cursor
        .read_bytes(4)
        .map_err(|e| format!("failed to read change-channel ip: {e}"))?;
    let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
    let port = cursor
        .read_short()
        .map_err(|e| format!("failed to read change-channel port: {e}"))? as u16;

    Ok(ChannelChangePacket { ip, port })
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
    cursor.read_byte().map_err(|e| e.to_string())?; // buddy capacity
    let linked_name = if cursor.read_byte().map_err(|e| e.to_string())? != 0 {
        Some(cursor.read_str_with_length().map_err(|e| e.to_string())?)
    } else {
        None
    };
    let meso = cursor.read_int().map_err(|e| e.to_string())?;
    let mut slot_limits = [0; 5];
    for limit in &mut slot_limits {
        *limit = cursor.read_byte().map_err(|e| e.to_string())?;
    }
    skip(&mut cursor, 8)?; // timestamp

    // Equipped, equipped cash, and equip bag: short slot lists ending in 0.
    let mut equipped = BTreeMap::new();
    let mut items = BTreeMap::new();
    for list in 0..3 {
        loop {
            let slot = cursor.read_short().map_err(|e| e.to_string())?;
            if slot == 0 {
                break;
            }
            let item_id = decode_item(&mut cursor)?;
            if list == 0 {
                equipped.insert(slot, item_id);
            } else if list == 2 {
                *items.entry(item_id).or_insert(0) += 1;
            }
        }
    }
    skip(&mut cursor, 2)?;
    // Use, setup, etc, and cash bags: byte slot lists ending in 0.
    for _ in 0..4 {
        while cursor.read_byte().map_err(|e| e.to_string())? != 0 {
            let (item_id, quantity) = decode_item_stack(&mut cursor)?;
            *items.entry(item_id).or_insert(0) += i32::from(quantity);
        }
    }
    let mut skills = BTreeMap::new();
    for _ in 0..cursor.read_short().map_err(|e| e.to_string())? {
        let skill_id = cursor.read_int().map_err(|e| e.to_string())?;
        skills.insert(skill_id, cursor.read_int().map_err(|e| e.to_string())?);
        skip(&mut cursor, 8)?;
        if (skill_id / 10_000) % 10 == 2 {
            skip(&mut cursor, 4)?;
        }
    }
    let cooldowns = cursor.read_short().map_err(|e| e.to_string())?;
    skip(&mut cursor, cooldowns.max(0) as usize * 6)?;

    let mut quests = std::collections::BTreeMap::new();
    let mut quest_progress = BTreeMap::new();
    let started = cursor.read_short().map_err(|e| e.to_string())?;
    if started < 0 {
        return Err("negative quest count".into());
    }
    for _ in 0..started {
        let id = cursor.read_short().map_err(|e| e.to_string())?;
        quest_progress.insert(
            id,
            cursor.read_str_with_length().map_err(|e| e.to_string())?,
        );
        quests.insert(id, 1);
    }
    let completed = cursor.read_short().map_err(|e| e.to_string())?;
    if completed < 0 {
        return Err("negative quest count".into());
    }
    for _ in 0..completed {
        let id = cursor.read_short().map_err(|e| e.to_string())?;
        cursor.read_long().map_err(|e| e.to_string())?;
        quests.insert(id, 2);
    }

    // Minigame records (the server sends none), the crush and friendship
    // rings, the couple's 0x30-byte marriage records, then the teleport
    // rocks.
    if cursor.read_short().map_err(|e| e.to_string())? != 0 {
        return Err("unexpected minigame records".into());
    }
    let mut couple_rings = Vec::new();
    for friendship in [false, true] {
        for _ in 0..cursor.read_short().map_err(|e| e.to_string())? {
            let partner_id = cursor.read_int().map_err(|e| e.to_string())?;
            let name = cursor.read_bytes(13).map_err(|e| e.to_string())?;
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            couple_rings.push(CoupleRingRecord {
                partner_id,
                partner_name: String::from_utf8_lossy(&name[..end]).into_owned(),
                ring_id: cursor.read_long().map_err(|e| e.to_string())?,
                partner_ring_id: cursor.read_long().map_err(|e| e.to_string())?,
                item_id: if friendship {
                    Some(cursor.read_int().map_err(|e| e.to_string())?)
                } else {
                    None
                },
            });
        }
    }
    let marriages = cursor.read_short().map_err(|e| e.to_string())?;
    skip(
        &mut cursor,
        usize::try_from(marriages).map_err(|_| "negative marriage count")? * 0x30,
    )?;
    skip(&mut cursor, 15 * 4)?;
    let monster_book_cover = cursor.read_int().map_err(|e| e.to_string())?;
    cursor.read_byte().map_err(|e| e.to_string())?;
    let mut monster_book = BTreeMap::new();
    for _ in 0..cursor.read_short().map_err(|e| e.to_string())? {
        let card = 2_380_000 + i32::from(cursor.read_short().map_err(|e| e.to_string())?);
        monster_book.insert(card, cursor.read_byte().map_err(|e| e.to_string())?);
    }

    Ok(SetFieldPacket {
        character_id: meta.id,
        character_name: meta.name,
        map_id: meta.map_id,
        spawn_point: meta.spawn_point,
        level: meta.level,
        exp: meta.exp,
        job: meta.job,
        str: meta.str,
        max_hp: meta.max_hp,
        fame: meta.fame,
        hair: meta.hair,
        ap: meta.ap,
        sp: meta.sp,
        skills,
        quests,
        quest_progress,
        equipped,
        items,
        meso,
        slot_limits,
        monster_book_cover,
        monster_book,
        linked_name,
        couple_rings,
    })
}

/// AUTO_HP_POT or AUTO_MP_POT: the potion's item id, or 0.
pub fn decode_auto_potion(packet: &Packet) -> Result<i32, String> {
    let mut cursor = Cursor::new(&packet.bytes[..]);
    skip(&mut cursor, 2)?;
    cursor
        .read_int()
        .map_err(|error| format!("auto potion: {error}"))
}

/// QUICKSLOT_INIT: the eight keys, or None for the client's defaults.
pub fn decode_quick_slot_init(packet: &Packet) -> Result<Option<[i32; 8]>, String> {
    expect_opcode(packet, SendOpcode::QuickSlotInit as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    skip(&mut cursor, 2)?;
    let set = cursor
        .read_byte()
        .map_err(|error| format!("quick slots: {error}"))?;
    if set == 0 {
        return Ok(None);
    }
    let mut keys = [0; 8];
    for key in &mut keys {
        *key = cursor
            .read_int()
            .map_err(|error| format!("quick slots: {error}"))?;
    }
    Ok(Some(keys))
}

/// KEYMAP: `(kind, action)` for each of the 90 keys.
pub fn decode_keymap(packet: &Packet) -> Result<Vec<(u8, i32)>, String> {
    expect_opcode(packet, SendOpcode::KeyMap as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    skip(&mut cursor, 2 + 1)?;
    (0..90)
        .map(|key| {
            let kind = cursor
                .read_byte()
                .map_err(|e| format!("failed to read key {key} kind: {e}"))?;
            let action = cursor
                .read_int()
                .map_err(|e| format!("failed to read key {key} action: {e}"))?;
            Ok((kind, action))
        })
        .collect()
}

/// A skill macro as sent: its name, whether it is used silently, and its
/// three skills.
pub type SkillMacroEntry = (String, bool, [i32; 3]);

pub fn decode_skill_macros(packet: &Packet) -> Result<Vec<SkillMacroEntry>, String> {
    expect_opcode(packet, SendOpcode::MacroSysDataInit as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    skip(&mut cursor, 2)?;
    let count = cursor.read_byte().map_err(|e| e.to_string())?;
    (0..count)
        .map(|_| {
            let name = cursor.read_str_with_length().map_err(|e| e.to_string())?;
            let silent = cursor.read_byte().map_err(|e| e.to_string())? != 0;
            let mut skills = [0; 3];
            for skill in &mut skills {
                *skill = cursor.read_int().map_err(|e| e.to_string())?;
            }
            Ok((name, silent, skills))
        })
        .collect()
}

/// SKILL_MACRO: the whole list, with `count` in place of its length.
pub fn build_skill_macro_change(macros: &[SkillMacroEntry], count: u8) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::SkillMacro as i16);
    packet.write_byte(count).map_err(|e| e.to_string())?;
    for (name, silent, skills) in macros {
        packet
            .write_str_with_length(name)
            .map_err(|e| e.to_string())?;
        packet
            .write_byte(u8::from(*silent))
            .map_err(|e| e.to_string())?;
        for skill in skills {
            packet.write_int(*skill).map_err(|e| e.to_string())?;
        }
    }
    Ok(packet)
}

/// CHANGE_KEYMAP mode 0 with `(key, kind, action)` bindings.
pub fn build_change_keymap(bindings: &[(i32, u8, i32)]) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::ChangeKeybinds as i16);
    packet.write_int(0).map_err(|e| e.to_string())?;
    packet
        .write_int(bindings.len() as i32)
        .map_err(|e| e.to_string())?;
    for &(key, kind, action) in bindings {
        packet.write_int(key).map_err(|e| e.to_string())?;
        packet.write_byte(kind).map_err(|e| e.to_string())?;
        packet.write_int(action).map_err(|e| e.to_string())?;
    }
    Ok(packet)
}

/// QUICK_SLOT: the client's eight quick slot keys.
pub fn build_quick_slot(keys: &[i32]) -> Result<Packet, String> {
    let mut packet = packet_with_opcode(RecvOpcode::QuickSlot as i16);
    for key in keys {
        packet.write_int(*key).map_err(|e| e.to_string())?;
    }
    Ok(packet)
}

pub fn decode_set_field_warp(packet: &Packet) -> Result<SetFieldWarpPacket, String> {
    expect_opcode(packet, SendOpcode::SetField as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read set-field warp opcode: {e}"))?;
    cursor
        .read_int()
        .map_err(|e| format!("failed to read set-field warp channel: {e}"))?;
    cursor
        .read_int()
        .map_err(|e| format!("failed to read set-field warp padding int: {e}"))?;
    cursor
        .read_byte()
        .map_err(|e| format!("failed to read set-field warp padding byte: {e}"))?;
    let map_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read set-field warp map id: {e}"))?;
    let spawn_point = cursor
        .read_byte()
        .map_err(|e| format!("failed to read set-field warp spawn point: {e}"))?;
    let read = |cursor: &mut Cursor<&[u8]>| -> Result<Option<(i32, i32)>, std::io::Error> {
        cursor.read_short()?; // HP
        if cursor.read_byte()? == 0 {
            return Ok(None);
        }
        Ok(Some((cursor.read_int()?, cursor.read_int()?)))
    };
    let chase =
        read(&mut cursor).map_err(|e| format!("failed to read set-field warp chase: {e}"))?;

    Ok(SetFieldWarpPacket {
        map_id,
        spawn_point,
        chase,
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

pub fn decode_spawn_player(packet: &Packet) -> Result<SpawnPlayerPacket, String> {
    expect_opcode(packet, SendOpcode::SpawnPlayer as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-player opcode: {e}"))?;
    let character_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read spawn-player character id: {e}"))?;
    let level = cursor
        .read_byte()
        .map_err(|e| format!("failed to read spawn-player level: {e}"))?;
    let character_name = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read spawn-player name: {e}"))?;
    let guild = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read spawn-player guild: {e}"))?;
    // The emblem, the fixed foreign-buff block, then job.
    skip(&mut cursor, 6 + 8 + 4 + 4 + 4 + 43 + 65 + 2)?;
    let visible_equips = decode_look(&mut cursor)?;
    // Item 5110000 count, item effect, chair.
    skip(&mut cursor, 4)?;
    let item_effect = cursor
        .read_int()
        .map_err(|e| format!("failed to read spawn-player item effect: {e}"))?;
    skip(&mut cursor, 4)?;
    let x = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-player x: {e}"))?;
    let y = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-player y: {e}"))?;
    let stance = cursor
        .read_byte()
        .map_err(|e| format!("failed to read spawn-player stance: {e}"))?;
    let pets = decode_spawn_player_pets(&mut cursor)
        .map_err(|e| format!("failed to read spawn-player pets: {e}"))?;
    let chalkboard = decode_spawn_player_chalkboard(&mut cursor)
        .map_err(|e| format!("failed to read spawn-player chalkboard: {e}"))?;
    let rings = decode_ring_looks(&mut cursor)?;
    skip(&mut cursor, 1)?; // no new year cards
    let berserk = cursor
        .read_byte()
        .map_err(|e| format!("failed to read spawn-player berserk: {e}"))?
        != 0;

    Ok(SpawnPlayerPacket {
        character_id,
        level,
        character_name,
        guild,
        visible_equips,
        item_effect,
        x,
        y,
        stance,
        pets,
        chalkboard,
        berserk,
        rings,
    })
}

/// The crush, friendship and marriage ring blocks that end a look in
/// SPAWN_PLAYER and UPDATE_CHAR_LOOK.
pub fn decode_ring_looks(cursor: &mut Cursor<&[u8]>) -> Result<RingLooks, String> {
    let pair = |cursor: &mut Cursor<&[u8]>| -> Result<_, String> {
        if cursor.read_byte().map_err(|e| e.to_string())? == 0 {
            return Ok(None);
        }
        let ring = cursor.read_long().map_err(|e| e.to_string())?;
        let partner = cursor.read_long().map_err(|e| e.to_string())?;
        let item = cursor.read_int().map_err(|e| e.to_string())?;
        Ok(Some((ring, partner, item)))
    };
    let crush = pair(cursor)?;
    let friendship = pair(cursor)?;
    let marriage = if cursor.read_byte().map_err(|e| e.to_string())? == 0 {
        None
    } else {
        let mut int = || cursor.read_int().map_err(|e| e.to_string());
        Some((int()?, int()?, int()?))
    };
    Ok(RingLooks {
        crush,
        friendship,
        marriage,
    })
}

/// UPDATE_CHAR_LOOK: the character and their ring blocks.
pub fn decode_char_look(packet: &Packet) -> Result<(i32, RingLooks), String> {
    expect_opcode(packet, SendOpcode::UpdateCharLook as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[2..]);
    let id = cursor.read_int().map_err(|e| e.to_string())?;
    skip(&mut cursor, 1)?; // flags: the look
    decode_look(&mut cursor)?;
    Ok((id, decode_ring_looks(&mut cursor)?))
}

/// After the foothold and a byte, each pet behind a 1, then a 0.
fn decode_spawn_player_pets(
    cursor: &mut Cursor<&[u8]>,
) -> Result<Vec<(i32, String, i64)>, std::io::Error> {
    cursor.read_short()?;
    cursor.read_byte()?;
    let mut pets = Vec::new();
    while cursor.read_byte()? == 1 {
        let pet = (
            cursor.read_int()?,
            cursor.read_str_with_length()?,
            cursor.read_long()?,
        );
        // Position, stance, foothold, name tag and chat balloon.
        cursor.read_bytes(2 + 2 + 1 + 2 + 1 + 1)?;
        pets.push(pet);
    }
    Ok(pets)
}

/// After the mount and any mini room balloon, whether a chalkboard is up
/// and its note.
fn decode_spawn_player_chalkboard(
    cursor: &mut Cursor<&[u8]>,
) -> Result<Option<String>, std::io::Error> {
    cursor.read_bytes(4 + 8)?;
    if cursor.read_byte()? != 0 {
        cursor.read_int()?;
        cursor.read_str_with_length()?;
        cursor.read_bytes(5)?;
    }
    if cursor.read_byte()? == 0 {
        return Ok(None);
    }
    cursor.read_str_with_length().map(Some)
}

pub fn decode_spawn_npc(packet: &Packet) -> Result<SpawnNpcPacket, String> {
    expect_opcode(packet, SendOpcode::SpawnNpc as i16)?;

    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc opcode: {e}"))?;
    let object_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read spawn-npc object id: {e}"))?;
    let npc_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read spawn-npc npc id: {e}"))?;
    let x = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc x: {e}"))?;
    let y = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc y: {e}"))?;
    let flip = cursor
        .read_byte()
        .map_err(|e| format!("failed to read spawn-npc flip: {e}"))?
        != 0;
    let foothold = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc foothold: {e}"))?;
    let rx0 = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc rx0: {e}"))?;
    let rx1 = cursor
        .read_short()
        .map_err(|e| format!("failed to read spawn-npc rx1: {e}"))?;

    Ok(SpawnNpcPacket {
        object_id,
        npc_id,
        x,
        y,
        flip,
        foothold,
        rx0,
        rx1,
    })
}

pub fn decode_chat_text(packet: &Packet) -> Result<ChatTextPacket, String> {
    expect_opcode(packet, SendOpcode::ChatText as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read chat-text opcode: {e}"))?;
    let character_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read chat-text character id: {e}"))?;
    let from_admin = cursor
        .read_byte()
        .map_err(|e| format!("failed to read chat-text admin flag: {e}"))?
        != 0;
    let message = cursor
        .read_str_with_length()
        .map_err(|e| format!("failed to read chat-text message: {e}"))?;
    let show = cursor
        .read_byte()
        .map_err(|e| format!("failed to read chat-text show flag: {e}"))?;

    Ok(ChatTextPacket {
        character_id,
        from_admin,
        message,
        show,
    })
}

pub fn decode_move_player(packet: &Packet) -> Result<MovePlayerPacket, String> {
    expect_opcode(packet, SendOpcode::MovePlayer as i16)?;
    let mut cursor = Cursor::new(&packet.bytes[..]);
    cursor
        .read_short()
        .map_err(|e| format!("failed to read move-player opcode: {e}"))?;
    let character_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read move-player character id: {e}"))?;
    skip(&mut cursor, 4)?;

    let total_len = cursor.get_ref().len();
    let current = cursor.position() as usize;
    let movement_bytes = cursor
        .read_bytes(total_len.saturating_sub(current))
        .map_err(|e| format!("failed to read move-player payload: {e}"))?;

    Ok(MovePlayerPacket {
        character_id,
        movement_bytes,
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

    skip(cursor, 1 + 1 + 4)?; // gender, skin, face
    let hair = cursor.read_int().map_err(|e| e.to_string())?;
    skip(cursor, 8 + 8 + 8)?;
    let level = cursor.read_byte().map_err(|e| e.to_string())?;
    let job = cursor.read_short().map_err(|e| e.to_string())?;
    let str = cursor.read_short().map_err(|e| e.to_string())?;
    skip(cursor, 2 * 4)?; // dex, int, luk, hp
    let max_hp = cursor.read_short().map_err(|e| e.to_string())?;
    skip(cursor, 2 * 2)?; // mp, max mp
    let ap = cursor.read_short().map_err(|e| e.to_string())?;
    let sp = cursor.read_short().map_err(|e| e.to_string())?;
    let exp = cursor.read_int().map_err(|e| e.to_string())?;
    let fame = cursor.read_short().map_err(|e| e.to_string())?;
    skip(cursor, 4)?;
    let map_id = cursor
        .read_int()
        .map_err(|e| format!("failed to read character map id: {e}"))?;
    let spawn_point = cursor.read_byte().map_err(|e| e.to_string())?;
    skip(cursor, 4)?;

    Ok(CharacterMeta {
        id,
        name,
        map_id,
        spawn_point,
        level,
        exp,
        job,
        str,
        max_hp,
        fame,
        hair,
        ap,
        sp,
    })
}

/// Decode an avatar look, returning visible equipment by body slot.
pub fn decode_look(cursor: &mut Cursor<&[u8]>) -> Result<BTreeMap<u8, i32>, String> {
    skip(cursor, 1 + 1 + 4 + 1 + 4)?; // gender, skin, face, megaphone flag, hair
    let mut visible = BTreeMap::new();
    for list in 0..2 {
        loop {
            let slot = cursor.read_byte().map_err(|e| e.to_string())?;
            if slot == 0xFF {
                break;
            }
            let item_id = cursor.read_int().map_err(|e| e.to_string())?;
            if list == 0 {
                visible.insert(slot, item_id);
            }
        }
    }
    skip(cursor, 4 + 3 * 4)?; // cash weapon, pet ids
    Ok(visible)
}

/// Decode one inventory item (without its slot prefix), returning its id.
fn decode_item(cursor: &mut Cursor<&[u8]>) -> Result<i32, String> {
    decode_item_stack(cursor).map(|(item_id, _)| item_id)
}

/// An item's id and quantity (1 for equipment).
pub fn decode_item_stack(cursor: &mut Cursor<&[u8]>) -> Result<(i32, i16), String> {
    let item_type = cursor.read_byte().map_err(|e| e.to_string())?;
    let item_id = cursor.read_int().map_err(|e| e.to_string())?;
    let is_cash = cursor.read_byte().map_err(|e| e.to_string())? != 0;
    if is_cash {
        skip(cursor, 8)?;
    }
    skip(cursor, 8)?; // expiration
    match item_type {
        1 => {
            skip(cursor, 1 + 1 + 15 * 2)?;
            cursor.read_str_with_length().map_err(|e| e.to_string())?;
            skip(cursor, 2)?;
            skip(cursor, if is_cash { 10 } else { 1 + 1 + 4 + 4 + 8 })?;
            skip(cursor, 8 + 4)?;
        }
        2 => {
            let quantity = cursor.read_short().map_err(|e| e.to_string())?;
            cursor.read_str_with_length().map_err(|e| e.to_string())?;
            skip(cursor, 2)?;
            if matches!(item_id / 10_000, 207 | 233) {
                skip(cursor, 8)?;
            }
            return Ok((item_id, quantity));
        }
        // A pet: name, level, closeness, fullness, date dead, attribute,
        // skill, remaining life and a short.
        3 => skip(cursor, 13 + 1 + 2 + 1 + 8 + 2 + 2 + 4 + 2)?,
        other => return Err(format!("unsupported item type {other} for item {item_id}")),
    }
    Ok((item_id, 1))
}

fn skip(cursor: &mut Cursor<&[u8]>, length: usize) -> Result<(), String> {
    cursor
        .read_bytes(length)
        .map(|_| ())
        .map_err(|e| format!("failed to skip {length} bytes: {e}"))
}
