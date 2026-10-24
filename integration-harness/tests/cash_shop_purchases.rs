use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::{item_owner_and_hours, set_meso, set_nx_credit};
use integration_harness::packets::{
    build_cash_shop_buy, build_cash_shop_buy_package, build_cash_shop_buy_with_mesos,
    build_cash_shop_expand, build_cash_shop_gift, build_cash_shop_put_back,
    build_cash_shop_take_out, build_cash_shop_wish_list, build_check_cash, build_enter_cash_shop,
    opcode_name,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

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

/// Commodity serial numbers from Etc.nx: Gachapon Tickets (1000 NX, on
/// sale), more tickets (on sale), and a hat no longer sold.
const TICKET: i32 = 50200005;
const TICKETS: i32 = 50200003;
const RETIRED_HAT: i32 = 20000000;
/// The Baby Tiger Package (16,900 NX): a pet, its equipment and Pet Food.
const PACKAGE: i32 = 10002307;

/// Enter the Cash Shop and return the wish list it opens with.
async fn enter_cash_shop(connection: &mut MapleTestConnection) -> Vec<i32> {
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    read_until(connection, "enter cash shop", SendOpcode::SetCashShop).await;
    let mut wish_list = None;
    for _ in 0..3 {
        let list = read_until(connection, "cash shop lists", SendOpcode::CashShopOperation).await;
        if list.packet.bytes[2] == 0x4F {
            wish_list = Some(wish_list_sns(&list));
        }
    }
    read_until(connection, "cash balance", SendOpcode::QueryCashResult).await;
    wish_list.expect("no wish list")
}

fn wish_list_sns(envelope: &PacketEnvelope) -> Vec<i32> {
    let mut reader = Cursor::new(&envelope.packet.bytes[3..]);
    (0..10)
        .map(|_| reader.read_int().expect("wish list slot"))
        .filter(|&sn| sn != 0)
        .collect()
}

/// The CASHSHOP_OPERATION failure (0x5C) code the server sent back.
async fn failure(connection: &mut MapleTestConnection, phase: &'static str) -> u8 {
    let reply = read_until(connection, phase, SendOpcode::CashShopOperation).await;
    assert_eq!(reply.packet.bytes[2], 0x5C, "{phase}: expected a failure");
    reply.packet.bytes[3]
}

/// The code of a failed buy (0x58), which a sold-out answer follows with the
/// commodity's SN.
async fn buy_failure(connection: &mut MapleTestConnection, phase: &'static str, sn: i32) -> u8 {
    let reply = read_until(connection, phase, SendOpcode::CashShopOperation).await;
    let bytes = &reply.packet.bytes[2..];
    assert_eq!(bytes[0], 0x58, "{phase}: expected a failed buy");
    if bytes[1] == 0xC0 {
        assert_eq!(&bytes[2..], &sn.to_le_bytes(), "{phase}: the SN");
    } else {
        assert_eq!(bytes.len(), 2, "{phase}");
    }
    bytes[1]
}

#[tokio::test]
async fn purchases_need_cash_and_the_wish_list_is_kept() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    enter_cash_shop(&mut session.connection).await;

    // A new account has no NX.
    session
        .connection
        .send_packet(build_cash_shop_buy(1, TICKET).unwrap(), "buy")
        .await
        .expect("send purchase");
    assert_eq!(
        buy_failure(&mut session.connection, "buy without NX", TICKET).await,
        0xA5
    );

    session
        .connection
        .send_packet(build_cash_shop_buy(1, RETIRED_HAT).unwrap(), "buy")
        .await
        .expect("send purchase");
    assert_eq!(
        buy_failure(&mut session.connection, "buy a retired item", RETIRED_HAT).await,
        0xC0
    );

    // Items the account doesn't hold can't move either way.
    session
        .connection
        .send_packet(build_cash_shop_take_out(999_999, 5, 1).unwrap(), "take out")
        .await
        .expect("send take out");
    assert_eq!(
        failure(&mut session.connection, "take out a missing item").await,
        0x00
    );
    session
        .connection
        .send_packet(build_cash_shop_put_back(999_999, 5).unwrap(), "put back")
        .await
        .expect("send put back");
    assert_eq!(
        failure(&mut session.connection, "put back a missing item").await,
        0x00
    );

    // Empty slots, repeats, and items off sale are dropped.
    let mut wishes = [0; 10];
    wishes[..5].copy_from_slice(&[TICKET, 0, RETIRED_HAT, TICKET, TICKETS]);
    session
        .connection
        .send_packet(build_cash_shop_wish_list(&wishes).unwrap(), "wish list")
        .await
        .expect("send wish list");
    let updated = read_until(
        &mut session.connection,
        "wish list",
        SendOpcode::CashShopOperation,
    )
    .await;
    assert_eq!(updated.packet.bytes[2], 0x55);
    assert_eq!(wish_list_sns(&updated), [TICKET, TICKETS]);
    drop(session);

    let mut session = relogin(&config).await;
    assert_eq!(
        enter_cash_shop(&mut session.connection).await,
        [TICKET, TICKETS],
        "the wish list wasn't saved"
    );
}

