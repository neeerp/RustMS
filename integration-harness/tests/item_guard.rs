//! The Item Guards: a timed one seals an equip for its days, another
//! extends the seal, and the lasting one makes it for good. A sealed item
//! can't be dropped. A timed seal that ran out is gone at the next login,
//! the item kept, and the client says so (SHOW_STATUS_INFO 11).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    date_item, flag_item, grant_item, item_flags_and_hours, stow_equip,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;
use tokio::time::timeout;

const EQUIP: i16 = 1;
const CASH: i16 = 5;
const ITEM_GUARD: i32 = 5_060_001;
/// Item Guard : 7 Days.
const WEEK: i32 = 5_061_000;
/// A pole arm, which beginners don't wear.
const POLE_ARM: i32 = 1_442_000;
const SLOT: i16 = 10;
const LOCK: i16 = 0x01;

fn guard(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_int(i32::from(EQUIP)).unwrap();
    p.write_int(i32::from(SLOT)).unwrap();
    p.write_int(0).unwrap();
    p
}

fn drop_pole_arm() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(EQUIP as u8).unwrap();
    p.write_short(SLOT).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

/// Read until an INVENTORY_OPERATION or a STAT_CHANGE; return its opcode
/// and body.
async fn answer(c: &mut MapleTestConnection) -> (i16, Vec<u8>) {
    let ops = [
        SendOpcode::InventoryOperation as i16,
        SendOpcode::StatChange as i16,
    ];
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("item guard"))
            .await
            .expect("no answer within 5 s")
            .expect("read packet")
            .packet;
        if ops.contains(&p.opcode()) {
            return (p.opcode(), p.bytes[2..].to_vec());
        }
    }
}

#[tokio::test]
async fn guards_seal_extend_and_make_lasting() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, SLOT, POLE_ARM, 7).await.expect("a pole arm");
    grant_item(id, CASH, 1, WEEK).await.expect("a timed guard");
    grant_item(id, CASH, 2, WEEK).await.expect("another");
    grant_item(id, CASH, 3, ITEM_GUARD)
        .await
        .expect("a lasting guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let inventory = SendOpcode::InventoryOperation as i16;
    for slot in 1..=3 {
        send(c, guard(slot, if slot == 3 { ITEM_GUARD } else { WEEK })).await;
        // Exclusive: the guard goes, then the pole arm is put back sealed.
        let (op, body) = answer(c).await;
        assert_eq!((op, &body[..2]), (inventory, &[1, 3][..]));
    }
    send(c, drop_pole_arm()).await;
    let (op, _) = answer(c).await;
    assert_eq!(op, SendOpcode::StatChange as i16, "a sealed pole arm stays");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (flags, hours) = item_flags_and_hours(id, POLE_ARM).await.unwrap();
    assert_eq!((flags, hours), (LOCK, None), "sealed for good");
}

#[tokio::test]
async fn a_timed_seal_extends_then_runs_out() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, SLOT, POLE_ARM, 7).await.expect("a pole arm");
    grant_item(id, CASH, 1, WEEK).await.expect("a timed guard");
    grant_item(id, CASH, 2, WEEK).await.expect("another");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    for slot in 1..=2 {
        send(c, guard(slot, WEEK)).await;
        answer(c).await;
    }
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (flags, hours) = item_flags_and_hours(id, POLE_ARM).await.unwrap();
    assert_eq!((flags, hours), (LOCK, Some(14 * 24 - 1)), "two weeks");

    date_item(id, POLE_ARM, -60).await.expect("an ended seal");
    flag_item(id, POLE_ARM, LOCK).await.expect("still sealed");
    let mut session = relogin(&config).await;
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| {
        !t.seals_expired.is_empty()
    })
    .await;
    assert_eq!(t.seals_expired, [POLE_ARM]);
    assert!(t.expired.is_empty(), "the pole arm stays");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (flags, hours) = item_flags_and_hours(id, POLE_ARM).await.unwrap();
    assert_eq!((flags, hours), (0, None), "unsealed, lasting");
}
