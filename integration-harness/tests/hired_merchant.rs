//! Hired merchants (ENTRUSTED_SHOP_REQUEST, then PLAYER_INTERACTION mini
//! room type 5): a player over level 15 with a permit sets a merchant up
//! on a Free Market map, lists items and opens it; it stays and sells
//! after they leave. The owner comes back for maintenance, takes the
//! mesos and closes it, and gets the unsold items back.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_stack, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PLAIN_MAP: i32 = 40_000;
const MARKET: i32 = 910_000_001;
const OTHER_MARKET: i32 = 910_000_002;
const PERMIT: i32 = 5030000;
const SNAIL_SHELL: i32 = 4000000;
const ETC: u8 = 4;
const CASH: u8 = 5;

/// ENTRUSTED_SHOP_CHECK_RESULT codes.
const OK: u8 = 7;
const OPEN_ELSEWHERE: u8 = 8;
const UNAVAILABLE: u8 = 0xb;

fn interaction(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PlayerInteraction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

/// The permit's use, as the client sends it before asking for a title.
fn check() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::EntrustedShopRequest as i16)
        .unwrap();
    p.write_byte(0).unwrap();
    p.write_long(0).unwrap();
    p
}

fn create(title: &str) -> Packet {
    let mut p = interaction(0);
    p.write_byte(5).unwrap();
    p.write_str_with_length(title).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(1).unwrap();
    p.write_int(PERMIT).unwrap();
    p
}

fn put_item(kind: u8, slot: i16, bundles: i16, per_bundle: i16, price: i32) -> Packet {
    let mut p = interaction(0x21);
    p.write_byte(kind).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(bundles).unwrap();
    p.write_short(per_bundle).unwrap();
    p.write_int(price).unwrap();
    p
}

/// The client sends its blacklist, empty here, and then opens.
async fn open(c: &mut MapleTestConnection) {
    let mut p = interaction(0x1e);
    p.write_short(0).unwrap();
    send(c, p).await;
    let mut p = interaction(0xb);
    p.write_byte(1).unwrap();
    send(c, p).await;
}

fn visit(room: i32) -> Packet {
    let mut p = interaction(4);
    p.write_int(room).unwrap();
    p
}

fn buy(index: u8, bundles: i16) -> Packet {
    let mut p = interaction(0x22);
    p.write_byte(index).unwrap();
    p.write_short(bundles).unwrap();
    p.write_int(0).unwrap();
    p
}

/// The first row of each item list: (bundles left, per bundle, price),
/// after the merchant's mesos.
fn first_rows(t: &Trace) -> Vec<(i32, i16, i16, i32)> {
    t.rooms
        .iter()
        .filter(|b| b[0] == 0x19 && b[5] > 0)
        .map(|b| {
            let mut r = &b[1..];
            let meso = r.read_int().unwrap();
            r.read_byte().unwrap();
            (
                meso,
                r.read_short().unwrap(),
                r.read_short().unwrap(),
                r.read_int().unwrap(),
            )
        })
        .collect()
}

/// The room of `owner`'s merchant, and its title, once it shows.
async fn merchant(c: &mut MapleTestConnection, t: &mut Trace, owner: i32) -> (i32, String) {
    let spawned = |(op, b): &(i16, Vec<u8>)| {
        *op == SendOpcode::SpawnHiredMerchant as i16 && (&b[..4]).read_int().unwrap() == owner
    };
    until(c, t, |t| t.merchants.iter().any(spawned)).await;
    let (_, body) = t.merchants.iter().rfind(|m| spawned(m)).unwrap();
    let mut r = &body[4..];
    assert_eq!(r.read_int().unwrap(), PERMIT);
    r.read_short().unwrap();
    r.read_short().unwrap();
    r.read_short().unwrap();
    r.read_str_with_length().unwrap();
    assert_eq!(r.read_byte().unwrap(), 5, "a merchant's room");
    let room = r.read_int().unwrap();
    (room, r.read_str_with_length().unwrap())
}

/// Read into `t` until `owner`'s merchant goes.
async fn merchant_gone(c: &mut MapleTestConnection, t: &mut Trace, owner: i32) {
    let gone = (
        SendOpcode::DestroyHiredMerchant as i16,
        owner.to_le_bytes().to_vec(),
    );
    until(c, t, |t| t.merchants.contains(&gone)).await;
}

/// A level 20 character with a permit and twenty snail shells.
async fn new_merchant(config: &HarnessConfig) -> i32 {
    let id = new_character(config).await;
    set_job(id, 0, 20, 50).await.expect("level the owner up");
    grant_item(id, CASH.into(), 1, PERMIT)
        .await
        .expect("give the owner a permit");
    grant_stack(id, ETC.into(), 1, SNAIL_SHELL, 20)
        .await
        .expect("give the owner snail shells");
    id
}

async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    id
}

async fn meet(a: &mut WorldSession, b: &mut WorldSession, map: i32) {
    let (mut at, mut bt) = (Trace::default(), Trace::default());
    assert_eq!(warp(&mut a.connection, map, "", &mut at).await, map);
    assert_eq!(warp(&mut b.connection, map, "", &mut bt).await, map);
    read_for(&mut a.connection, &mut at, 300).await;
    read_for(&mut b.connection, &mut bt, 300).await;
}

/// Read into `t` until it has a room packet starting with `head`.
async fn more(c: &mut MapleTestConnection, t: &mut Trace, head: &[u8]) {
    until(c, t, |t| t.rooms.iter().any(|b| b.starts_with(head))).await;
}