#[tokio::test]
async fn gifts_need_a_birthday_code_another_account_and_cash() {
    let config = load_harness_config_or_fail().await;
    let receiver_config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    let mut receiver = login_to_world_session(&receiver_config)
        .await
        .expect("receiver login failed");
    enter_cash_shop(&mut session.connection).await;

    let cases = [
        (
            0,
            receiver_config.character_name.as_str(),
            0xC4,
            "no birthday code",
        ),
        (19830113, "NoSuchName", 0xA9, "an unknown receiver"),
        (
            19830113,
            config.character_name.as_str(),
            0xA8,
            "the sender's account",
        ),
        // A new account has no NX.
        (
            19830113,
            receiver_config.character_name.as_str(),
            0xA5,
            "no NX",
        ),
    ];
    for (birthday, to, code, phase) in cases {
        session
            .connection
            .send_packet(
                build_cash_shop_gift(birthday, TICKET, to, "for you").unwrap(),
                "gift",
            )
            .await
            .expect("send gift");
        assert_eq!(
            failure(&mut session.connection, phase).await,
            code,
            "{phase}"
        );
    }

    // Nothing reached the receiver.
    receiver
        .connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    for _ in 0..2 {
        let list = read_until(
            &mut receiver.connection,
            "receiver lists",
            SendOpcode::CashShopOperation,
        )
        .await;
        if list.packet.bytes[2] == 0x4D {
            assert_eq!(list.packet.bytes[3..5], [0, 0], "unexpected gifts");
            return;
        }
    }
    panic!("no gift list");
}

/// The NX Credit in the balance (QUERY_CASH_RESULT) that follows a purchase.
async fn credit(connection: &mut MapleTestConnection, phase: &'static str) -> i32 {
    let balance = read_until(connection, phase, SendOpcode::QueryCashResult).await;
    Cursor::new(&balance.packet.bytes[2..])
        .read_int()
        .expect("NX Credit")
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

/// Etc > Wedding: the Cathedral wedding ticket (25,000 NX), and the
/// Event tab's copy of it, which is off sale.
const CATHEDRAL_TICKET: i32 = 50400000;
const OLD_CATHEDRAL_TICKET: i32 = 10100885;

#[tokio::test]
async fn wedding_tickets_are_on_sale() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    set_nx_credit(&config.username, 30_000)
        .await
        .expect("grant NX");
    enter_cash_shop(&mut session.connection).await;
    let connection = &mut session.connection;

    let buy = build_cash_shop_buy(1, OLD_CATHEDRAL_TICKET).unwrap();
    connection
        .send_packet(buy, "buy")
        .await
        .expect("send purchase");
    assert_eq!(
        buy_failure(connection, "buy an old ticket", OLD_CATHEDRAL_TICKET).await,
        0xC0
    );

    let buy = build_cash_shop_buy(1, CATHEDRAL_TICKET).unwrap();
    let bought = request(connection, buy, "buy a ticket").await;
    assert_eq!(bought[0], 0x57, "buy a ticket");
    // The item: cash id, account, character, then the item id.
    let item_id = Cursor::new(&bought[17..]).read_int().expect("item id");
    assert_eq!(item_id, 5251000);
    assert_eq!(credit(connection, "buy a ticket").await, 5_000);
}

