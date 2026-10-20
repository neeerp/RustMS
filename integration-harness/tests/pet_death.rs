//! Pet death: when a summoned pet's Water of Life dries up, it goes back
//! for good. Its item is re-sent dated 2079, which the client takes for a
//! doll, and a doll isn't summoned again, even after a relog.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{date_pet, grant_pet};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const KITTY: i32 = 5_000_000;
const CASH: u8 = 5;
/// PetGone::Expired: the pet's magical time has run out.
const EXPIRED: u8 = 2;
/// The first pet's serial in a stat change.
const PET: i32 = 0x8;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// Read until the empty stat change that unlocks the client.
async fn unlocked(c: &mut MapleTestConnection, t: &mut Trace) {
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::StatChange as i16 && p.bytes[2..] == [1, 0, 0, 0, 0] {
            return;
        }
    }
}

#[tokio::test]
async fn a_pet_whose_time_runs_out_is_a_doll_for_good() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_200_000_000 + i64::from(owner_id);
    grant_pet(owner_id, 1, KITTY, serial, "Kitty")
        .await
        .expect("grant a pet");
    date_pet(serial, 8).await.expect("date the pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| !t.pets.is_empty() && t.stat(PET).is_some()).await;
    assert_eq!(t.stat(PET), Some(serial));

    // Its time runs out: it goes back, and its item is re-sent.
    until(o, &mut t, |t| {
        !t.pets_gone.is_empty() && !t.added.is_empty()
    })
    .await;
    assert_eq!(t.pets_gone, [(owner_id, 0, EXPIRED)]);
    assert_eq!(t.removed, [(CASH, 1)]);
    assert_eq!(t.added, [(CASH, 1, KITTY)]);
    until(o, &mut t, |t| t.stat(PET) == Some(0)).await;

    // The doll stays home.
    send(o, spawn_pet(1)).await;
    unlocked(o, &mut t).await;
    assert_eq!(t.pets.len(), 1, "no pet comes out");

    drop(owner);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());
    send(o, spawn_pet(1)).await;
    unlocked(o, &mut t).await;
    assert!(t.pets.is_empty(), "still a doll after a relog");
}
