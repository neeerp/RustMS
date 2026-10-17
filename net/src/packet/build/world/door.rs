//! Mystic Door packets: the door in its field (SPAWN_DOOR, REMOVE_DOOR) and
//! its owner's way back from town (SPAWN_PORTAL). The party's doors ride in
//! the party packets (`party::build_party_door`).

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The map id written where there is no door.
pub const NO_DOOR: i32 = 999_999_999;

/// A Mystic Door as its owner and party see it: the town it leads to, the
/// field it stands in, and where in the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorView {
    pub town: i32,
    pub field: i32,
    pub x: i16,
    pub y: i16,
}

/// SPAWN_PORTAL (0x43, v83 FUN_00a226a6): the owner's own door, kept for
/// when they are alone, or none. Without a party the client draws it at the
/// town's first door portal and lets the owner in.
pub fn build_spawn_portal(door: Option<&DoorView>) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnPortal as i16)?;
    match door {
        Some(door) => {
            p.write_int(door.town)?;
            p.write_int(door.field)?;
            p.write_short(door.x)?;
            p.write_short(door.y)?;
        }
        None => {
            p.write_int(NO_DOOR)?;
            p.write_int(NO_DOOR)?;
        }
    }
    Ok(p)
}

/// SPAWN_DOOR (0x113, v83 FUN_007bd6c6): `owner`'s door in the field, on
/// the ground below `at`. A door just cast opens (`cDoor`); one already
/// there stands (`mDoor` in its `Frame`).
pub fn build_spawn_door(
    owner: i32,
    at: (i16, i16),
    just_cast: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnDoor as i16)?;
    p.write_byte(u8::from(!just_cast))?;
    p.write_int(owner)?;
    p.write_short(at.0)?;
    p.write_short(at.1)?;
    Ok(p)
}

/// REMOVE_DOOR (0x114, v83 FUN_007be064): `owner`'s door fades out over a
/// second.
pub fn build_remove_door(owner: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::RemoveDoor as i16)?;
    p.write_byte(0)?;
    p.write_int(owner)?;
    Ok(p)
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

    fn expect(packet: &Packet, opcode: SendOpcode) -> Expect<'_> {
        let mut expect = Expect::new(&packet.bytes);
        expect.short(opcode as i16);
        expect
    }

    #[test]
    fn the_owner_hears_of_their_door_or_its_end() {
        let packet = build_spawn_portal(Some(&DOOR)).unwrap();
        let mut e = expect(&packet, SendOpcode::SpawnPortal);
        e.int(101_000_000)
            .int(101_010_000)
            .short(-120)
            .short(275)
            .end();

        let packet = build_spawn_portal(None).unwrap();
        let mut e = expect(&packet, SendOpcode::SpawnPortal);
        e.int(NO_DOOR).int(NO_DOOR).end();
    }

    #[test]
    fn a_door_opens_when_cast_and_stands_for_those_who_come_later() {
        let packet = build_spawn_door(7, (-120, 275), true).unwrap();
        let mut e = expect(&packet, SendOpcode::SpawnDoor);
        e.byte(0).int(7).short(-120).short(275).end();

        let packet = build_spawn_door(7, (-120, 275), false).unwrap();
        let mut e = expect(&packet, SendOpcode::SpawnDoor);
        e.byte(1).int(7).short(-120).short(275).end();

        let packet = build_remove_door(7).unwrap();
        let mut e = expect(&packet, SendOpcode::RemoveDoor);
        e.byte(0).int(7).end();
    }
}