#[tokio::test]
async fn nx_buys_items_packages_gifts_and_slots() {
    let config = load_harness_config_or_fail().await;
    let receiver_config = load_harness_config_or_fail().await;
    drop(
        login_to_world_session(&receiver_config)
            .await
            .expect("receiver login failed"),
    );
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    set_nx_credit(&config.username, 30_000)
        .await
        .expect("grant NX");
    enter_cash_shop(&mut session.connection).await;
    let connection = &mut session.connection;

    let bought = request(connection, build_cash_shop_buy(1, TICKET).unwrap(), "buy").await;
    assert_eq!(bought[0], 0x57, "buy an item");
    assert_eq!(credit(connection, "buy an item").await, 29_000);

    let package = build_cash_shop_buy_package(1, PACKAGE).unwrap();
    let bought = request(connection, package, "buy a package").await;
    assert_eq!(bought[..2], [0x89, 3], "a package's three items");
    assert_eq!(credit(connection, "buy a package").await, 12_100);

    let gift = build_cash_shop_gift(19830113, TICKET, &receiver_config.character_name, "enjoy");
    let sent = request(connection, gift.unwrap(), "gift").await;
    assert_eq!(sent[0], 0x5E, "send a gift");
    assert_eq!(credit(connection, "gift").await, 11_100);

    // The slot buttons add four slots for 4,000 NX.
    let expand = build_cash_shop_expand(1, Some(3)).unwrap();
    let expanded = request(connection, expand, "expand the Set-up bag").await;
    assert_eq!(expanded, [0x60, 3, 28, 0]);
    assert_eq!(credit(connection, "expand the Set-up bag").await, 7_100);

    let expand = build_cash_shop_expand(1, None).unwrap();
    let expanded = request(connection, expand, "expand the storage").await;
    assert_eq!(expanded, [0x62, 8, 0]);
    assert_eq!(credit(connection, "expand the storage").await, 3_100);

    let expand = build_cash_shop_expand(1, Some(3)).unwrap();
    let failed = request(connection, expand, "expand without NX").await;
    assert_eq!(failed, [0x5C, 0xA5]);

    // NX charged outside the game shows on Check.
    set_nx_credit(&config.username, 9_000)
        .await
        .expect("charge NX");
    connection
        .send_packet(build_check_cash(), "check")
        .await
        .expect("send check");
    assert_eq!(credit(connection, "check").await, 9_000);
    drop(session);

    // The bag keeps its slots, and the Cash Inventory its four items and the
    // storage's.
    let mut session = relogin(&config).await;
    assert_eq!(session.slot_limits, [24, 24, 28, 24, 96]);
    let connection = &mut session.connection;
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    let inventory = read_until(connection, "cash inventory", SendOpcode::CashShopOperation).await;
    let inventory = &inventory.packet.bytes[2..];
    assert_eq!(inventory[..3], [0x4B, 4, 0], "Cash Inventory");
    assert_eq!(
        inventory[inventory.len() - 4..][..2],
        [8, 0],
        "storage slots"
    );

    // The gift waits for the receiver, and both commodities are now among the
    // Best Items: the last of SET_CASH_SHOP is 8 tabs of five for each
    // gender, then 11 more bytes.
    let mut receiver = relogin(&receiver_config).await;
    let connection = &mut receiver.connection;
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    let shop = read_until(connection, "receiver shop", SendOpcode::SetCashShop).await;
    let bytes = &shop.packet.bytes;
    let mut best = Cursor::new(&bytes[bytes.len() - 11 - 8 * 2 * 5 * 12..]);
    let men: Vec<i32> = (0..5)
        .map(|_| {
            let tab = best.read_int().unwrap();
            let gender = best.read_int().unwrap();
            assert_eq!((tab, gender), (1, 0));
            best.read_int().unwrap()
        })
        .collect();
    assert!(
        men.contains(&TICKET) && men.contains(&PACKAGE),
        "Best Items {men:?}"
    );
    // The gift is already in the Cash Inventory, and then announced.
    let mut in_inventory = false;
    for _ in 0..2 {
        let list = read_until(connection, "receiver lists", SendOpcode::CashShopOperation).await;
        let list = &list.packet.bytes[2..];
        if list[0] == 0x4B {
            assert_eq!(list[1..3], [1, 0], "the gift in the Cash Inventory");
            in_inventory = true;
        }
        if list[0] == 0x4D {
            assert!(in_inventory, "the gift isn't in the Cash Inventory");
            assert_eq!(list[1..3], [1, 0], "one gift");
            let item_id = Cursor::new(&list[11..]).read_int().unwrap();
            assert_eq!(item_id, 5220000);
            let from = &list[15..15 + config.character_name.len()];
            assert_eq!(from, config.character_name.as_bytes());
            return;
        }
    }
    panic!("no gift list");
}

