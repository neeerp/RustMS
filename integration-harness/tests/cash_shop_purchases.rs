use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::packets::{
    build_cash_shop_buy, build_cash_shop_gift, build_cash_shop_put_back, build_cash_shop_take_out,
    build_cash_shop_wish_list, build_enter_cash_shop, opcode_name,
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
        failure(&mut session.connection, "buy without NX").await,
        0xA5
    );

    session
        .connection
        .send_packet(build_cash_shop_buy(1, RETIRED_HAT).unwrap(), "buy")
        .await
        .expect("send purchase");
    assert_eq!(
        failure(&mut session.connection, "buy a retired item").await,
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
