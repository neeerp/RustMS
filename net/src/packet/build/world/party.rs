//! Party packets: `PARTY_OPERATION`, `UPDATE_PARTYMEMBER_HP`, and the party
//! mode of `MULTICHAT`.

use crate::codec::stats::write_padded_name;
use crate::packet::build::world::door::{DoorView, NO_DOOR};
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// Most members a party can hold; the status block always has this many.
pub const PARTY_CAPACITY: usize = 6;

const OP_INVITE: u8 = 0x04;
const OP_SILENT_UPDATE: u8 = 0x07;
const OP_CREATED: u8 = 0x08;
const OP_LEAVE: u8 = 0x0C;
const OP_JOIN: u8 = 0x0F;
const OP_CHANGE_LEADER: u8 = 0x1B;
const OP_DOOR: u8 = 0x25;

/// `MULTICHAT` mode for party chat (0 is buddies, 2 the guild).
pub const MULTICHAT_PARTY: u8 = 1;

/// A party status message the client shows by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyStatus {
    /// "Already have joined a party."
    AlreadyInParty = 16,
    /// "The party you're trying to join is already in full capacity."
    Full = 17,
    /// "Unable to find the requested character in this channel."
    NotFound = 19,
    /// "'name' is currently blocking any party invitations."
    Blocking = 21,
    /// "'name' is taking care of another invitation."
    BusyWithInvite = 22,
    /// "name have denied request to the party."
    Declined = 23,
}

impl PartyStatus {
    fn names_someone(self) -> bool {
        matches!(
            self,
            PartyStatus::Blocking | PartyStatus::BusyWithInvite | PartyStatus::Declined
        )
    }
}

/// One member as a particular receiver sees them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyMemberView {
    pub id: i32,
    pub name: String,
    pub job: i16,
    pub level: u8,
    /// Zero-based channel, or `None` while offline.
    pub channel: Option<u8>,
    /// Their map, or 0 when they aren't on the receiver's channel.
    pub map_id: i32,
    /// Their Mystic Door, if it is on the receiver's channel.
    pub door: Option<DoorView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyView {
    pub id: i32,
    pub leader: i32,
    pub members: Vec<PartyMemberView>,
}

/// How a member left, for `build_party_left`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Departure {
    Left,
    Expelled,
}

fn party_operation(op: u8) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::PartyOperation as i16)?;
    packet.write_byte(op)?;
    Ok(packet)
}

/// A member's door: its town, its field, and where in the field.
fn write_door(packet: &mut Packet, door: Option<&DoorView>) -> Result<(), NetworkError> {
    match door {
        Some(door) => {
            packet.write_int(door.town)?;
            packet.write_int(door.field)?;
            packet.write_int(i32::from(door.x))?;
            packet.write_int(i32::from(door.y))?;
        }
        None => {
            packet.write_int(NO_DOOR)?;
            packet.write_int(NO_DOOR)?;
            packet.write_int(0)?;
            packet.write_int(0)?;
        }
    }
    Ok(())
}

/// The member block (`addPartyStatus`): six slots of each column, empty
/// slots zeroed.
fn write_party_status(packet: &mut Packet, party: &PartyView) -> Result<(), NetworkError> {
    let slot = |i: usize| party.members.get(i);
    for i in 0..PARTY_CAPACITY {
        packet.write_int(slot(i).map_or(0, |m| m.id))?;
    }
    for i in 0..PARTY_CAPACITY {
        write_padded_name(packet, slot(i).map_or("", |m| m.name.as_str()))?;
    }
    for i in 0..PARTY_CAPACITY {
        packet.write_int(slot(i).map_or(0, |m| i32::from(m.job)))?;
    }
    for i in 0..PARTY_CAPACITY {
        packet.write_int(slot(i).map_or(0, |m| i32::from(m.level)))?;
    }
    for i in 0..PARTY_CAPACITY {
        let channel = match slot(i) {
            Some(PartyMemberView {
                channel: Some(channel),
                ..
            }) => i32::from(*channel),
            _ => -2,
        };
        packet.write_int(channel)?;
    }
    packet.write_int(party.leader)?;
    for i in 0..PARTY_CAPACITY {
        packet.write_int(slot(i).map_or(0, |m| m.map_id))?;
    }
    for i in 0..PARTY_CAPACITY {
        write_door(packet, slot(i).and_then(|m| m.door.as_ref()))?;
    }
    Ok(())
}

