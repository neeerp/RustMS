//! Catch items: the Pheromone Perfume, thrown at a Tamable Hog within its
//! reach, catches it for everyone to see and turns into the Hog mount. Out
//! of reach it catches nothing and is kept.
//!
//! The Hogs come back an hour after they're caught, so on one stack this
//! test passes twice, once per Hog.

use integration_harness::database::grant_stack;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const EQUIP: u8 = 1;
const USE: i16 = 2;
const PERFUME: i32 = 2_270_000;
const TAMABLE_HOG: i32 = 9_300_101;
/// The Hog mount the perfume turns into.
const HOG: i32 = 1_902_000;
const HOG_FIELD: i32 = 922_200_000;

/// USE_CATCH_ITEM of the item in USE slot `slot` at the mob `object_id`.
fn throw(slot: i16, item_id: i32, object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCatchItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(object_id).unwrap();
    p
}

#[tokio::test]
async fn the_perfume_catches_a_hog_in_reach() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, PERFUME, 1)
        .await
        .expect("grant the perfume");
    let mut player = relogin(&config).await;
    let (c, mut t) = (&mut player.connection, Trace::default());
    assert_eq!(warp(c, HOG_FIELD, "", &mut t).await, HOG_FIELD);
    until(c, &mut t, |t| !t.field.alive(TAMABLE_HOG).is_empty()).await;
    let hog = t.field.alive(TAMABLE_HOG)[0];
    let (x, y) = t.field.mob_positions[&hog];

    // From the spawn point every Hog is out of reach: the client is
    // unlocked and keeps its perfume.
    send(c, throw(1, PERFUME, hog)).await;
    let op = SendOpcode::StatChange as i16;
    let unlock = loop {
        let p = read(c, &mut t).await;
        if p.opcode() == op {
            break p;
        }
    };
    assert_eq!(
        unlock.bytes[2], 1,
        "an empty stat change unlocks the client"
    );
    assert!(t.field.catches.is_empty());
    assert!(t.removed.is_empty(), "the perfume is kept");

    // Beside it, the Hog is caught, and the perfume becomes the mount.
    send(c, walk_to(x, y)).await;
    send(c, throw(1, PERFUME, hog)).await;
    until(c, &mut t, |t| {
        t.field.killed.contains(&hog) && t.added.iter().any(|&(_, _, item)| item == HOG)
    })
    .await;
    assert_eq!(t.field.catches, [(hog, PERFUME, true)]);
    assert_eq!(t.field.leave_types.get(&hog), Some(&1));
    assert_eq!(t.removed.last(), Some(&(USE as u8, 1)));
    assert!(t
        .added
        .iter()
        .any(|&(kind, _, item)| kind == EQUIP && item == HOG));
}
