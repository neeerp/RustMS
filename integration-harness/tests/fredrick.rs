//! Fredrick's Store Bank (STORE_BANK, STORE_BANK_REQUEST,
//! STORE_BANK_MESSAGE): he keeps what a hired merchant held when it closed
//! without its owner, and hands it all back for a storing fee of 1% a day
//! of its mesos and the listed price of its items.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{has_merchant, place_merchant, set_meso, MerchantRow};
use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, until, warp, StorageResult, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;

const FM_ENTRANCE: i32 = 910_000_000;
const MARKET: i32 = 910_000_002;
const FREDRICK: i32 = 9030000;
const RED_POTION: i32 = 2000000;
const SNAIL_SHELL: i32 = 4000000;
const USE: u8 = 2;
const ETC: u8 = 4;

const OPEN: u8 = 0x23;
const FEE: u8 = 0x24;
const ELSEWHERE: u8 = 0x25;

const RETRIEVED: u8 = 0x1e;
const TOO_MUCH_MESO: u8 = 0x1f;
const NO_FEE: u8 = 0x21;

fn request(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::StoreBankRequest as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn get_all() -> Packet {
    request(0x1a)
}

fn confirm() -> Packet {
    request(0x1b)
}

async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    id
}

/// Click Fredrick and read his answer: its mode and the rest.
async fn talk_to_fredrick(c: &mut MapleTestConnection) -> (u8, Vec<u8>) {
    let mut t = Trace::default();
    assert_eq!(warp(c, FM_ENTRANCE, "", &mut t).await, FM_ENTRANCE);
    let mut t = Trace::default();
    send(c, talk(FM_ENTRANCE, FREDRICK)).await;
    until(c, &mut t, |t| !t.store_bank.is_empty()).await;
    t.store_bank.remove(0)
}

/// Where Fredrick says the merchant is open: the map and the channel.
fn elsewhere((mode, body): &(u8, Vec<u8>)) -> (i32, i32, u8) {
    assert_eq!(*mode, ELSEWHERE);
    let mut r = &body[..];
    (
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_byte().unwrap(),
    )
}

async fn answer(c: &mut MapleTestConnection, p: Packet) -> Trace {
    let mut t = Trace::default();
    send(c, p).await;
    until(c, &mut t, |t| {
        !t.store_bank.is_empty() || !t.store_bank_messages.is_empty()
    })
    .await;
    t
}

#[tokio::test]
async fn fredrick_keeps_nothing_or_says_where_the_merchant_is_open() {
    let config = load_harness_config_or_fail().await;
    let id = new_character(&config).await;

    let mut s = relogin(&config).await;
    assert_eq!(
        elsewhere(&talk_to_fredrick(&mut s.connection).await),
        (FREDRICK, 999_999_999, 0xff),
        "nothing kept"
    );

    place_merchant(id, 1, MARKET, 1, 0, 500, vec![])
        .await
        .expect("an open merchant");
    assert_eq!(
        elsewhere(&talk_to_fredrick(&mut s.connection).await),
        (FREDRICK, MARKET, 1),
        "open in channel 2"
    );
}

#[tokio::test]
async fn fredrick_hands_back_a_closed_merchant_for_a_fee() {
    let config = load_harness_config_or_fail().await;
    let id = new_character(&config).await;
    set_meso(id, 1_000).await.expect("pocket money");
    // Worth 100,000 + 2 x 1,000 + 50,000 = 152,000; three days at 1%.
    place_merchant(
        id,
        0,
        MARKET,
        2,
        3,
        100_000,
        vec![
            MerchantRow {
                item_id: SNAIL_SHELL,
                bundles: 2,
                quantity: 5,
                price: 1_000,
            },
            MerchantRow {
                item_id: RED_POTION,
                bundles: 1,
                quantity: 1,
                price: 50_000,
            },
        ],
    )
    .await
    .expect("a closed merchant");

    let mut s = relogin(&config).await;
    let c = &mut s.connection;
    let (mode, body) = talk_to_fredrick(c).await;
    assert_eq!(mode, OPEN);
    let kept = StorageResult::store_bank(&body);
    assert_eq!(kept.npc, Some(FREDRICK));
    assert_eq!(kept.meso, Some(100_000));
    assert_eq!(kept.bags[&ETC], vec![(SNAIL_SHELL, 10)]);
    assert_eq!(kept.bags[&USE], vec![(RED_POTION, 1)]);

    let t = answer(c, get_all()).await;
    let (mode, body) = &t.store_bank[0];
    assert_eq!(*mode, FEE);
    let mut r = &body[..];
    assert_eq!((r.read_int().unwrap(), r.read_int().unwrap()), (3, 4_560));

    let mut t = answer(c, confirm()).await;
    assert_eq!(t.store_bank_messages, vec![RETRIEVED]);
    until(c, &mut t, |t| t.stat(stat::MESO).is_some()).await;
    assert_eq!(t.stat(stat::MESO), Some(1_000 + 100_000 - 4_560));
    assert!(!has_merchant(id).await.unwrap(), "Fredrick keeps nothing");

    assert_eq!(
        elsewhere(&talk_to_fredrick(c).await),
        (FREDRICK, 999_999_999, 0xff)
    );
}

#[tokio::test]
async fn fredrick_keeps_it_all_when_the_owner_cant_pay_or_carry_the_mesos() {
    let config = load_harness_config_or_fail().await;
    let id = new_character(&config).await;
    set_meso(id, 500_000).await.expect("pocket money");
    // 200 days: the fee is all of it, 1,000,000.
    let shell = || MerchantRow {
        item_id: SNAIL_SHELL,
        bundles: 1,
        quantity: 1,
        price: 1_000_000,
    };
    place_merchant(id, 0, MARKET, 2, 200, 0, vec![shell()])
        .await
        .expect("a long closed merchant");

    let mut s = relogin(&config).await;
    let c = &mut s.connection;
    assert_eq!(talk_to_fredrick(c).await.0, OPEN);
    let t = answer(c, get_all()).await;
    let mut r = &t.store_bank[0].1[..];
    assert_eq!(
        (r.read_int().unwrap(), r.read_int().unwrap()),
        (200, 1_000_000)
    );
    let t = answer(c, confirm()).await;
    assert_eq!(t.store_bank_messages, vec![NO_FEE]);
    assert!(has_merchant(id).await.unwrap(), "Fredrick keeps it");
    drop(s);

    // Fresh, so free, but more mesos than a purse holds.
    set_meso(id, i32::MAX - 10).await.expect("a full purse");
    place_merchant(id, 0, MARKET, 2, 0, 100, vec![shell()])
        .await
        .expect("a fresh merchant");
    let mut s = relogin(&config).await;
    let c = &mut s.connection;
    assert_eq!(talk_to_fredrick(c).await.0, OPEN);
    let t = answer(c, get_all()).await;
    let mut r = &t.store_bank[0].1[..];
    assert_eq!((r.read_int().unwrap(), r.read_int().unwrap()), (0, 0));
    let t = answer(c, confirm()).await;
    assert_eq!(t.store_bank_messages, vec![TOO_MUCH_MESO]);
    assert!(has_merchant(id).await.unwrap(), "Fredrick still keeps it");
}
