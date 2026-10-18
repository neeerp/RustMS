//! Personal shops (PLAYER_INTERACTION, mini room type 4): a player with a
//! store permit opens a shop on a Free Market map, lists items in bundles,
//! and sells them to visitors, less tax. The owner can turn a visitor out;
//! whatever is unsold goes back to the owner when the shop closes, or when
//! they log out.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PLAIN_MAP: i32 = 40_000;
const MARKET: i32 = 910_000_001;
const OTHER_MARKET: i32 = 910_000_002;
const STORE_PERMIT: i32 = 5140000;
const RED_POTION: i32 = 2000000;
const SNAIL_SHELL: i32 = 4000000;
const USE: u8 = 2;
const ETC: u8 = 4;
const CASH: u8 = 5;

fn interaction(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PlayerInteraction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn create(title: &str) -> Packet {
    let mut p = interaction(0);
    p.write_byte(4).unwrap();
    p.write_str_with_length(title).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(1).unwrap();
    p.write_int(STORE_PERMIT).unwrap();
    p
}

fn put_item(kind: u8, slot: i16, bundles: i16, per_bundle: i16, price: i32) -> Packet {
    let mut p = interaction(0x16);
    p.write_byte(kind).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(bundles).unwrap();
    p.write_short(per_bundle).unwrap();
    p.write_int(price).unwrap();
    p
}

fn take_back(index: i16) -> Packet {
    let mut p = interaction(0x1b);
    p.write_short(index).unwrap();
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
    let mut p = interaction(0x17);
    p.write_byte(index).unwrap();
    p.write_short(bundles).unwrap();
    p.write_int(0).unwrap();
    p
}

fn ban(slot: u8, name: &str) -> Packet {
    let mut p = interaction(0x1c);
    p.write_byte(slot).unwrap();
    p.write_str_with_length(name).unwrap();
    p
}

fn leave() -> Packet {
    interaction(0xa)
}

/// The first listing of each item list: (bundles left, per bundle,
/// price).
fn first_listings(t: &Trace) -> Vec<(i16, i16, i32)> {
    t.rooms
        .iter()
        .filter(|b| b[0] == 0x19 && b[1] > 0)
        .map(|b| {
            let mut r = &b[2..];
            (
                r.read_short().unwrap(),
                r.read_short().unwrap(),
                r.read_int().unwrap(),
            )
        })
        .collect()
}

/// The number of listings in each item list.
fn listing_counts(t: &Trace) -> Vec<u8> {
    t.rooms
        .iter()
        .filter(|b| b[0] == 0x19)
        .map(|b| b[1])
        .collect()
}

/// The shop room id and skin in the balloon over `owner`, once it shows.
async fn balloon(c: &mut MapleTestConnection, owner: i32) -> (i32, u8) {
    let mut t = Trace::default();
    let over = |b: &Vec<u8>| (&b[..4]).read_int().unwrap() == owner && b[4] == 4;
    until(c, &mut t, |t| t.char_boxes.iter().any(over)).await;
    let body = t.char_boxes.iter().rfind(|b| over(b)).unwrap();
    let mut r = &body[5..];
    let room = r.read_int().unwrap();
    r.read_str_with_length().unwrap();
    assert_eq!(r.read_byte().unwrap(), 0, "a password");
    (room, r.read_byte().unwrap())
}

/// Read into `t` until the balloon over `owner` goes.
async fn balloon_gone(c: &mut MapleTestConnection, t: &mut Trace, owner: i32) {
    let mut gone = owner.to_le_bytes().to_vec();
    gone.push(0);
    until(c, t, |t| t.char_boxes.contains(&gone)).await;
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
async fn a_shop_sells_its_stock_less_tax_and_closes_when_sold_out() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_character(&a_config).await;
    let b_id = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(a_id, 1_000).await.expect("give A mesos");
    grant_item(a_id, CASH.into(), 1, STORE_PERMIT)
        .await
        .expect("give A a permit");
    grant_stack(a_id, ETC.into(), 1, SNAIL_SHELL, 20)
        .await
        .expect("give A snail shells");
    grant_stack(a_id, USE.into(), 1, RED_POTION, 3)
        .await
        .expect("give A red potions");
    set_meso(b_id, 300_000).await.expect("give B mesos");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;

    // Only the Free Market allows shops.
    let mut at = Trace::default();
    assert_eq!(
        warp(&mut a.connection, PLAIN_MAP, "", &mut at).await,
        PLAIN_MAP
    );
    let t = step(&mut a.connection, "plain", create("Shells")).await;
    assert_eq!(t.rooms, [vec![5, 0, 0xf]]);

    meet(&mut a, &mut b, MARKET).await;
    let t = step(&mut a.connection, "create", create("Shells")).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 4, 4, 0]);

    // Two bundles of five shells at 100,000 each, and the potions, which
    // A takes back before opening.
    let t = step(&mut a.connection, "shells", put_item(ETC, 1, 2, 5, 100_000)).await;
    assert_eq!(t.quantities, [(ETC, 1, 10)]);
    assert_eq!(first_listings(&t), [(2, 5, 100_000)]);
    let t = step(&mut a.connection, "potions", put_item(USE, 1, 1, 3, 50)).await;
    assert_eq!(t.removed, [(USE, 1)]);
    assert_eq!(listing_counts(&t), [2]);
    let t = step(&mut a.connection, "take-back", take_back(1)).await;
    assert!(t
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (USE, RED_POTION)));
    assert_eq!(listing_counts(&t), [1]);

    // Opening puts a balloon over A, which B uses to come in.
    open(&mut a.connection).await;
    let (room, skin) = balloon(&mut b.connection, a_id).await;
    assert_eq!(skin, 0);
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms.len(), 1, "{:?}", t.rooms);
    assert_eq!(t.rooms[0][..4], [5, 4, 4, 1]);
    room_packet(&mut a.connection, &[4, 1]).await;

    // B buys a bundle; A gets the price less 0.4% tax. The new list comes
    // from the owner, maybe after B's own answer.
    let mut t = step(&mut b.connection, "buy", buy(0, 1)).await;
    assert_eq!(t.stat(stat::MESO), Some(200_000));
    assert!(t
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (ETC, SNAIL_SHELL)));
    more(&mut b.connection, &mut t, &[0x19]).await;
    assert_eq!(first_listings(&t), [(1, 5, 100_000)]);
    let mut at = room_packet(&mut a.connection, &[0x1a, 0, 1, 0]).await;
    until(&mut a.connection, &mut at, |t| t.stat(stat::MESO).is_some()).await;
    assert_eq!(at.stat(stat::MESO), Some(1_000 + 99_600));

    // There is only one bundle left.
    let t = step(&mut b.connection, "too-many", buy(0, 2)).await;
    assert_eq!(t.rooms, [vec![0x18, 1]]);
    assert_eq!(t.stat(stat::MESO), None);

    // The last bundle sells out the shop, which closes for both.
    let mut t = step(&mut b.connection, "last", buy(0, 1)).await;
    assert_eq!(t.stat(stat::MESO), Some(100_000));
    more(&mut b.connection, &mut t, &[0xa, 1, 0xe]).await;
    let at = room_packet(&mut a.connection, &[0xa, 0, 0xe]).await;
    assert_eq!(at.stat(stat::MESO), Some(1_000 + 2 * 99_600));
    balloon_gone(&mut b.connection, &mut t, a_id).await;
    let t = step(&mut b.connection, "closed", visit(room)).await;
    assert_eq!(t.rooms, [vec![5, 0, 1]]);
    drop(a);
    drop(b);

    let a = relogin(&a_config).await;
    assert_eq!(a.meso, 1_000 + 2 * 99_600);
    assert_eq!(a.items.get(&SNAIL_SHELL), Some(&10));
    assert_eq!(a.items.get(&RED_POTION), Some(&3));
    let b = relogin(&b_config).await;
    assert_eq!(b.meso, 100_000);
    assert_eq!(b.items.get(&SNAIL_SHELL), Some(&10));
}

