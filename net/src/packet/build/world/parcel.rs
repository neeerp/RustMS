//! PARCEL (322, v83 FUN_006f56ea, report b028): the Duey window, the
//! parcels in it, what became of a send or claim, and the notifier of a
//! parcel that arrived.

use crate::codec::item::{encode_item, SlotPrefix};
use crate::codec::time::filetime;
use crate::{duey::INBOX_LIMIT, error::NetworkError, packet::op::SendOpcode};
use db::parcel::Parcel;
use packet::{io::write::PktWrite, Packet};
use std::time::Duration;

const OPEN: u8 = 0x08;
const REMOVED: u8 = 0x17;
const ADDED: u8 = 0x18;
const ARRIVED: u8 = 0x19;
const QUICK_SEND: u8 = 0x1a;
const WAITING: u8 = 0x1b;

/// Why a parcel left the window (mode 0x17's kind).
pub const DELETED: u8 = 3;
pub const CLAIMED: u8 = 4;

/// The fixed buffers a parcel's sender and message are copied into; each
/// keeps a NUL at its end.
const SENDER_LEN: usize = 13;
const MESSAGE_LEN: usize = 200;

/// The most expired parcels one window lists: v83 reads the count as a
/// byte.
const MAX_EXPIRED: usize = 255;

fn parcel_packet(mode: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::Parcel as i16)?;
    packet.write_byte(mode)?;
    Ok(packet)
}

/// `text` in a NUL-padded buffer of `len` bytes, cut short to leave room
/// for the NUL.
fn write_fixed(packet: &mut Packet, text: &str, len: usize) -> Result<(), NetworkError> {
    let mut end = text.len().min(len - 1);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    packet.write_bytes(&text.as_bytes()[..end])?;
    packet.write_bytes(&vec![0; len - end])?;
    Ok(())
}

/// A day short of a parcel's expiry, which the window shows: it counts the
/// days left, and calls a parcel 30 or more days off still on its way. The
/// day spare keeps a new parcel claimable on a client whose clock is behind.
const MARGIN: Duration = Duration::from_secs(24 * 60 * 60);

/// One parcel as the window lists it.
fn write_parcel(packet: &mut Packet, parcel: &Parcel) -> Result<(), NetworkError> {
    packet.write_int(i32::try_from(parcel.id).unwrap_or(-1))?;
    write_fixed(packet, &parcel.sender, SENDER_LEN)?;
    packet.write_int(parcel.mesos)?;
    packet.write_long(filetime(parcel.expires_at() - MARGIN))?;
    let message = parcel.message.as_deref().unwrap_or("");
    packet.write_int(i32::from(!message.is_empty()))?;
    write_fixed(packet, message, MESSAGE_LEN)?;
    packet.write_byte(u8::from(parcel.quick))?;
    packet.write_byte(u8::from(parcel.item.is_some()))?;
    if let Some(item) = &parcel.item {
        encode_item(packet, SlotPrefix::None, item)?;
    }
    Ok(())
}

/// Mode 0x08: open the Duey window with the parcels waiting, and tell the
/// player which ones expired. `receive_only` leaves out the send tab.
pub fn build_parcel_window(
    receive_only: bool,
    waiting: &[Parcel],
    expired: &[Parcel],
) -> Result<Packet, NetworkError> {
    let waiting = &waiting[..waiting.len().min(INBOX_LIMIT)];
    let expired = &expired[..expired.len().min(MAX_EXPIRED)];
    let mut packet = parcel_packet(OPEN)?;
    packet.write_byte(u8::from(receive_only))?;
    for list in [waiting, expired] {
        packet.write_byte(list.len() as u8)?;
        for parcel in list {
            write_parcel(&mut packet, parcel)?;
        }
    }
    Ok(packet)
}

/// Mode 0x17: parcel `id` was claimed or deleted (`kind`).
pub fn build_parcel_removed(id: i32, kind: u8) -> Result<Packet, NetworkError> {
    let mut packet = parcel_packet(REMOVED)?;
    packet.write_int(id)?;
    packet.write_byte(kind)?;
    Ok(packet)
}

/// Mode 0x18: `parcel` arrived while the window is open; it joins the
/// list. A client without the window ignores it.
pub fn build_parcel_added(parcel: &Parcel) -> Result<Packet, NetworkError> {
    let mut packet = parcel_packet(ADDED)?;
    write_parcel(&mut packet, parcel)?;
    Ok(packet)
}

/// Mode 0x19: a parcel from `sender` just arrived.
pub fn build_parcel_arrived(sender: &str, quick: bool) -> Result<Packet, NetworkError> {
    let mut packet = parcel_packet(ARRIVED)?;
    packet.write_str_with_length(sender)?;
    packet.write_byte(u8::from(quick))?;
    Ok(packet)
}

