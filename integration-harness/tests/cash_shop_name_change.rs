//! Name Change (5400000): the Cash Shop's rules check and name check, the
//! purchase from NX Prepaid, cancelling with the receipt, and the new name
//! taking effect at logout, with a month before the next (report c060).

use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::database::{rename_character, set_job, set_nx_prepaid};
use integration_harness::packets::{
    build_cash_shop_take_out, build_check_char_name, build_enter_cash_shop, build_leave_cash_shop,
    build_player_logged_in, decode_channel_change, opcode_name,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin, HarnessConfig};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{sleep, timeout, Duration};

/// Commodity 50600000: Name Change, 10,000 NX.
const NAME_CHANGE_SN: i32 = 50600000;
const NAME_CHANGE: i32 = 5400000;
const PRICE: i32 = 10_000;
/// The harness accounts' birthday code.
const BIRTHDAY: i32 = 19830113;

async fn read_until(
    connection: &mut MapleTestConnection,
    phase: &'static str,
    expected: SendOpcode,
) -> PacketEnvelope {
    let expected = expected as i16;
    for _ in 0..48 {
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

/// Enter the Cash Shop: how many items the Cash Inventory holds, and the
/// NX Prepaid.
async fn enter_cash_shop(connection: &mut MapleTestConnection) -> (i16, i32) {
    connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    let mut items = None;
    loop {
        let envelope = read_until_either(connection).await;
        let bytes = &envelope.packet.bytes;
        if envelope.opcode() == SendOpcode::QueryCashResult as i16 {
            let prepaid = Cursor::new(&bytes[10..]).read_int().expect("NX Prepaid");
            return (items.expect("no Cash Inventory"), prepaid);
        }
        if bytes[2] == 0x4B {
            items = Some(Cursor::new(&bytes[3..]).read_short().expect("count"));
        }
    }
}

/// The next CASHSHOP_OPERATION or QUERY_CASH_RESULT.
async fn read_until_either(connection: &mut MapleTestConnection) -> PacketEnvelope {
    for _ in 0..48 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet("enter"))
            .await
            .expect("enter cash shop: timed out")
            .expect("enter cash shop");
        if envelope.opcode() == SendOpcode::CashShopOperation as i16
            || envelope.opcode() == SendOpcode::QueryCashResult as i16
        {
            return envelope;
        }
    }
    panic!("enter cash shop: never received the balance");
}

/// The NX Prepaid in the next balance.
async fn prepaid(connection: &mut MapleTestConnection, phase: &'static str) -> i32 {
    let balance = read_until(connection, phase, SendOpcode::QueryCashResult).await;
    Cursor::new(&balance.packet.bytes[10..])
        .read_int()
        .expect("NX Prepaid")
}

/// Op 0x10, the Name Change button: the rules result (0 allowed, 1 a
/// request waiting, 3 changed within a month).
async fn rules(connection: &mut MapleTestConnection, character_id: i32) -> u8 {
    let mut packet = Packet::new_empty();
    packet
        .write_short(RecvOpcode::NameChangeRules as i16)
        .unwrap();
    packet.write_int(character_id).unwrap();
    packet.write_int(BIRTHDAY).unwrap();
    connection
        .send_packet(packet, "rules")
        .await
        .expect("send rules");
    let reply = read_until(connection, "rules", SendOpcode::NameChangeRulesResult).await;
    assert_eq!(reply.packet.bytes.len(), 11, "{{i32, u8, i32}}");
    reply.packet.bytes[6]
}

/// Op 0x15 in the Cash Shop: whether `name` is free for a Name Change.
async fn name_is_free(connection: &mut MapleTestConnection, name: &str) -> bool {
    let check = build_check_char_name(name).unwrap();
    connection
        .send_packet(check, "check name")
        .await
        .expect("send check");
    let reply = read_until(connection, "check name", SendOpcode::CheckNewNameResult).await;
    let mut reader = Cursor::new(&reply.packet.bytes[2..]);
    assert_eq!(reader.read_str_with_length().unwrap(), name);
    reader.read_byte().unwrap() == 0
}

/// CASHSHOP_OPERATION 0x2E; the CASHSHOP_OPERATION reply.
async fn buy(connection: &mut MapleTestConnection, old: &str, new: &str) -> Vec<u8> {
    let mut packet = Packet::new_empty();
    packet
        .write_short(RecvOpcode::CashShopOperation as i16)
        .unwrap();
    packet.write_byte(0x2E).unwrap();
    packet.write_int(NAME_CHANGE_SN).unwrap();
    packet.write_str_with_length(old).unwrap();
    packet.write_str_with_length(new).unwrap();
    connection
        .send_packet(packet, "buy")
        .await
        .expect("send buy");
    let reply = read_until(connection, "buy", SendOpcode::CashShopOperation).await;
    reply.packet.bytes[2..].to_vec()
}

