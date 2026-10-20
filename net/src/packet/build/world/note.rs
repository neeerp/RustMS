//! MEMO_RESULT (0x29, v83 FUN_00a2508b, report 204m): notes waiting for the
//! player, and what became of one they sent.

use crate::codec::time::filetime;
use crate::{error::NetworkError, packet::op::SendOpcode};
use db::note::Note;
use packet::{io::write::PktWrite, Packet};

const LIST: u8 = 3;
const SENT: u8 = 4;
const NOT_SENT: u8 = 5;
const ARRIVED: u8 = 7;

/// The most notes one list holds: v83 reads the count as a byte.
pub const MAX_LISTED: usize = 255;
/// The longest sender name and message v83 copies into a note's fixed
/// buffers (13 and 201 bytes).
const SENDER_LEN: usize = 12;
pub const MESSAGE_LEN: usize = 200;

/// Why a note wasn't sent (mode 5's code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotSent {
    /// "The other character is online now. Please use the whisper function,"
    Online = 0,
    /// "Please check the name of the receiving character."
    NotFound = 1,
    /// "The receiver's inbox is full. Please try again."
    InboxFull = 2,
}

/// Mode 3: every note waiting for the player. A non-empty list tells them
/// they have new notes.
pub fn build_note_list(notes: &[Note]) -> Result<Packet, NetworkError> {
    let notes = &notes[..notes.len().min(MAX_LISTED)];
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MemoResult as i16)?;
    packet.write_byte(LIST)?;
    packet.write_byte(notes.len() as u8)?;
    for note in notes {
        packet.write_int(note.id)?;
        packet.write_str_with_length(truncated(&note.sender, SENDER_LEN))?;
        packet.write_str_with_length(truncated(&note.message, MESSAGE_LEN))?;
        packet.write_long(filetime(note.sent_at))?;
        packet.write_byte(note.flag as u8)?;
    }
    Ok(packet)
}

/// Mode 4: "The note has successfully been sent".
pub fn build_note_sent() -> Result<Packet, NetworkError> {
    build_mode(SENT)
}

/// Mode 5: the note wasn't sent, and why. It also lets the client use cash
/// items again.
pub fn build_note_not_sent(why: NotSent) -> Result<Packet, NetworkError> {
    let mut packet = build_mode(NOT_SENT)?;
    packet.write_byte(why as u8)?;
    Ok(packet)
}

/// Mode 7: a note arrived; the client asks for the list.
pub fn build_note_arrived() -> Result<Packet, NetworkError> {
    build_mode(ARRIVED)
}

fn build_mode(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MemoResult as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// At most `len` bytes of `text`, cut at a character boundary.
fn truncated(text: &str, len: usize) -> &str {
    let mut end = text.len().min(len);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn a_list_holds_each_note_with_its_time_and_flag() {
        let note = Note {
            id: 9,
            sender: "Alice".to_string(),
            message: "hi".to_string(),
            sent_at: UNIX_EPOCH,
            flag: 1,
        };
        let packet = build_note_list(&[note]).unwrap();
        let mut expected = vec![0x29, 0, 3, 1, 9, 0, 0, 0, 5, 0];
        expected.extend_from_slice(b"Alice");
        expected.extend_from_slice(&[2, 0]);
        expected.extend_from_slice(b"hi");
        expected.extend_from_slice(&filetime(UNIX_EPOCH).to_le_bytes());
        expected.push(1);
        assert_eq!(packet.bytes, expected);
    }

    #[test]
    fn a_long_message_is_cut_to_what_the_client_holds() {
        assert_eq!(truncated(&"a".repeat(300), MESSAGE_LEN).len(), 200);
        assert_eq!(truncated("ab\u{e9}", 3), "ab");
    }
}