/// Wait until a room packet starting with `head` arrives.
async fn room_packet(c: &mut MapleTestConnection, head: &[u8]) -> Trace {
    let mut t = Trace::default();
    more(c, &mut t, head).await;
    t
}

#[tokio::test]
async fn a_merchant_sells_while_its_owner_is_away_and_closes_into_their_bags() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_merchant(&a_config).await;
    let b_id = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(a_id, 1_000).await.expect("give A mesos");
    set_meso(b_id, 300_000).await.expect("give B mesos");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;

    // Only the Free Market takes merchants.
    let mut at = Trace::default();
    assert_eq!(
        warp(&mut a.connection, PLAIN_MAP, "", &mut at).await,
        PLAIN_MAP
    );
    let t = step(&mut a.connection, "plain", check()).await;
    assert_eq!(t.merchant_checks, [vec![UNAVAILABLE]]);

    meet(&mut a, &mut b, MARKET).await;
    let t = step(&mut a.connection, "check", check()).await;
    assert_eq!(t.merchant_checks, [vec![OK]]);
    let t = step(&mut a.connection, "create", create("Shells")).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 5, 4, 0]);

    // Two bundles of five shells at 100,000 each.
    let t = step(&mut a.connection, "shells", put_item(ETC, 1, 2, 5, 100_000)).await;
    assert_eq!(t.quantities, [(ETC, 1, 10)]);
    assert_eq!(first_rows(&t), [(0, 2, 5, 100_000)]);

    // Opened, it shows to B; A can't hire a second.
    open(&mut a.connection).await;
    let (room, title) = merchant(&mut b.connection, &mut Trace::default(), a_id).await;
    assert_eq!(title, "Shells");
    let t = step(&mut a.connection, "again", check()).await;
    let mut elsewhere = vec![OPEN_ELSEWHERE];
    elsewhere.extend(MARKET.to_le_bytes());
    elsewhere.push(0);
    assert_eq!(t.merchant_checks, [elsewhere]);

    // B comes in and buys a bundle.
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 5, 4, 1]);
    let mut t = step(&mut b.connection, "buy", buy(0, 1)).await;
    assert_eq!(t.stat(stat::MESO), Some(200_000));
    assert!(t
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (ETC, SNAIL_SHELL)));
    until(&mut b.connection, &mut t, |t| !first_rows(t).is_empty()).await;
    assert_eq!(first_rows(&t), [(99_600, 1, 5, 100_000)]);

    // A comes back for maintenance, which turns B out, takes the mesos
    // and closes the merchant, getting the last bundle back.
    send(&mut a.connection, visit(room)).await;
    room_packet(&mut b.connection, &[0xa, 1, 0x11]).await;
    let t = room_packet(&mut a.connection, &[5, 5, 4, 0]).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    let mut t = step(&mut a.connection, "meso", interaction(0x2b)).await;
    more(&mut a.connection, &mut t, &[0x2c]).await;
    until(&mut a.connection, &mut t, |t| t.stat(stat::MESO).is_some()).await;
    assert_eq!(t.stat(stat::MESO), Some(1_000 + 99_600));
    let t = step(&mut a.connection, "close", interaction(0x29)).await;
    assert!(t.rooms.contains(&vec![0x2a, 0]), "{:?}", t.rooms);
    assert_eq!(t.quantities, [(ETC, 1, 15)]);
    merchant_gone(&mut b.connection, &mut Trace::default(), a_id).await;
    let t = step(&mut b.connection, "closed", visit(room)).await;
    assert_eq!(t.rooms, [vec![5, 0, 1]]);
    drop(a);
    drop(b);

    // The permit stays for the next merchant.
    let a = relogin(&a_config).await;
    assert_eq!(a.meso, 1_000 + 99_600);
    assert_eq!(a.items.get(&SNAIL_SHELL), Some(&15));
    assert_eq!(a.items.get(&PERMIT), Some(&1));
    let b = relogin(&b_config).await;
    assert_eq!(b.meso, 200_000);
    assert_eq!(b.items.get(&SNAIL_SHELL), Some(&5));
}

#[tokio::test]
async fn a_merchant_stays_open_when_its_owner_leaves_the_room_or_the_game() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_merchant(&a_config).await;
    let b_id = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    // B has the permit and the shells, but is too young.
    grant_item(b_id, CASH.into(), 1, PERMIT)
        .await
        .expect("give B a permit");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    meet(&mut a, &mut b, OTHER_MARKET).await;
    let t = step(&mut b.connection, "young", check()).await;
    assert_eq!(t.merchant_checks, [vec![UNAVAILABLE]]);

    // Leaving the room once something is listed opens the merchant.
    step(&mut a.connection, "create", create("Leftovers")).await;
    step(&mut a.connection, "shells", put_item(ETC, 1, 4, 5, 10)).await;
    send(&mut a.connection, interaction(0xa)).await;
    let mut bt = Trace::default();
    let (room, _) = merchant(&mut b.connection, &mut bt, a_id).await;

    // It is still there after A logs out, and when A comes back.
    drop(a);
    read_for(&mut b.connection, &mut bt, 1_000).await;
    assert!(!bt
        .merchants
        .iter()
        .any(|(op, _)| *op == SendOpcode::DestroyHiredMerchant as i16));
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 5, 4, 1]);

    let mut a = relogin(&a_config).await;
    assert_eq!(a.items.get(&SNAIL_SHELL), None);
    let mut at = Trace::default();
    warp(&mut a.connection, MARKET, "", &mut at).await;
    warp(&mut a.connection, OTHER_MARKET, "", &mut at).await;
    let (again, title) = merchant(&mut a.connection, &mut at, a_id).await;
    assert_eq!((again, title.as_str()), (room, "Leftovers"));
}
