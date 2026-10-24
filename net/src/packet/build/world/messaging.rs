use crate::{
    codec::item::{encode_item, SlotPrefix},
    codec::look::encode_avatar_look,
    error::NetworkError,
    packet::op::SendOpcode,
};
use game::{AvatarLook, Item};
use packet::{io::write::PktWrite, Packet};

/// Client → server: send a whisper.
pub const WHISPER_REQUEST_MODE: u8 = 0x06;
/// Server → client: the result of the client's whisper.
pub const WHISPER_RESULT_MODE: u8 = 0x0A;
/// Server → client: an incoming whisper.
pub const WHISPER_RECEIVE_MODE: u8 = 0x12;
/// Both ways: the receiver's Game Option turned a whisper away.
pub const WHISPER_BLOCKED_MODE: u8 = 0x22;
/// Client → server: find where a character is (`/find`).
pub const FIND_REQUEST_MODE: u8 = 0x05;
/// Server → client: where the character the client looked for is.
pub const FIND_RESULT_MODE: u8 = 0x09;
/// Server → client: spouse chat's answer to the sender.
const SPOUSE_CHAT_RESULT_MODE: u8 = 4;
/// Server → client: the partner's spouse chat line.
const SPOUSE_CHAT_RECEIVE_MODE: u8 = 5;

/// Where a found character is, as the asker may learn it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindLocation {
    /// On the asker's channel, on this map.
    Map(i32),
    /// On another channel, zero-based.
    Channel(u8),
}

/// How a v83 client shows a BROADCAST_MSG (FUN_00a22785), for the kinds
/// that carry only their message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Broadcast {
    /// "[Notice] " and the message in blue chat, or a dialog outside a
    /// field.
    Notice = 0,
    /// A dialog.
    Popup = 1,
    /// A megaphone's line, to the sender's channel.
    Megaphone = 2,
    /// Pink chat, as for a mob's Banish.
    Pink = 5,
}

/// BROADCAST_MSG kinds that carry the sender's channel and ear.
const SUPER_MEGAPHONE: u8 = 3;
/// BROADCAST_MSG kind for a chat line that links an item.
const ITEM_LINE: u8 = 6;
const ITEM_MEGAPHONE: u8 = 8;
const TRIPLE_MEGAPHONE: u8 = 10;
const GACHAPON: u8 = 11;

/// A message from the server, shown as `kind` says.
pub fn build_broadcast(kind: Broadcast, message: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(kind as u8)?;
    packet.write_str_with_length(message)?;
    Ok(packet)
}

/// A line in chat that links an item, or none for 0 (v83 FUN_00a22785
/// reads the item after the message).
pub fn build_item_line(message: &str, item_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(ITEM_LINE)?;
    packet.write_str_with_length(message)?;
    packet.write_int(item_id)?;
    Ok(packet)
}

/// A super megaphone's line, to every channel. `channel` is the sender's
/// zero-based channel; `ear` shows the ear that lets readers whisper back.
pub fn build_super_megaphone(line: &str, channel: u8, ear: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(SUPER_MEGAPHONE)?;
    packet.write_str_with_length(line)?;
    packet.write_byte(channel)?;
    packet.write_byte(ear as u8)?;
    Ok(packet)
}

/// The world hears that `name` won `item` from `town`'s Gachapon (report
/// c058 Q3). v83 FUN_00a22785 splits the line at " : " and shows "<name>
/// got a(n)  :  <item>  from the <town> Gachapon! Congrats!" (StringPool
/// 0x137d); a channel of -1 would name the Cash Shop Surprise instead.
pub fn build_gachapon_message(
    name: &str,
    channel: u8,
    town: &str,
    item: &Item,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(GACHAPON)?;
    packet.write_str_with_length(&format!("{name} : got a(n)"))?;
    packet.write_int(i32::from(channel))?;
    packet.write_str_with_length(town)?;
    encode_item(&mut packet, SlotPrefix::None, item)?;
    Ok(packet)
}

/// An item megaphone's line, with the item it shows, if any.
pub fn build_item_megaphone(
    line: &str,
    channel: u8,
    ear: bool,
    item: Option<&Item>,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(ITEM_MEGAPHONE)?;
    packet.write_str_with_length(line)?;
    packet.write_byte(channel)?;
    packet.write_byte(ear as u8)?;
    packet.write_byte(item.is_some() as u8)?;
    if let Some(item) = item {
        encode_item(&mut packet, SlotPrefix::None, item)?;
    }
    Ok(packet)
}

