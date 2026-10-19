//! Return scrolls: a Nearest Town scroll goes to the map's return map, and a
//! town scroll to its town while the player is in the same area. A scroll
//! that can't be used is kept.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const USE: i16 = 2;
const NEAREST_TOWN: i32 = 2_030_000;
const TO_HENESYS: i32 = 2_030_004;
const HENESYS: i32 = 100_000_000;
const ELLINIA: i32 = 101_000_000;
/// A field outside Ellinia whose return map is Ellinia.
const ELLINIA_FIELD: i32 = 101_010_000;
const LITH_HARBOR: i32 = 104_000_000;
const ORBIS: i32 = 200_000_000;

/// USE_RETURN_SCROLL of the scroll in USE slot `slot`.
fn read_scroll(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseReturnScroll as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// Read until the next map change, and return the map.
async fn arrival(c: &mut MapleTestConnection, t: &mut Trace) -> i32 {
    let before = t.warps.len();
    until(c, t, |t| t.warps.len() > before).await;
    t.warps[before]
}

#[tokio::test]
async fn return_scrolls_go_to_town_within_their_area() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, NEAREST_TOWN, 2)
        .await
        .expect("grant Nearest Town scrolls");
    grant_stack(id, USE, 2, TO_HENESYS, 2)
        .await
        .expect("grant Henesys scrolls");
    let mut player = relogin(&config).await;
    let (c, mut t) = (&mut player.connection, Trace::default());

    // Nearest Town takes the player to the field's return map.
    assert_eq!(warp(c, ELLINIA_FIELD, "", &mut t).await, ELLINIA_FIELD);
    send(c, read_scroll(1, NEAREST_TOWN)).await;
    assert_eq!(arrival(c, &mut t).await, ELLINIA);
    assert_eq!(t.quantities.last(), Some(&(USE as u8, 1, 1)));

    // A Henesys scroll doesn't leave Ossyria, and is kept.
    assert_eq!(warp(c, ORBIS, "", &mut t).await, ORBIS);
    send(c, read_scroll(2, TO_HENESYS)).await;
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
    assert_eq!(
        t.quantities.last(),
        Some(&(USE as u8, 1, 1)),
        "the Henesys scroll is kept"
    );

    // From elsewhere in Victoria it goes to Henesys.
    assert_eq!(warp(c, LITH_HARBOR, "", &mut t).await, LITH_HARBOR);
    send(c, read_scroll(2, TO_HENESYS)).await;
    assert_eq!(arrival(c, &mut t).await, HENESYS);
    assert_eq!(t.quantities.last(), Some(&(USE as u8, 2, 1)));
}
