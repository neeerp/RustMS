//! A GM's `!mute` (row 205 S6): the account's chat block. v83 learns of it
//! only from the login result (report b180), after which the client itself
//! refuses to send chat; until then, and against a client that sends it
//! anyway, the world drops what the player says here.

use crate::packet::build::world::messaging::WHISPER_REQUEST_MODE;
use crate::packet::op::RecvOpcode;
use packet::Packet;

/// What the client says when it refuses (StringPool 0x37d).
pub const BLOCKED: &str = "You are currently blocked from chatting.";

/// The reasons a chat block may give, by the `type` of their
/// Etc/ChatBlockReason.img node, whose number is the place here plus one.
pub const REASONS: [&str; 6] = [
    "foul",        // "Foul language/Harrassment"
    "advertising", // "Advertising websites"
    "hack",        // "Fake GM"
    "account",     // "Trading or selling account"
    "trading",     // "Do not report fame scams and trade scams."
    "penalty",     // "The reporter's conversation is also subject to penalties."
];

/// The chat modes of a whisper, the Maple Messenger and a mini-room.
const CHAT_MODE: u8 = 0x06;

/// Whether `packet` is the player speaking: chat to the map, a group, a
/// spouse, a whisper, the Messenger or a mini-room (all the client refuses),
/// or an item that carries the player's words (megaphones, Maple TV, kites,
/// notes, weather, the chalkboard, avatar megaphones), which it doesn't.
pub fn is_chat(packet: &Packet) -> bool {
    let bytes = &packet.bytes;
    let mode = bytes.get(2).copied();
    match packet.opcode() {
        op if op == RecvOpcode::AllChat as i16
            || op == RecvOpcode::MultiChat as i16
            || op == RecvOpcode::SpouseChat as i16 =>
        {
            true
        }
        op if op == RecvOpcode::Whisper as i16 => mode == Some(WHISPER_REQUEST_MODE),
        op if op == RecvOpcode::Messenger as i16 || op == RecvOpcode::PlayerInteraction as i16 => {
            mode == Some(CHAT_MODE)
        }
        op if op == RecvOpcode::UseCashItem as i16 => bytes
            .get(4..8)
            .map(|id| i32::from_le_bytes(id.try_into().unwrap()) / 10_000)
            .is_some_and(|kind| matches!(kind, 507 | 508 | 509 | 512 | 537 | 539)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use packet::io::write::PktWrite;

    fn packet(opcode: RecvOpcode, body: &[u8]) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(opcode as i16).unwrap();
        p.write_bytes(body).unwrap();
        p
    }

    fn cash_item(item_id: i32) -> Packet {
        let mut body = 1i16.to_le_bytes().to_vec();
        body.extend(item_id.to_le_bytes());
        packet(RecvOpcode::UseCashItem, &body)
    }

    #[test]
    fn only_what_the_player_says_is_chat() {
        assert!(is_chat(&packet(RecvOpcode::AllChat, &[1, 0, b'a'])));
        assert!(is_chat(&packet(RecvOpcode::MultiChat, &[1])));
        assert!(is_chat(&packet(RecvOpcode::SpouseChat, &[])));
        assert!(is_chat(&packet(RecvOpcode::Whisper, &[0x06])));
        // `/find` is not.
        assert!(!is_chat(&packet(RecvOpcode::Whisper, &[0x05])));
        assert!(is_chat(&packet(RecvOpcode::Messenger, &[0x06])));
        assert!(!is_chat(&packet(RecvOpcode::Messenger, &[0x02])));
        assert!(is_chat(&packet(RecvOpcode::PlayerInteraction, &[0x06])));
        assert!(!is_chat(&packet(RecvOpcode::PlayerInteraction, &[0x0a])));
        assert!(is_chat(&cash_item(5_071_000)));
        assert!(is_chat(&cash_item(5_075_000)));
        assert!(is_chat(&cash_item(5_090_000)));
        assert!(is_chat(&cash_item(5_370_000)));
        assert!(is_chat(&cash_item(5_390_000)));
        assert!(!is_chat(&cash_item(5_040_000)));
        assert!(!is_chat(&packet(RecvOpcode::UseCashItem, &[1, 0])));
        assert!(!is_chat(&packet(RecvOpcode::ChangeMap, &[0])));
    }
}
