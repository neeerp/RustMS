//! Map hazards: El Nath's Ice Valley (211040000) takes 10 HP every ten
//! seconds (its `decHP`), unless the player wears its `protectItem`, the
//! cape 1102109. The v83 client leaves this to the server.

use integration_harness::database::{set_hp, wear_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ICE_VALLEY: i32 = 211_040_000;
const CAPE: i32 = 1_102_109;
const CAPE_SLOT: i16 = -9;
const EQUIP: u8 = 1;

/// Take off the cape into the Equip bag's slot `to`.
fn take_off(to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(EQUIP).unwrap();
    p.write_short(CAPE_SLOT).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn the_cold_hurts_unless_the_cape_is_worn() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 50, 50).await.expect("50 HP");
    wear_equip(id, CAPE_SLOT, CAPE)
        .await
        .expect("wear the cape");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, ICE_VALLEY, "", &mut h).await, ICE_VALLEY);

    // The cape keeps the cold out.
    read_for(c, &mut h, 11_000).await;
    assert_eq!(h.stat(stat::HP), None);

    // Without it, the next tick takes 10.
    send(c, take_off(24)).await;
    let mut h = Trace::default();
    until(c, &mut h, |t| t.stat(stat::HP).is_some()).await;
    assert_eq!(h.stat(stat::HP), Some(40));
}