/// The requester's new party, and their door, now its first.
pub fn build_party_created(party_id: i32, door: Option<&DoorView>) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_CREATED)?;
    packet.write_int(party_id)?;
    write_door(&mut packet, door)?;
    Ok(packet)
}

/// The member in `slot` opened a Mystic Door, or theirs closed (v83
/// FUN_00a3e31c 0x25): the client draws it at the town's door portal of
/// that slot.
pub fn build_party_door(slot: u8, door: Option<&DoorView>) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_DOOR)?;
    packet.write_byte(slot)?;
    let (town, field, x, y) = door.map_or((NO_DOOR, NO_DOOR, 0, 0), |door| {
        (door.town, door.field, door.x, door.y)
    });
    packet.write_int(town)?;
    packet.write_int(field)?;
    packet.write_short(x)?;
    packet.write_short(y)?;
    Ok(packet)
}

/// `inviter` asks the receiver to join `party_id`.
pub fn build_party_invite(party_id: i32, inviter: &str) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_INVITE)?;
    packet.write_int(party_id)?;
    packet.write_str_with_length(inviter)?;
    packet.write_byte(0)?;
    Ok(packet)
}

/// A status message; `name` fills in codes that mention someone.
pub fn build_party_status(status: PartyStatus, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(status as u8)?;
    if status.names_someone() {
        packet.write_str_with_length(name)?;
    }
    Ok(packet)
}

/// Members' channels, maps, levels, or jobs changed.
pub fn build_party_update(party: &PartyView) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_SILENT_UPDATE)?;
    packet.write_int(party.id)?;
    write_party_status(&mut packet, party)?;
    Ok(packet)
}

/// `name` joined; `party` already lists them.
pub fn build_party_joined(party: &PartyView, name: &str) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_JOIN)?;
    packet.write_int(party.id)?;
    packet.write_str_with_length(name)?;
    write_party_status(&mut packet, party)?;
    Ok(packet)
}

/// `member_id` left or was expelled; `party` no longer lists them.
pub fn build_party_left(
    party: &PartyView,
    member_id: i32,
    name: &str,
    how: Departure,
) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_LEAVE)?;
    packet.write_int(party.id)?;
    packet.write_int(member_id)?;
    packet.write_byte(1)?;
    packet.write_byte((how == Departure::Expelled) as u8)?;
    packet.write_str_with_length(name)?;
    write_party_status(&mut packet, party)?;
    Ok(packet)
}

/// The leader `leader_id` left, which ends the party.
pub fn build_party_disbanded(party_id: i32, leader_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_LEAVE)?;
    packet.write_int(party_id)?;
    packet.write_int(leader_id)?;
    packet.write_byte(0)?;
    packet.write_int(party_id)?;
    Ok(packet)
}

pub fn build_party_leader_changed(leader_id: i32) -> Result<Packet, NetworkError> {
    let mut packet = party_operation(OP_CHANGE_LEADER)?;
    packet.write_int(leader_id)?;
    packet.write_byte(0)?;
    Ok(packet)
}

/// A party member's HP, for the receiver's party HP display.
pub fn build_party_member_hp(id: i32, hp: i32, max_hp: i32) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::UpdatePartyMemberHp as i16)?;
    packet.write_int(id)?;
    packet.write_int(hp)?;
    packet.write_int(max_hp)?;
    Ok(packet)
}

