//! Notes (memos, report 204m): NOTE_ACTION (0x83) reads, thanks for and asks
//! for them, and the note item (509xxxx) sends one through USE_CASH_ITEM.

use crate::handler::{
    HandlerResult, NoteRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::note::MESSAGE_LEN;
use crate::packet::build::world::stat::build_stat_update;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

const THANK: u8 = 0;
const DELETE: u8 = 1;
const LIST: u8 = 2;
/// A wedding invitation, which carries one more int when deleted.
const INVITATION: u8 = 3;

pub struct NoteActionHandler;

impl WorldHandler for NoteActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let request = match r.read_byte()? {
            // The Cash Shop's answer to a gift (FUN_0047959e): its sender,
            // the text, then 1, the gift's index and its cash serial.
            THANK => {
                let to = r.read_str_with_length()?;
                let text = r.read_str_with_length()?;
                r.read_byte()?;
                r.read_int()?;
                let gift = r.read_long()?;
                if text.is_empty() || text.len() > MESSAGE_LEN {
                    return Ok(HandlerResult::empty());
                }
                NoteRequest::ThankForGift { to, text, gift }
            }
            // Closing the inbox (FUN_0064aa57): how many notes, how many of
            // them are invitations, free Etc slots, then each note. An
            // invitation without a slot for it is left out, so read until
            // the packet ends.
            DELETE => {
                let count = r.read_byte()?;
                r.read_byte()?;
                r.read_byte()?;
                let mut ids = Vec::new();
                for _ in 0..count {
                    let Ok(id) = r.read_int() else {
                        break;
                    };
                    if r.read_byte()? == INVITATION {
                        r.read_int()?;
                    }
                    ids.push(id);
                }
                NoteRequest::Delete { ids }
            }
            LIST => NoteRequest::List,
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Note(request)))
    }
}

/// Send a note with the note item `item_id` in Cash `slot`; `r` is past the
/// item id. The client has made sure of the name and text (FUN_007dc659),
/// so anything else only unlocks it.
pub fn note(
    ctx: &mut WorldContext,
    r: &mut Cursor<&Vec<u8>>,
    slot: i16,
    item_id: i32,
) -> WorldResult {
    let to = r.read_str_with_length()?;
    let text = r.read_str_with_length()?;
    let to = to.trim();
    let valid = (4..=12).contains(&to.len())
        && !to.eq_ignore_ascii_case(ctx.player.name())
        && !text.trim().is_empty()
        && text.len() <= MESSAGE_LEN;
    if !valid {
        return Ok(HandlerResult::reply(build_stat_update(true, &[])?));
    }
    Ok(
        HandlerResult::empty().with(WorldAction::Note(NoteRequest::Send {
            slot,
            item_id,
            to: to.to_string(),
            text,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn handle(packet: &mut Packet) -> NoteRequest {
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let actions = NoteActionHandler.handle(packet, &mut ctx).unwrap().actions;
        let [WorldAction::Note(request)] = &actions[..] else {
            panic!("expected a note request: {actions:?}");
        };
        request.clone()
    }

    fn note_action(mode: u8) -> Packet {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::NoteAction as i16).unwrap();
        packet.write_byte(mode).unwrap();
        packet
    }

    #[test]
    fn deleting_reads_invitations_and_stops_at_the_end() {
        let mut packet = note_action(DELETE);
        // Three notes, one an invitation, but one invitation left out.
        packet.write_bytes(&[3, 2, 0]).unwrap();
        packet.write_int(7).unwrap();
        packet.write_byte(0).unwrap();
        packet.write_int(8).unwrap();
        packet.write_byte(INVITATION).unwrap();
        packet.write_int(4_031_377).unwrap();
        assert_eq!(handle(&mut packet), NoteRequest::Delete { ids: vec![7, 8] });
    }

    #[test]
    fn a_thank_you_names_the_gift() {
        let mut packet = note_action(THANK);
        packet.write_str_with_length("Alice").unwrap();
        packet.write_str_with_length("thanks").unwrap();
        packet.write_byte(1).unwrap();
        packet.write_int(0).unwrap();
        packet.write_long(42).unwrap();
        assert_eq!(
            handle(&mut packet),
            NoteRequest::ThankForGift {
                to: "Alice".to_string(),
                text: "thanks".to_string(),
                gift: 42,
            }
        );
    }
}
