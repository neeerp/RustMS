use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::create_coupon;
use integration_harness::login_to_world_session;
use integration_harness::packets::{build_coupon_code, build_enter_cash_shop, opcode_name};
use integration_harness::preconditions::load_harness_config_or_fail;
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// Kinds of coupon reward.
const ITEM: i16 = 0;
const CREDIT: i16 = 1;
const MAPLE_POINTS: i16 = 2;
const MESOS: i16 = 4;

const RED_POTION: i32 = 2000000;
const SAFETY_CHARM: i32 = 5130000;

/// Read until `expected`, returning it and the opcodes read before it.
async fn read_until(
    connection: &mut MapleTestConnection,
    phase: &'static str,
    expected: SendOpcode,
) -> (PacketEnvelope, Vec<i16>) {
    let expected = expected as i16;
    let mut before = Vec::new();
    for _ in 0..32 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .unwrap_or_else(|_| panic!("{phase}: timed out"))
            .unwrap_or_else(|e| panic!("{phase}: {e}"));
        if envelope.opcode() == expected {
            return (envelope, before);
        }
        before.push(envelope.opcode());
    }
    panic!("{phase}: never received {}", opcode_name(expected));
}

async fn enter_cash_shop(connection: &mut MapleTestConnection) {
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    read_until(connection, "enter cash shop", SendOpcode::SetCashShop).await;
    read_until(connection, "cash balance", SendOpcode::QueryCashResult).await;
}

/// Type `code` in the Coupon dialog: the CASHSHOP_OPERATION reply, what
/// came before it, and the NX Credit in the balances that follow.
async fn redeem(
    connection: &mut MapleTestConnection,
    code: &str,
    phase: &'static str,
) -> (Vec<u8>, Vec<i16>, i32) {
    connection
        .send_packet(build_coupon_code(code).unwrap(), phase)
        .await
        .expect(phase);
    let (reply, before) = read_until(connection, phase, SendOpcode::CashShopOperation).await;
    let (balance, _) = read_until(connection, phase, SendOpcode::QueryCashResult).await;
    let credit = Cursor::new(&balance.packet.bytes[2..])
        .read_int()
        .expect("NX Credit");
    (reply.packet.bytes[2..].to_vec(), before, credit)
}

/// The failure (0x5C) code of a reply.
fn failure(reply: &[u8]) -> u8 {
    assert_eq!(reply[0], 0x5C, "expected a failure, got {reply:02X?}");
    reply[1]
}

#[tokio::test]
async fn coupons_give_items_nx_and_mesos_once() {
    let config = load_harness_config_or_fail().await;
    let tag = config.character_name.to_uppercase();
    let (gift, old, big) = (format!("G{tag}"), format!("O{tag}"), format!("B{tag}"));
    create_coupon(
        &gift,
        false,
        &[
            (CREDIT, 0, 500),
            (MAPLE_POINTS, 0, 200),
            (ITEM, RED_POTION, 5),
            (ITEM, SAFETY_CHARM, 1),
            (MESOS, 0, 1000),
        ],
    )
    .await
    .expect("create coupon");
    create_coupon(&old, true, &[(CREDIT, 0, 500)])
        .await
        .expect("create expired coupon");
    // More potions than the Use bag holds.
    create_coupon(&big, false, &[(ITEM, RED_POTION, 30_000)])
        .await
        .expect("create a coupon too big");

    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    let connection = &mut session.connection;
    enter_cash_shop(connection).await;

    let (reply, _, credit) = redeem(connection, &format!("N{tag}"), "unknown code").await;
    assert_eq!(failure(&reply), 0xB0);
    assert_eq!(credit, 0);

    // Codes are typed in any case.
    let typed = format!("  {}", gift.to_lowercase());
    let (reply, before, credit) = redeem(connection, &typed, "redeem").await;
    assert_eq!(reply[..2], [0x59, 1], "one cash item: {reply:02X?}");
    let item_id = Cursor::new(&reply[18..]).read_int().unwrap();
    assert_eq!(item_id, SAFETY_CHARM);
    let mut rest = Cursor::new(&reply[2 + 55..]);
    assert_eq!(rest.read_int().unwrap(), 200, "Maple Points");
    assert_eq!(rest.read_int().unwrap(), 1, "one bag item");
    assert_eq!(rest.read_short().unwrap(), 5);
    assert_eq!(rest.read_short().unwrap(), 0);
    assert_eq!(rest.read_int().unwrap(), RED_POTION);
    assert_eq!(rest.read_int().unwrap(), 1000, "mesos");
    assert!(before.contains(&(SendOpcode::InventoryOperation as i16)));
    assert!(before.contains(&(SendOpcode::StatChange as i16)));
    assert_eq!(credit, 500);

    let (reply, _, credit) = redeem(connection, &gift, "redeem again").await;
    assert_eq!(failure(&reply), 0xB3);
    assert_eq!(credit, 500);

    let (reply, _, _) = redeem(connection, &old, "expired").await;
    assert_eq!(failure(&reply), 0xB2);

    // Nothing is given, and the coupon stays for later.
    for phase in ["bag full", "bag still full"] {
        let (reply, before, _) = redeem(connection, &big, phase).await;
        assert_eq!(failure(&reply), 0xBB, "{phase}");
        assert!(!before.contains(&(SendOpcode::InventoryOperation as i16)));
    }

    // The redemption cleared the first wrong code; the third in a row
    // sends the player out.
    for (n, code) in [0xB0, 0xB0, 0xB1].into_iter().enumerate() {
        let (reply, _, _) = redeem(connection, &format!("W{n}{tag}"), "wrong code").await;
        assert_eq!(failure(&reply), code, "wrong code {n}");
    }
}