pub fn build_party_chat(name: &str, text: &str) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::MultiChat as i16)?;
    packet.write_byte(MULTICHAT_PARTY)?;
    packet.write_str_with_length(name)?;
    packet.write_str_with_length(text)?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    const DOOR: DoorView = DoorView {
        town: 101_000_000,
        field: 101_010_000,
        x: -120,
        y: 275,
    };

    fn view() -> PartyView {
        PartyView {
            id: 7,
            leader: 2,
            members: vec![
                PartyMemberView {
                    id: 2,
                    name: "Alice".into(),
                    job: 0,
                    level: 8,
                    channel: Some(1),
                    map_id: 10000,
                    door: Some(DOOR),
                },
                PartyMemberView {
                    id: 3,
                    name: "Bobby".into(),
                    job: 100,
                    level: 12,
                    channel: None,
                    map_id: 0,
                    door: None,
                },
            ],
        }
    }

    fn name(expect: &mut Expect, name: &str) {
        let mut padded = name.as_bytes().to_vec();
        padded.resize(13, 0);
        expect.bytes(&padded);
    }

    fn party_status(expect: &mut Expect) {
        expect.int(2).int(3);
        for _ in 0..4 {
            expect.int(0);
        }
        name(expect, "Alice");
        name(expect, "Bobby");
        for _ in 0..4 {
            name(expect, "");
        }
        expect.int(0).int(100).int(0).int(0).int(0).int(0);
        expect.int(8).int(12).int(0).int(0).int(0).int(0);
        expect.int(1).int(-2).int(-2).int(-2).int(-2).int(-2);
        expect.int(2);
        expect.int(10000).int(0).int(0).int(0).int(0).int(0);
        expect.int(101_000_000).int(101_010_000).int(-120).int(275);
        for _ in 0..5 {
            expect.int(NO_DOOR).int(NO_DOOR).int(0).int(0);
        }
    }

    #[test]
    fn join_lists_every_slot_with_offline_members_on_channel_minus_two() {
        let packet = build_party_joined(&view(), "Bobby").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(0x0F);
        expect.int(7).string("Bobby");
        party_status(&mut expect);
        expect.end();
    }

    #[test]
    fn expel_marks_the_departure() {
        let packet = build_party_left(&view(), 4, "Carol", Departure::Expelled).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(0x0C);
        expect.int(7).int(4).byte(1).byte(1).string("Carol");
        party_status(&mut expect);
        expect.end();
    }

    #[test]
    fn disband_and_small_operations() {
        let packet = build_party_disbanded(7, 2).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(0x0C);
        expect.int(7).int(2).byte(0).int(7);
        expect.end();

        let packet = build_party_created(7, None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PartyOperation as i16)
            .byte(8)
            .int(7);
        expect.int(NO_DOOR).int(NO_DOOR).int(0).int(0);
        expect.end();

        let packet = build_party_created(7, Some(&DOOR)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PartyOperation as i16)
            .byte(8)
            .int(7);
        expect.int(101_000_000).int(101_010_000).int(-120).int(275);
        expect.end();

        let packet = build_party_door(3, Some(&DOOR)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PartyOperation as i16)
            .byte(0x25)
            .byte(3);
        expect
            .int(101_000_000)
            .int(101_010_000)
            .short(-120)
            .short(275);
        expect.end();

        let packet = build_party_door(0, None).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PartyOperation as i16)
            .byte(0x25)
            .byte(0);
        expect.int(NO_DOOR).int(NO_DOOR).short(0).short(0);
        expect.end();

        let packet = build_party_invite(7, "Alice").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(4);
        expect.int(7).string("Alice").byte(0);
        expect.end();

        let packet = build_party_status(PartyStatus::Full, "ignored").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(17);
        expect.end();

        let packet = build_party_status(PartyStatus::Declined, "Bobby").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(23);
        expect.string("Bobby");
        expect.end();

        let packet = build_party_leader_changed(3).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::PartyOperation as i16).byte(0x1B);
        expect.int(3).byte(0);
        expect.end();
    }

    #[test]
    fn member_hp_and_party_chat() {
        let packet = build_party_member_hp(3, 40, 50).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::UpdatePartyMemberHp as i16);
        expect.int(3).int(40).int(50);
        expect.end();

        let packet = build_party_chat("Alice", "hi").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::MultiChat as i16).byte(1);
        expect.string("Alice").string("hi");
        expect.end();
    }
}
