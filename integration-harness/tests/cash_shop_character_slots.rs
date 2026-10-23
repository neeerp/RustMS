use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::{set_character_slots, set_nx_credit};
use integration_harness::packets::{
    build_cash_shop_buy, build_cash_shop_buy_character_slot, build_check_cash,
    build_enter_cash_shop, opcode_name,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// The Extra Character Slot Coupon's commodity (5430000, 6,900 NX).
const CHARACTER_SLOT: i32 = 50200083;

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

/// Enter the Cash Shop and return the character slots its Cash Inventory
/// (0x4B) ends with.
async fn enter_cash_shop(connection: &mut MapleTestConnection) -> i16 {
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    read_until(connection, "enter cash shop", SendOpcode::SetCashShop).await;
    let mut slots = None;
    for _ in 0..3 {
        let list = read_until(connection, "cash shop lists", SendOpcode::CashShopOperation).await;
        let bytes = &list.packet.bytes;
        if bytes[2] == 0x4B {
            slots = Some(i16::from_le_bytes([
                bytes[bytes.len() - 2],
                bytes[bytes.len() - 1],
            ]));
        }
    }
    read_until(connection, "cash balance", SendOpcode::QueryCashResult).await;
    slots.expect("no Cash Inventory")
}

/// Send a Cash Shop request and return the CASHSHOP_OPERATION reply.
async fn request(
    connection: &mut MapleTestConnection,
    packet: packet::Packet,
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

#[tokio::test]
async fn nx_buys_character_slots_up_to_fifteen() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    set_nx_credit(&config.username, 10_000)
        .await
        .expect("grant NX");
    set_character_slots(&config.username, 3)
        .await
        .expect("set slots");
    assert_eq!(enter_cash_shop(&mut session.connection).await, 3);
    let connection = &mut session.connection;

    // The coupon adds a slot at once; the balance follows to unlock the shop.
    let buy = build_cash_shop_buy_character_slot(1, CHARACTER_SLOT).unwrap();
    assert_eq!(request(connection, buy, "buy a slot").await, [0x64, 4, 0]);
    assert_eq!(credit(connection, "buy a slot").await, 3_100);

    let buy = build_cash_shop_buy_character_slot(1, CHARACTER_SLOT).unwrap();
    let failed = request(connection, buy, "buy a slot without NX").await;
    assert_eq!(failed, [0x5C, 0xA5]);

    // Not as an item for the Cash Inventory.
    let buy = build_cash_shop_buy(1, CHARACTER_SLOT).unwrap();
    let failed = request(connection, buy, "buy the coupon as an item").await;
    assert_eq!(failed, [0x5C, 0x00]);
    drop(session);

    // The slot stays; at 15, a purchase fails and costs nothing.
    let mut session = relogin(&config).await;
    assert_eq!(enter_cash_shop(&mut session.connection).await, 4);
    let connection = &mut session.connection;
    set_nx_credit(&config.username, 10_000)
        .await
        .expect("grant NX");
    set_character_slots(&config.username, 15)
        .await
        .expect("set slots");
    let buy = build_cash_shop_buy_character_slot(1, CHARACTER_SLOT).unwrap();
    let failed = request(connection, buy, "buy a sixteenth slot").await;
    assert_eq!(failed, [0x5C, 0x00]);
    connection
        .send_packet(build_check_cash(), "check")
        .await
        .expect("send check");
    assert_eq!(credit(connection, "check").await, 10_000);
}