/// Mode 0x1a: open the window for a quick send, from the coupon.
pub fn build_quick_send_window() -> Result<Packet, NetworkError> {
    parcel_packet(QUICK_SEND)
}

/// Mode 0x1b: on entering the game, parcels are waiting; `quick` if one
/// is a quick delivery.
pub fn build_parcels_waiting(quick: bool) -> Result<Packet, NetworkError> {
    let mut packet = parcel_packet(WAITING)?;
    packet.write_byte(u8::from(quick))?;
    Ok(packet)
}

/// A result mode (`crate::duey::result`), which also lets the window take
/// requests again.
pub fn build_parcel_result(mode: u8) -> Result<Packet, NetworkError> {
    parcel_packet(mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Item;
    use std::time::{Duration, UNIX_EPOCH};

    fn parcel(item: Option<Item>, message: Option<&str>) -> Parcel {
        Parcel {
            id: 9,
            sender: "Duey".into(),
            mesos: 1000,
            message: message.map(Into::into),
            quick: message.is_some(),
            sent_at: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            item,
        }
    }

    #[test]
    fn a_parcel_is_its_fixed_record_then_its_item() {
        let packet = build_parcel_window(false, &[parcel(None, Some("hi"))], &[]).unwrap();
        let body = &packet.bytes[2..];
        assert_eq!(&body[..3], &[OPEN, 0, 1]);
        let record = &body[3..];
        assert_eq!(&record[..4], &9i32.to_le_bytes());
        assert_eq!(&record[4..17], b"Duey\0\0\0\0\0\0\0\0\0");
        assert_eq!(&record[17..21], &1000i32.to_le_bytes());
        let expires =
            UNIX_EPOCH + Duration::from_secs(1_700_000_000) + db::parcel::LIFETIME - MARGIN;
        assert_eq!(&record[21..29], &filetime(expires).to_le_bytes());
        assert_eq!(&record[29..33], &1i32.to_le_bytes());
        assert_eq!(&record[33..35], b"hi");
        assert!(record[35..233].iter().all(|&b| b == 0));
        assert_eq!(&record[233..235], &[1, 0], "quick, no item");
        assert_eq!(&record[235..], &[0], "no expired parcels");
        assert_eq!(record.len(), 0xeb + 1);
    }

    #[test]
    fn an_item_follows_without_a_slot() {
        let with = build_parcel_window(
            true,
            &[parcel(Some(Item::stack(2000000, 3)), None)],
            &[parcel(None, None)],
        )
        .unwrap();
        let body = &with.bytes[2..];
        assert_eq!(body[1], 1, "receive only");
        let record = &body[3..];
        assert_eq!(&record[233..235], &[0, 1]);
        assert_eq!(record[235], 2, "a stack");
        assert_eq!(&record[236..240], &2000000i32.to_le_bytes());
    }

    #[test]
    fn long_names_and_messages_keep_their_nul() {
        let mut long = parcel(None, Some(&"x".repeat(300)));
        long.sender = "abcdefghijklmnop".into();
        let packet = build_parcel_window(false, &[long], &[]).unwrap();
        let record = &packet.bytes[5..];
        assert_eq!(&record[4..17], b"abcdefghijkl\0");
        assert_eq!(record[33 + 199], 0);
        assert_eq!(record[33 + 198], b'x');
    }

    #[test]
    fn an_added_parcel_is_one_record() {
        let packet = build_parcel_added(&parcel(None, None)).unwrap();
        let body = &packet.bytes[2..];
        assert_eq!(body[0], ADDED);
        assert_eq!(&body[1..5], &9i32.to_le_bytes());
        assert_eq!(body.len(), 1 + 0xeb);
    }

    #[test]
    fn notices_and_results() {
        assert_eq!(
            &build_parcel_removed(5, CLAIMED).unwrap().bytes[2..],
            &[REMOVED, 5, 0, 0, 0, CLAIMED]
        );
        assert_eq!(
            &build_parcel_arrived("Ann", true).unwrap().bytes[2..],
            &[ARRIVED, 3, 0, b'A', b'n', b'n', 1]
        );
        assert_eq!(
            &build_quick_send_window().unwrap().bytes[2..],
            &[QUICK_SEND]
        );
        assert_eq!(
            &build_parcels_waiting(false).unwrap().bytes[2..],
            &[WAITING, 0]
        );
        assert_eq!(&build_parcel_result(0x12).unwrap().bytes[2..], &[0x12]);
        assert_eq!(
            build_parcel_result(0x12).unwrap().opcode(),
            SendOpcode::Parcel as i16
        );
    }
}