/// A triple megaphone's one to three lines. v83 reads the count after the
/// first line, then the others.
pub fn build_triple_megaphone(
    lines: &[String],
    channel: u8,
    ear: bool,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BroadcastMsg as i16)?;
    packet.write_byte(TRIPLE_MEGAPHONE)?;
    let (first, rest) = lines.split_first().ok_or(NetworkError::PacketHandlerError(
        "a triple megaphone needs a line",
    ))?;
    packet.write_str_with_length(first)?;
    packet.write_byte(lines.len() as u8)?;
    for line in rest {
        packet.write_str_with_length(line)?;
    }
    packet.write_byte(channel)?;
    packet.write_byte(ear as u8)?;
    Ok(packet)
}

/// Why an avatar megaphone was refused (recv 110, FUN_00a2a3bc): the client
/// unlocks and keeps the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarRefusal {
    /// "The waiting line is longer than 15 seconds." (StringPool 0xf84).
    Busy = b'S' as isize,
    /// Only for characters over level 10 (StringPool 0xea1).
    TooLow = b'T' as isize,
}

/// An avatar megaphone is refused.
pub fn build_avatar_megaphone_refusal(why: AvatarRefusal) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::AvatarMegaphoneResult as i16)?;
    packet.write_byte(why as u8)?;
    Ok(packet)
}

/// An avatar megaphone's banner (recv 111, FUN_00a2a486): the item, who
/// sent it from which zero-based channel, their four lines and their look.
/// The client shows it for 10 s, or until the next one replaces it.
pub fn build_avatar_megaphone(
    item_id: i32,
    name: &str,
    lines: &[String; 4],
    channel: u8,
    ear: bool,
    look: &AvatarLook,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::AvatarMegaphone as i16)?;
    packet.write_int(item_id)?;
    packet.write_str_with_length(name)?;
    for line in lines {
        packet.write_str_with_length(line)?;
    }
    packet.write_int(channel.into())?;
    packet.write_byte(ear as u8)?;
    encode_avatar_look(&mut packet, look, true)?;
    Ok(packet)
}

/// An incoming whisper. `channel` is the sender's zero-based channel.
pub fn build_whisper_receive(
    sender_name: &str,
    channel: u8,
    from_admin: bool,
    message: &str,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(WHISPER_RECEIVE_MODE)?;
    packet.write_str_with_length(sender_name)?;
    packet.write_byte(channel)?;
    packet.write_byte(from_admin as u8)?;
    packet.write_str_with_length(message)?;
    Ok(packet)
}

pub fn build_whisper_result(target_name: &str, success: bool) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(WHISPER_RESULT_MODE)?;
    packet.write_str_with_length(target_name)?;
    packet.write_byte(success as u8)?;
    Ok(packet)
}

/// Spouse chat's answer to its sender: `Some(text)` puts "name : text" in
/// their log, `None` says the partner isn't there (0x9f, FUN_00532087).
pub fn build_spouse_chat_result(text: Option<&str>) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpouseChat as i16)?;
    packet.write_byte(SPOUSE_CHAT_RESULT_MODE)?;
    packet.write_byte(text.is_some() as u8)?;
    if let Some(text) = text {
        packet.write_str_with_length(text)?;
    }
    Ok(packet)
}

/// `from`'s line to their partner, which the partner's client prints as
/// "from : text" in spouse chat's colour (FUN_00532087).
pub fn build_spouse_chat(from: &str, text: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::SpouseChat as i16)?;
    packet.write_byte(SPOUSE_CHAT_RECEIVE_MODE)?;
    packet.write_str_with_length(from)?;
    // Skipped by the client; Cosmic sends the mode again.
    packet.write_byte(SPOUSE_CHAT_RECEIVE_MODE)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

/// `target` blocks whispers, so the receiver's last one went unseen.
pub fn build_whisper_blocked(target: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(WHISPER_BLOCKED_MODE)?;
    packet.write_str_with_length(target)?;
    // Nonzero would say "Unable to find" instead.
    packet.write_byte(0)?;
    Ok(packet)
}

