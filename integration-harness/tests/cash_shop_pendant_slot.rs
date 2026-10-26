use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::{set_nx_credit, stow_equip, wear_equip};
use integration_harness::packets::{
    build_cash_shop_buy, build_cash_shop_buy_pendant_slot, build_enter_cash_shop, opcode_name,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{timeout, Duration};

/// The second pendant slot for 30 days (5550000), which the shop adds to
/// the client's list.
const PENDANT_SLOT: i32 = 50200172;
/// Pendant of the Spirit, which any level may wear.
const PENDANT: i32 = 1122017;
/// 1900, the expiry of a slot never bought.
const NEVER: i64 = 94_354_848_000_000_000;

/// Read until `expected`, skipping whatever the map sends meanwhile.
async fn read_until(
    connection: &mut MapleTestConnection,
    phase: &'static str,
    expected: SendOpcode,
) -> PacketEnvelope {
    let expected = expected as i16;
    for _ in 0..32 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .unwrap_or_else(|_| panic!("{phase}: timed out"))
            .unwrap_or_else(|e| panic!("{phase}: {e}"));
        if envelope.opcode() == expected {
            return envelope;
        }
    }
    panic!("{phase}: never received {}", opcode_name(expected));
}

/// Enter the Cash Shop and wait for its lists and balance.
async fn enter_cash_shop(connection: &mut MapleTestConnection) {
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    read_until(connection, "enter cash shop", SendOpcode::SetCashShop).await;
    read_until(connection, "cash balance", SendOpcode::QueryCashResult).await;
}

/// Send a Cash Shop request and return the CASHSHOP_OPERATION reply.
async fn request(
    connection: &mut MapleTestConnection,
    packet: Packet,
    phase: &'static str,
) -> Vec<u8> {
    connection.send_packet(packet, phase).await.expect(phase);
    let reply = read_until(connection, phase, SendOpcode::CashShopOperation).await;
    reply.packet.bytes[2..].to_vec()
}

/// The NX Credit in the next balance (QUERY_CASH_RESULT).
async fn credit(connection: &mut MapleTestConnection, phase: &'static str) -> i32 {
    let balance = read_until(connection, phase, SendOpcode::QueryCashResult).await;
    Cursor::new(&balance.packet.bytes[2..])
        .read_int()
        .expect("NX Credit")
}

/// Double-click the pendant in Equip slot 1 onto `to`: whether it moved
/// (INVENTORY_OPERATION) rather than was refused (an empty STAT_CHANGED).
async fn wear(connection: &mut MapleTestConnection, to: i16, phase: &'static str) -> bool {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(1).unwrap();
    p.write_short(1).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    connection.send_packet(p, phase).await.expect(phase);
    for _ in 0..32 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .unwrap_or_else(|_| panic!("{phase}: timed out"))
            .unwrap_or_else(|e| panic!("{phase}: {e}"));
        match envelope.opcode() {
            op if op == SendOpcode::InventoryOperation as i16 => return true,
            op if op == SendOpcode::StatChange as i16 => return false,
            _ => {}
        }
    }
    panic!("{phase}: no reply to the move");
}

/// Now plus `days`, as a FILETIME.
fn filetime_in(days: u64) -> i64 {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + days * 86_400;
    (unix as i64 + 11_644_473_600) * 10_000_000
}

#[tokio::test]
async fn nx_opens_the_second_pendant_slot_for_thirty_days() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    assert_eq!(session.extra_pendant, NEVER);
    assert!(session.pendant_cell, "every Equip window has the cell");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(id, -17, PENDANT).await.expect("a pendant worn");
    stow_equip(id, 1, PENDANT, 0)
        .await
        .expect("a second pendant");

    // Shut, the slot takes nothing: the move is refused with no changes.
    let mut session = relogin(&config).await;
    assert!(!wear(&mut session.connection, -51, "wear into the shut slot").await);

    set_nx_credit(&config.username, 10_000)
        .await
        .expect("grant NX");
    enter_cash_shop(&mut session.connection).await;
    let connection = &mut session.connection;
    // Not as an ordinary purchase.
    let buy = build_cash_shop_buy(1, PENDANT_SLOT).unwrap();
    let failed = request(connection, buy, "buy the slot as an item").await;
    assert_eq!(failed, [0x58, 0x00]);
    // Thirty days for 2,000 NX, twice over; Maple Points the account lacks.
    for (phase, left) in [("buy the slot", 8_000), ("buy it again", 6_000)] {
        let buy = build_cash_shop_buy_pendant_slot(false, PENDANT_SLOT).unwrap();
        assert_eq!(request(connection, buy, phase).await, [0x66, 0, 0, 30, 0]);
        assert_eq!(credit(connection, phase).await, left);
    }
    let buy = build_cash_shop_buy_pendant_slot(true, PENDANT_SLOT).unwrap();
    let failed = request(connection, buy, "buy with no Maple Points").await;
    assert_eq!(failed, [0x5C, 0xA5]);
    drop(session);

    // Open for sixty days now, and the pendant goes in.
    let mut session = relogin(&config).await;
    let hour = 3_600 * 10_000_000;
    assert!((session.extra_pendant - filetime_in(60)).abs() < hour);
    assert!(wear(&mut session.connection, -51, "wear into the open slot").await);
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.equipped.get(&51), Some(&PENDANT));
    assert_eq!(session.equipped.get(&17), Some(&PENDANT));
}
