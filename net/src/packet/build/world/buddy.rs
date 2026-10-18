//! Buddy list packets: `BUDDYLIST` and the buddy mode of `MULTICHAT`.

use crate::codec::stats::write_padded_name;
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

const OP_LIST: u8 = 0x07;
const OP_REQUEST: u8 = 0x09;
const OP_CHANNEL: u8 = 0x14;
const OP_CAPACITY: u8 = 0x15;

/// `MULTICHAT` mode for buddy chat.
pub const MULTICHAT_BUDDY: u8 = 0;

/// Bytes of the group name field, terminator included.
const GROUP_FIELD_LEN: usize = 17;

/// Why the client's add request failed; the client shows each by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuddyError {
    /// "Your buddy list is full."
    OwnListFull = 0x0B,
    /// "The user's buddy list is full."
    TheirListFull = 0x0C,
    /// "That character is already registered as your buddy."
    AlreadyAdded = 0x0D,
    /// "That character is not registered."
    NotFound = 0x0F,
}

/// One buddy as the list owner sees them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuddyView {
    pub id: i32,
    pub name: String,
    pub group: String,
    /// Zero-based channel, or `None` while offline or not mutual.
    pub channel: Option<u8>,
    /// They asked to be added and haven't been answered. The client keeps
    /// the entry, hidden, and asks about it again after each map change.
    pub requesting: bool,
}

fn buddy_list(op: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::BuddyList as i16)?;
    packet.write_byte(op)?;
    Ok(packet)
}

fn channel_int(channel: Option<u8>) -> i32 {
    channel.map_or(-1, i32::from)
}

/// The 39-byte `GW_Friend` record.
fn write_entry(packet: &mut Packet, buddy: &BuddyView) -> Result<(), NetworkError> {
    packet.write_int(buddy.id)?;
    write_padded_name(packet, &buddy.name)?;
    packet.write_byte(u8::from(buddy.requesting))?;
    packet.write_int(channel_int(buddy.channel))?;
    let group = buddy.group.as_bytes();
    let group = &group[..group.len().min(GROUP_FIELD_LEN - 1)];
    packet.write_bytes(group)?;
    packet.write_bytes(&[0u8; GROUP_FIELD_LEN][..GROUP_FIELD_LEN - group.len()])?;
    Ok(())
}

/// The whole list, replacing what the client shows.
pub fn build_buddy_list(buddies: &[BuddyView]) -> Result<Packet, NetworkError> {
    let mut packet = buddy_list(OP_LIST)?;
    packet.write_byte(buddies.len() as u8)?;
    for buddy in buddies {
        write_entry(&mut packet, buddy)?;
    }
    // Whether each buddy is in the Cash Shop.
    for _ in buddies {
        packet.write_int(0)?;
    }
    Ok(packet)
}

/// `from` asks to be added to the receiver's list.
pub fn build_buddy_request(
    from_id: i32,
    from_name: &str,
    channel: Option<u8>,
) -> Result<Packet, NetworkError> {
    let mut packet = buddy_list(OP_REQUEST)?;
    packet.write_int(from_id)?;
    packet.write_str_with_length(from_name)?;
    let entry = BuddyView {
        id: from_id,
        name: from_name.into(),
        group: "Default Group".into(),
        channel,
        requesting: true,
    };
    write_entry(&mut packet, &entry)?;
    packet.write_byte(0)?; // in the Cash Shop
    Ok(packet)
}

/// A buddy logged in, changed channel, or logged out (`None`).
pub fn build_buddy_channel(id: i32, channel: Option<u8>) -> Result<Packet, NetworkError> {
    let mut packet = buddy_list(OP_CHANNEL)?;
    packet.write_int(id)?;
    packet.write_byte(0)?; // in the Cash Shop
    packet.write_int(channel_int(channel))?;
    Ok(packet)
}

pub fn build_buddy_capacity(capacity: u8) -> Result<Packet, NetworkError> {
    let mut packet = buddy_list(OP_CAPACITY)?;
    packet.write_byte(capacity)?;
    Ok(packet)
}

pub fn build_buddy_error(error: BuddyError) -> Result<Packet, NetworkError> {
    buddy_list(error as u8)
}

pub fn build_buddy_chat(name: &str, text: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MultiChat as i16)?;
    packet.write_byte(MULTICHAT_BUDDY)?;
    packet.write_str_with_length(name)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    fn padded(text: &str, len: usize) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.resize(len, 0);
        bytes
    }

    #[test]
    fn list_writes_each_entry_then_cash_shop_flags() {
        let buddies = [
            BuddyView {
                id: 7,
                name: "Bobby".into(),
                group: "Default Group".into(),
                channel: Some(2),
                requesting: false,
            },
            BuddyView {
                id: 9,
                name: "Carol".into(),
                group: "A group name that is far too long".into(),
                channel: None,
                requesting: true,
            },
        ];
        let packet = build_buddy_list(&buddies).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::BuddyList as i16).byte(7).byte(2);
        expect
            .int(7)
            .bytes(&padded("Bobby", 13))
            .byte(0)
            .int(2)
            .bytes(&padded("Default Group", 17));
        expect
            .int(9)
            .bytes(&padded("Carol", 13))
            .byte(1)
            .int(-1)
            .bytes(&padded("A group name tha", 17));
        expect.int(0).int(0).end();
    }

    #[test]
    fn request_carries_the_requester_entry() {
        let packet = build_buddy_request(3, "Alice", Some(1)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::BuddyList as i16)
            .byte(9)
            .int(3)
            .string("Alice")
            .int(3)
            .bytes(&padded("Alice", 13))
            .byte(1)
            .int(1)
            .bytes(&padded("Default Group", 17))
            .byte(0)
            .end();
    }

    #[test]
    fn channel_capacity_and_errors() {
        let packet = build_buddy_channel(5, None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::BuddyList as i16)
            .byte(0x14)
            .int(5)
            .byte(0)
            .int(-1)
            .end();

        let packet = build_buddy_capacity(30).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::BuddyList as i16)
            .byte(0x15)
            .byte(30)
            .end();

        let packet = build_buddy_error(BuddyError::NotFound).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::BuddyList as i16).byte(0x0F).end();
    }
}
