//! One-of-a-kind (`only`) items: a second copy on the floor can't be picked
//! up while one is held ("This item is unavailable for the pick-up.").

use integration_harness::database::grant_item;
use integration_harness::hunting::pickup;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ETC: i16 = 4;
/// One of a kind, tradeable, and lands when dropped.
const ONLY: i32 = 4_000_381;
const FOREST: i32 = 50_000;

fn throw(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC as u8).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn a_second_one_of_a_kind_item_stays_on_the_floor() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, ONLY).await.expect("one copy");
    grant_item(id, ETC, 2, ONLY).await.expect("another copy");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut t).await, FOREST);
    read_for(c, &mut t, 300).await;

    send(c, throw(2)).await;
    until(c, &mut t, |t| t.field.item_drop(ONLY).is_some()).await;
    let first = t.field.item_drop(ONLY).unwrap();

    // Still holding the other copy: refused, and the client is unlocked.
    let updates = t.stat_updates.len();
    send(c, pickup(first.object_id, first.x, first.y)).await;
    until(c, &mut t, |t| t.unavailable == 1).await;
    read_for(c, &mut t, 300).await;
    assert!(t.stat_updates[updates..].contains(&Default::default()));
    assert!(!t.added.iter().any(|a| a.2 == ONLY));

    // It went back on the ground (as a full bag's does), so once the other
    // copy is on the floor too, it can be picked up.
    send(c, throw(1)).await;
    until(c, &mut t, |t| t.removed.contains(&(ETC as u8, 1))).await;
    send(c, pickup(first.object_id, first.x, first.y)).await;
    until(c, &mut t, |t| t.added.iter().any(|a| a.2 == ONLY)).await;
    assert_eq!(t.unavailable, 1);
}
