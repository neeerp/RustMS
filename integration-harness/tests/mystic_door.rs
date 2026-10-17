//! Mystic Door (2311002) on The Road to the Dungeon (101010000): a Priest's
//! door opens for the whole map and leads its owner to Ellinia's first door
//! portal and back. Others may use it only once in the Priest's party, who
//! then hear of it, and it closes when the Priest leaves.

use integration_harness::database::{grant_skill, grant_stack, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const FIELD: i32 = 101_010_000;
const ELLINIA: i32 = 101_000_000;
/// Ellinia's first town door portal.
const ELLINIA_DOOR: u8 = 49;
const PRIEST: i16 = 231;
const MYSTIC_DOOR: i32 = 2311002;
const ETC: i16 = 4;
const MAGIC_ROCK: i32 = 4006000;
/// The field's start point, away from its other portals.
const AT: (i16, i16) = (-1711, 1914);
const PARTY_ONLY: u8 = 6;
const OP_JOIN: u8 = 0x0F;
const NO_DOOR: i32 = 999_999_999;

fn cast_door(x: i16, y: i16) -> Packet {
    let mut p = use_skill(MYSTIC_DOOR, 20);
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p
}

fn use_door(owner: i32, from_town: bool) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseDoor as i16).unwrap();
    p.write_int(owner).unwrap();
    p.write_byte(u8::from(from_town)).unwrap();
    p
}

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn invite(name: &str) -> Packet {
    let mut p = party_operation(4);
    p.write_str_with_length(name).unwrap();
    p
}

fn accept(party_id: i32) -> Packet {
    let mut p = party_operation(3);
    p.write_int(party_id).unwrap();
    p
}

#[tokio::test]
async fn a_priests_door_leads_to_town_and_back_for_their_party() {
    let config = load_harness_config_or_fail().await;
    let priest_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(priest_id, PRIEST, 70, 2000)
        .await
        .expect("make a priest");
    grant_skill(priest_id, MYSTIC_DOOR, 20)
        .await
        .expect("teach Mystic Door");
    // Each door costs a Magic Rock.
    grant_stack(priest_id, ETC, 1, MAGIC_ROCK, 5)
        .await
        .expect("give Magic Rocks");

    let mut priest = relogin(&config).await;
    let other_config = load_harness_config_or_fail().await;
    let mut other = login_to_world_session(&other_config).await.unwrap();
    let (p, o) = (&mut priest.connection, &mut other.connection);
    let (mut pt, mut ot) = (Trace::default(), Trace::default());

    // No door in a town.
    assert_eq!(warp(p, ELLINIA, "", &mut pt).await, ELLINIA);
    send(p, cast_door(0, 0)).await;
    read_for(p, &mut pt, 300).await;
    assert!(pt.own_doors.is_empty(), "a door opened in town");

    // In the field the door opens for everyone there; its owner hears of it.
    assert_eq!(warp(o, FIELD, "", &mut ot).await, FIELD);
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    send(p, cast_door(AT.0, AT.1)).await;
    until(o, &mut ot, |t| !t.field.doors.is_empty()).await;
    until(p, &mut pt, |t| !t.own_doors.is_empty()).await;
    assert_eq!(ot.field.doors, [(priest_id, AT, true)]);
    assert_eq!(pt.own_doors, [Some((ELLINIA, FIELD, AT.0, AT.1))]);
    until(p, &mut pt, |t| !t.quantities.is_empty()).await;
    assert_eq!(pt.quantities, [(ETC as u8, 1, 4)]);

    // The owner goes to town and comes back to the door.
    pt.arrivals.clear();
    send(p, use_door(priest_id, false)).await;
    until(p, &mut pt, |t| !t.arrivals.is_empty()).await;
    assert_eq!(pt.arrivals[0], (ELLINIA, ELLINIA_DOOR, None));
    pt.field = Default::default();
    send(p, use_door(priest_id, true)).await;
    until(p, &mut pt, |t| {
        t.arrivals.len() == 2 && !t.field.doors.is_empty()
    })
    .await;
    let (map, _, chase) = pt.arrivals[1];
    assert_eq!(
        (map, chase),
        (FIELD, Some((i32::from(AT.0), i32::from(AT.1))))
    );
    assert_eq!(pt.field.doors, [(priest_id, AT, false)]);

    // A stranger may not pass.
    ot.arrivals.clear();
    send(o, use_door(priest_id, false)).await;
    until(o, &mut ot, |t| !t.blocked.is_empty()).await;
    assert_eq!(ot.blocked, [(SendOpcode::BlockedMap as i16, PARTY_ONLY)]);
    assert!(ot.arrivals.is_empty());

    // In the priest's party they hear of the door and may.
    send(p, invite(&other.character_name)).await;
    until(o, &mut ot, |t| !t.party_invites.is_empty()).await;
    let party_id = ot.party_invites[0].0;
    send(o, accept(party_id)).await;
    until(o, &mut ot, |t| t.party_ops.contains(&OP_JOIN)).await;
    until(p, &mut pt, |t| t.party_ops.contains(&OP_JOIN)).await;
    // Casting again closes the old door before opening the new one.
    send(p, cast_door(AT.0, AT.1)).await;
    until(o, &mut ot, |t| t.party_doors.len() == 2).await;
    assert_eq!(
        ot.party_doors,
        [(0, NO_DOOR, NO_DOOR, 0, 0), (0, ELLINIA, FIELD, AT.0, AT.1)]
    );
    send(o, use_door(priest_id, false)).await;
    until(o, &mut ot, |t| !t.arrivals.is_empty()).await;
    assert_eq!(ot.arrivals[0], (ELLINIA, ELLINIA_DOOR, None));
    send(o, use_door(priest_id, true)).await;
    until(o, &mut ot, |t| t.arrivals.len() == 2).await;
    ot.field = Default::default();
    assert_eq!(ot.arrivals[1].0, FIELD);

    // The door closes when the priest leaves.
    drop(priest);
    until(o, &mut ot, |t| t.field.doors_removed.contains(&priest_id)).await;
}
