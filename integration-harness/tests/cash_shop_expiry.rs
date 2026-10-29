//! A Cash Inventory item whose time runs out while its owner is in the Cash
//! Shop goes, and the client is told which (CASHSHOP_OPERATION 0x6E, v83
//! 0x47b1ab), while the others stay.

use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::{cash_item_exists, expire_cash_item_in, gift_cash_item};
use integration_harness::login_to_world_session;
use integration_harness::packets::{build_enter_cash_shop, opcode_name};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::relogin;
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{sleep, timeout, Duration};

/// Two cash hats.
const TRANSPARENT_HAT: i32 = 1_002_186;
const BLUE_COWBOY_HAT: i32 = 1_002_187;
const CASH_INVENTORY: u8 = 0x4B;
const EXPIRED: u8 = 0x6E;

/// The next CASHSHOP_OPERATION of `mode`.
async fn read_mode(connection: &mut MapleTestConnection, mode: u8) -> PacketEnvelope {
    for _ in 0..64 {
        let envelope = timeout(Duration::from_secs(10), connection.read_packet("cash shop"))
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for mode {mode:#x}"))
            .expect("read");
        if envelope.opcode() == SendOpcode::CashShopOperation as i16
            && envelope.packet.bytes[2] == mode
        {
            return envelope;
        }
        assert_ne!(
            (envelope.opcode(), envelope.packet.bytes.get(2)),
            (SendOpcode::CashShopOperation as i16, Some(&EXPIRED)),
            "an unexpected expiry ({})",
            opcode_name(envelope.opcode())
        );
    }
    panic!("never received mode {mode:#x}");
}

#[tokio::test]
async fn a_locker_item_whose_time_runs_out_in_the_shop_goes() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.expect("login");
    let id = session.character_id;
    drop(session);
    sleep(Duration::from_millis(500)).await;
    let short = gift_cash_item(id, TRANSPARENT_HAT, "").await.expect("gift");
    let long = gift_cash_item(id, BLUE_COWBOY_HAT, "").await.expect("gift");
    expire_cash_item_in(short, 3).await.expect("expiry");
    expire_cash_item_in(long, 3600).await.expect("expiry");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    c.send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    let listed = read_mode(c, CASH_INVENTORY).await;
    let count = Cursor::new(&listed.packet.bytes[3..]).read_short().unwrap();
    assert_eq!(count, 2);

    let expired = read_mode(c, EXPIRED).await;
    let bytes = &expired.packet.bytes;
    assert_eq!(bytes.len(), 11, "{bytes:?}");
    assert_eq!(Cursor::new(&bytes[3..]).read_long().unwrap(), short);
    assert!(!cash_item_exists(short).await.unwrap());
    assert!(cash_item_exists(long).await.unwrap());
}
