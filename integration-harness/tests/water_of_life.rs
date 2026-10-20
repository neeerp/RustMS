//! The Water of Life: used, it brings Mar the Fairy, who offers to wake a
//! doll. The doll picked lives again and comes out, and the water is gone.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{date_pet, grant_pet, grant_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read, send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const KITTY: i32 = 5_000_000;
const WATER: i32 = 5_180_000;
const MAR: i32 = 1_032_102;
const CASH: u8 = 5;
/// The first pet's serial in a stat change.
const PET: i32 = 0x8;
/// NPC dialogue types: a page, a yes/no and a menu.
const SAY: u8 = 0;
const ASK: u8 = 1;
const MENU: u8 = 4;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// WATER_OF_LIFE: a Water of Life was double-clicked.
fn water_of_life() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::WaterOfLife as i16).unwrap();
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
async fn the_water_of_life_wakes_a_doll() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_400_000_000 + i64::from(owner_id);
    grant_pet(owner_id, 1, KITTY, serial, "Kitty")
        .await
        .expect("grant a pet");
    date_pet(serial, -60).await.expect("date the pet");
    grant_stack(owner_id, CASH.into(), 2, WATER, 1)
        .await
        .expect("grant the water");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    send(o, spawn_pet(1)).await;
    unlocked(o, &mut t).await;
    assert!(t.pets.is_empty(), "a doll stays home");

    send(o, water_of_life()).await;
    until(o, &mut t, |t| !t.prompts.is_empty()).await;
    assert_eq!(t.prompts, [(MAR, MENU)]);
    send(o, answer(MENU, 1, Some(0))).await;
    until(o, &mut t, |t| t.prompts.len() == 2).await;
    assert_eq!(t.prompts[1], (MAR, ASK));
    send(o, answer(ASK, 1, None)).await;
    until(o, &mut t, |t| t.prompts.len() == 3 && !t.added.is_empty()).await;
    assert_eq!(t.prompts[2], (MAR, SAY));
    assert!(t.removed.contains(&(CASH, 2)), "the water is used up");
    assert!(t.removed.contains(&(CASH, 1)));
    assert_eq!(t.added, [(CASH, 1, KITTY)]);

    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| !t.pets.is_empty() && t.stat(PET).is_some()).await;
    assert_eq!(t.stat(PET), Some(serial));
}
