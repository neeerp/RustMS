//! The Item Tag and a Magical Sandglass, each dropped on a worn equip: the
//! tag prints the user's name on it, the sandglass adds a week to the time
//! it lasts. Each is spent, and the answer unlocks the client.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{date_item, grant_item, item_owner_and_hours, wear_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;
use tokio::time::timeout;

const CASH: i16 = 5;
const TAG: i32 = 5_060_000;
/// [7days]Magical Sandglass.
const WEEK: i32 = 5_500_001;
const ZENUMIST_CAPE: i32 = 1_102_135;
const CAPE: i16 = -9;
const DAY: i32 = 86_400;

fn use_on(slot: i16, item_id: i32, target: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_short(target).unwrap();
    p.write_int(0).unwrap();
    p
}

/// Read until an INVENTORY_OPERATION; return its body.
async fn operation(c: &mut MapleTestConnection) -> Vec<u8> {
    let op = SendOpcode::InventoryOperation as i16;
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("item tag"))
            .await
            .expect("no inventory operation within 5 s")
            .expect("read packet")
            .packet;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

#[tokio::test]
async fn a_tag_and_a_sandglass_change_a_worn_cape() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(id, CAPE, ZENUMIST_CAPE).await.expect("a cape");
    date_item(id, ZENUMIST_CAPE, 10 * DAY)
        .await
        .expect("ten days");
    grant_item(id, CASH, 1, TAG).await.expect("a tag");
    grant_item(id, CASH, 2, WEEK).await.expect("a sandglass");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    send(c, use_on(1, TAG, CAPE)).await;
    // Exclusive: the cash item goes, then the cape is put back changed.
    let body = operation(c).await;
    assert_eq!(body[..2], [1, 3], "an exclusive answer of three changes");
    send(c, use_on(2, WEEK, CAPE)).await;
    let body = operation(c).await;
    assert_eq!(body[..2], [1, 3]);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (owner, hours) = item_owner_and_hours(id, ZENUMIST_CAPE).await.unwrap();
    assert_eq!(owner, config.character_name);
    assert_eq!(hours, Some(17 * 24 - 1), "ten days and a week");
    let session = relogin(&config).await;
    assert_eq!(session.items.get(&TAG), None);
    assert_eq!(session.items.get(&WEEK), None);
}