#[tokio::test]
async fn the_owner_bans_visitors_and_gets_unsold_stock_back() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_character(&a_config).await;
    let b_id = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(a_id, CASH.into(), 1, STORE_PERMIT)
        .await
        .expect("give A a permit");
    grant_stack(a_id, ETC.into(), 1, SNAIL_SHELL, 20)
        .await
        .expect("give A snail shells");
    set_meso(b_id, 1_000).await.expect("give B mesos");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;
    meet(&mut a, &mut b, OTHER_MARKET).await;

    step(&mut a.connection, "create", create("Shells")).await;
    let t = step(&mut a.connection, "shells", put_item(ETC, 1, 20, 1, 10)).await;
    assert_eq!(t.removed, [(ETC, 1)]);
    open(&mut a.connection).await;
    let (room, _) = balloon(&mut b.connection, a_id).await;
    step(&mut b.connection, "visit", visit(room)).await;
    room_packet(&mut a.connection, &[4, 1]).await;

    // Turned out, B may not come back while the shop is open.
    send(&mut a.connection, ban(1, &b.character_name)).await;
    room_packet(&mut b.connection, &[0xa, 1, 5]).await;
    room_packet(&mut a.connection, &[0xa, 1]).await;
    let t = step(&mut b.connection, "banned", visit(room)).await;
    assert_eq!(t.rooms, [vec![5, 0, 0x11]]);

    // Closing the shop gives A the shells back.
    let t = step(&mut a.connection, "close", leave()).await;
    assert!(t
        .added
        .iter()
        .any(|&(kind, _, id)| (kind, id) == (ETC, SNAIL_SHELL)));
    balloon_gone(&mut b.connection, &mut Trace::default(), a_id).await;

    // So does logging out with the shop open.
    step(&mut a.connection, "create", create("Again")).await;
    step(&mut a.connection, "shells", put_item(ETC, 1, 4, 5, 10)).await;
    open(&mut a.connection).await;
    balloon(&mut b.connection, a_id).await;
    drop(a);
    balloon_gone(&mut b.connection, &mut Trace::default(), a_id).await;
    drop(b);

    let a = relogin(&a_config).await;
    assert_eq!(a.items.get(&SNAIL_SHELL), Some(&20));
}