/// The answer to a find. A character who isn't online gets the failed
/// whisper result instead.
pub fn build_find_result(
    target_name: &str,
    location: FindLocation,
) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Whisper as i16)?;
    packet.write_byte(FIND_RESULT_MODE)?;
    packet.write_str_with_length(target_name)?;
    match location {
        FindLocation::Map(map_id) => {
            packet.write_byte(1)?;
            packet.write_int(map_id)?;
            // The position on the map; the client shows only the map.
            packet.write_int(0)?;
            packet.write_int(0)?;
        }
        FindLocation::Channel(channel) => {
            packet.write_byte(3)?;
            packet.write_int(i32::from(channel))?;
        }
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn spouse_chat_echoes_the_text_or_says_no_one_heard() {
        let packet = build_spouse_chat_result(Some("hi")).unwrap();
        Expect::new(&packet.bytes)
            .short(0x88)
            .byte(4)
            .byte(1)
            .string("hi")
            .end();
        let packet = build_spouse_chat_result(None).unwrap();
        Expect::new(&packet.bytes).short(0x88).byte(4).byte(0).end();
    }

    #[test]
    fn a_spouse_chat_line_names_its_sender() {
        let packet = build_spouse_chat("Romeo", "hi").unwrap();
        Expect::new(&packet.bytes)
            .short(0x88)
            .byte(5)
            .string("Romeo")
            .byte(5)
            .string("hi")
            .end();
    }

    #[test]
    fn a_broadcast_is_its_kind_and_message() {
        let packet = build_broadcast(Broadcast::Pink, "Begone").unwrap();
        Expect::new(&packet.bytes)
            .short(0x44)
            .byte(5)
            .string("Begone")
            .end();
    }

    #[test]
    fn an_item_line_ends_with_its_item() {
        let packet = build_item_line("Tired", 0).unwrap();
        Expect::new(&packet.bytes)
            .short(0x44)
            .byte(6)
            .string("Tired")
            .int(0)
            .end();
    }

    #[test]
    fn a_gachapon_win_names_the_winner_the_channel_the_town_and_the_item() {
        let item = Item::stack(2_049_100, 1);
        let packet = build_gachapon_message("Lanec", 2, "Henesys", &item).unwrap();
        let mut body = Packet::new_empty();
        encode_item(&mut body, SlotPrefix::None, &item).unwrap();
        Expect::new(&packet.bytes)
            .short(0x44)
            .byte(11)
            .string("Lanec : got a(n)")
            .int(2)
            .string("Henesys")
            .bytes(&body.bytes)
            .end();
    }

    #[test]
    fn an_avatar_megaphone_refusal_is_its_letter() {
        let packet = build_avatar_megaphone_refusal(AvatarRefusal::Busy).unwrap();
        Expect::new(&packet.bytes).short(0x6E).byte(b'S').end();
        let packet = build_avatar_megaphone_refusal(AvatarRefusal::TooLow).unwrap();
        Expect::new(&packet.bytes).short(0x6E).byte(b'T').end();
    }

    #[test]
    fn an_avatar_banner_has_the_lines_then_the_channel_and_the_look() {
        let look = crate::codec::stats::sample_player().look();
        let lines = ["one", "two", "", "four"].map(String::from);
        let packet = build_avatar_megaphone(5_390_000, "Alice", &lines, 2, true, &look).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x6F).int(5_390_000).string("Alice");
        expect.string("one").string("two").string("").string("four");
        expect.int(2).byte(1).look(&look, true).end();
    }

    #[test]
    fn a_blocked_whisper_names_who_blocked_it() {
        let packet = build_whisper_blocked("Bobby").unwrap();
        Expect::new(&packet.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x22)
            .string("Bobby")
            .byte(0)
            .end();
    }

    #[test]
    fn whisper_receive_matches_reference() {
        let packet = build_whisper_receive("Alice", 2, false, "hello there").unwrap();

        // The reference writes the sender's channel as a short; the v83
        // client reads it as channel byte + admin flag byte.
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::Whisper as i16).byte(0x12);
        expect.string("Alice").short(2).string("hello there");
        expect.end();
    }

    #[test]
    fn whisper_from_admin_sets_the_second_byte() {
        let packet = build_whisper_receive("GM", 0, true, "hi").unwrap();

        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::Whisper as i16).byte(0x12);
        expect.string("GM").byte(0).byte(1).string("hi");
        expect.end();
    }

    #[test]
    fn whisper_result_matches_reference() {
        let found = build_whisper_result("Bob", true).unwrap();
        Expect::new(&found.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x0A)
            .string("Bob")
            .byte(1)
            .end();

        let missing = build_whisper_result("Nobody", false).unwrap();
        Expect::new(&missing.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x0A)
            .string("Nobody")
            .byte(0)
            .end();
    }

    #[test]
    fn find_results_match_reference() {
        let map = build_find_result("Bob", FindLocation::Map(100_000_000)).unwrap();
        Expect::new(&map.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x09)
            .string("Bob")
            .byte(1)
            .int(100_000_000)
            .int(0)
            .int(0)
            .end();

        let channel = build_find_result("Bob", FindLocation::Channel(2)).unwrap();
        Expect::new(&channel.bytes)
            .short(SendOpcode::Whisper as i16)
            .byte(0x09)
            .string("Bob")
            .byte(3)
            .int(2)
            .end();
    }
}