#[tokio::test]
async fn name_change_is_bought_cancelled_and_made_at_logout() {
    let config = load_harness_config_or_fail().await;
    let other = load_harness_config_or_fail().await;
    drop(
        login_to_world_session(&other)
            .await
            .expect("the other character"),
    );
    let session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    let id = session.character_id;
    let old_name = session.character_name.clone();
    drop(session);
    sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.expect("level 10");
    set_nx_prepaid(&config.username, 2 * PRICE)
        .await
        .expect("NX Prepaid");

    let session = relogin(&config).await;
    let mut shop = session.connection;
    let connection = &mut shop;
    enter_cash_shop(connection).await;
    assert_eq!(rules(connection, id).await, 0, "allowed");

    let new_name = format!("rename{id}");
    assert!(name_is_free(connection, &new_name).await);
    assert!(!name_is_free(connection, &other.character_name).await);
    assert!(!name_is_free(connection, &old_name).await, "its own name");
    assert!(!name_is_free(connection, "bad name").await);

    let bought = buy(connection, &old_name, &new_name).await;
    assert_eq!(bought[0], 0x9E, "a Name Change receipt");
    let receipt = Cursor::new(&bought[1..]).read_long().unwrap();
    assert_eq!(
        Cursor::new(&bought[1 + 8 + 4 + 4..]).read_int().unwrap(),
        NAME_CHANGE
    );
    assert_eq!(prepaid(connection, "buy").await, PRICE);

    // One request at a time, and the name waits for it.
    assert_eq!(rules(connection, id).await, 1, "pending");
    assert!(!name_is_free(connection, &new_name).await);
    let again = buy(connection, &old_name, &format!("other{id}")).await;
    assert_eq!(again[0], 0x5C, "a second request");

    // Using the receipt cancels the request, with no refund.
    let take_out = build_cash_shop_take_out(receipt, 5, 1).unwrap();
    connection
        .send_packet(take_out, "take out")
        .await
        .expect("send take out");
    let taken = read_until(connection, "take out", SendOpcode::CashShopOperation).await;
    assert_ne!(taken.packet.bytes[2], 0x5C, "take out the receipt");
    connection
        .send_packet(build_leave_cash_shop(), "leave")
        .await
        .expect("send leave");
    // Leaving is a channel change, which keeps the request waiting.
    let redirect = read_until(connection, "leave", SendOpcode::ChangeChannel).await;
    let redirect = decode_channel_change(&redirect.packet).unwrap();
    let mut field = MapleTestConnection::connect(
        format!("{}:{}", redirect.ip, redirect.port)
            .parse()
            .expect("redirect address"),
        "return from the cash shop",
    )
    .await
    .expect("reconnect after the Cash Shop");
    let connection = &mut field;
    connection
        .send_packet(build_player_logged_in(id).unwrap(), "re-enter")
        .await
        .expect("send PLAYER_LOGGEDIN");
    read_until(connection, "re-enter", SendOpcode::SetField).await;
    let mut cancel = Packet::new_empty();
    cancel.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    cancel.write_short(1).unwrap();
    cancel.write_int(NAME_CHANGE).unwrap();
    cancel.write_byte(1).unwrap();
    cancel.write_int(0).unwrap();
    connection
        .send_packet(cancel, "cancel")
        .await
        .expect("send cancel");
    let cancelled = read_until(connection, "cancel", SendOpcode::CancelNameChangeResult).await;
    assert_eq!(cancelled.packet.bytes[2..], [0]);

    enter_cash_shop(connection).await;
    assert_eq!(rules(connection, id).await, 0, "allowed again");
    let bought = buy(connection, &old_name, &new_name).await;
    assert_eq!(bought[0], 0x9E, "buy again");
    assert_eq!(prepaid(connection, "buy again").await, 0);
    drop(shop);
    drop(field);
    // Let the logout finish first: a login missing the new name would make
    // a character with it.
    sleep(Duration::from_millis(1500)).await;

    // The name changes at logout; the old one is free and the next change
    // waits a month.
    let renamed = HarnessConfig {
        character_name: new_name.clone(),
        ..config.clone()
    };
    let mut session = relogin(&renamed).await;
    assert_eq!(session.character_id, id);
    assert_eq!(session.character_name, new_name);
    let connection = &mut session.connection;
    assert_eq!(
        enter_cash_shop(connection).await,
        (0, 0),
        "the receipt went"
    );
    assert_eq!(rules(connection, id).await, 3, "within a month");
    assert!(name_is_free(connection, &old_name).await);
}

#[tokio::test]
async fn a_name_taken_meanwhile_refunds_the_request() {
    let config = load_harness_config_or_fail().await;
    let other = load_harness_config_or_fail().await;
    let other_id = login_to_world_session(&other)
        .await
        .expect("the other character")
        .character_id;
    let session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    let id = session.character_id;
    let old_name = session.character_name.clone();
    drop(session);
    sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.expect("level 10");
    set_nx_prepaid(&config.username, PRICE)
        .await
        .expect("NX Prepaid");

    let mut session = relogin(&config).await;
    let connection = &mut session.connection;
    enter_cash_shop(connection).await;
    let new_name = format!("wanted{id}");
    let bought = buy(connection, &old_name, &new_name).await;
    assert_eq!(bought[0], 0x9E, "a Name Change receipt");
    assert_eq!(prepaid(connection, "buy").await, 0);
    // Another character gets the name before the request is made.
    rename_character(other_id, &new_name)
        .await
        .expect("take the name");
    drop(session);
    sleep(Duration::from_millis(1500)).await;

    let mut session = relogin(&config).await;
    assert_eq!(session.character_name, old_name);
    let connection = &mut session.connection;
    assert_eq!(enter_cash_shop(connection).await, (0, PRICE), "refunded");
    assert_eq!(rules(connection, id).await, 0, "no month to wait");
}
