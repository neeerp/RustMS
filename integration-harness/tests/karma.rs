//! The Scissors of Karma: an equip with `tradeAvailable` is given its Karma
//! (flag 0x10) for one scissors, and a second pair is refused.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, item_flags_and_hours, stow_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;
use tokio::time::timeout;

const EQUIP: i16 = 1;
const CASH: i16 = 5;
const KARMA_SCISSORS: i32 = 5_520_000;
/// A sword with `tradeAvailable`.
const SWORD: i32 = 1_302_112;
const SLOT: i16 = 10;
const KARMA: i16 = 0x10;

fn scissors(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(KARMA_SCISSORS).unwrap();
    p.write_int(i32::from(EQUIP)).unwrap();
    p.write_int(i32::from(SLOT)).unwrap();
    p.write_int(0).unwrap();
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
        let p = timeout(Duration::from_secs(5), c.read_packet("karma"))
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
async fn scissors_give_an_equip_its_karma_once() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, SLOT, SWORD, 7).await.expect("a sword");
    grant_item(id, CASH, 1, KARMA_SCISSORS)
        .await
        .expect("scissors");
    grant_item(id, CASH, 2, KARMA_SCISSORS)
        .await
        .expect("more scissors");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    send(c, scissors(1)).await;
    // Exclusive: the scissors go, then the sword is put back with its Karma.
    let (op, body) = answer(c).await;
    assert_eq!(
        (op, &body[..2]),
        (SendOpcode::InventoryOperation as i16, &[1, 3][..])
    );
    send(c, scissors(2)).await;
    let (op, _) = answer(c).await;
    assert_eq!(op, SendOpcode::StatChange as i16, "cut once only");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (flags, _) = item_flags_and_hours(id, SWORD).await.unwrap();
    assert_eq!(flags & KARMA, KARMA);
}