/// The Golden Bell, sold for 1 meso, and a commodity of the same range off
/// sale (report b094).
const GOLDEN_BELL: i32 = 80000001;
const GOLDEN_BELL_ITEM: i32 = 4031191;
const MESOS_OFF_SALE: i32 = 80000004;

#[tokio::test]
async fn quest_items_are_bought_with_mesos() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    let character_id = session.character_id;
    enter_cash_shop(&mut session.connection).await;
    let connection = &mut session.connection;

    // A new character has no mesos; NX doesn't buy the bell either.
    let buy = build_cash_shop_buy_with_mesos(GOLDEN_BELL).unwrap();
    connection.send_packet(buy, "buy").await.expect("send buy");
    assert_eq!(failure(connection, "buy without mesos").await, 0xC2);
    let buy = build_cash_shop_buy_with_mesos(MESOS_OFF_SALE).unwrap();
    connection.send_packet(buy, "buy").await.expect("send buy");
    assert_eq!(failure(connection, "buy one off sale").await, 0xC0);
    let buy = build_cash_shop_buy(1, GOLDEN_BELL).unwrap();
    connection.send_packet(buy, "buy").await.expect("send buy");
    assert_eq!(
        buy_failure(connection, "buy with NX", GOLDEN_BELL).await,
        0x00
    );
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(character_id, 10).await.expect("give mesos");
    let mut session = relogin(&config).await;
    enter_cash_shop(&mut session.connection).await;
    let connection = &mut session.connection;
    let buy = build_cash_shop_buy_with_mesos(GOLDEN_BELL).unwrap();
    connection.send_packet(buy, "buy").await.expect("send buy");

    // The mesos, the bag, where the bell went, then the balances.
    let stat = read_until(connection, "buy", SendOpcode::StatChange).await;
    let meso = Cursor::new(&stat.packet.bytes[stat.packet.bytes.len() - 4..])
        .read_int()
        .expect("meso");
    assert_eq!(meso, 9);
    let added = read_until(connection, "buy", SendOpcode::InventoryOperation).await;
    // The tick byte, one change: add to Etc (4).
    assert_eq!(added.packet.bytes[2..6], [1, 1, 0, 4]);
    let slot = i16::from_le_bytes([added.packet.bytes[6], added.packet.bytes[7]]);
    let bought = read_until(connection, "buy", SendOpcode::CashShopOperation).await;
    let mut expected = vec![0x8D, 1, 0, 0, 0, 1, 0];
    expected.extend(slot.to_le_bytes());
    expected.extend(GOLDEN_BELL_ITEM.to_le_bytes());
    assert_eq!(bought.packet.bytes[2..], expected);
    read_until(connection, "buy", SendOpcode::QueryCashResult).await;
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    item_owner_and_hours(character_id, GOLDEN_BELL_ITEM)
        .await
        .expect("the bell wasn't saved");
}
