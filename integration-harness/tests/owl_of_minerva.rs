//! The Owl of Minerva (2310000): in the Free Market it searches the
//! personal shops of every market room for an item, is used up only when
//! something turns up, and GO on a row takes the player into that shop.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PLAIN_MAP: i32 = 40_000;
const MARKET: i32 = 910_000_001;
const OTHER_MARKET: i32 = 910_000_002;
const STORE_PERMIT: i32 = 5140000;
const SNAIL_SHELL: i32 = 4000000;
const BLUE_SNAIL_SHELL: i32 = 4000001;
const OWL: i32 = 2310000;
const USE: u8 = 2;
const ETC: u8 = 4;
const CASH: u8 = 5;

fn interaction(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PlayerInteraction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

async fn open_shop(c: &mut MapleTestConnection) {
    let mut p = interaction(0);
    p.write_byte(4).unwrap();
    p.write_str_with_length("Shells").unwrap();
    p.write_byte(0).unwrap();
    p.write_short(1).unwrap();
    p.write_int(STORE_PERMIT).unwrap();
    step(c, "create", p).await;
    let mut p = interaction(0x16);
    p.write_byte(ETC).unwrap();
    p.write_short(1).unwrap();
    p.write_short(2).unwrap();
    p.write_short(5).unwrap();
    p.write_int(100_000).unwrap();
    step(c, "shells", p).await;
    let mut p = interaction(0x1e);
    p.write_short(0).unwrap();
    send(c, p).await;
    let mut p = interaction(0xb);
    p.write_byte(1).unwrap();
    step(c, "open", p).await;
}

fn owl_open() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::OwlOpen as i16).unwrap();
    p.write_byte(5).unwrap();
    p
}

fn use_owl(item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseOwl as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(OWL).unwrap();
    p.write_int(item_id).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(0).unwrap();
    p
}

fn go(room: i32, map: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::OwlGo as i16).unwrap();
    p.write_int(room).unwrap();
    p.write_int(map).unwrap();
    p
}

/// A search's answer: (item, rows as (owner, map, title, bundles, per
/// bundle, price, room, channel, item kind)).
#[allow(clippy::type_complexity)]
fn results(t: &Trace) -> (i32, Vec<(String, i32, String, i32, i32, i32, i32, u8, u8)>) {
    let bodies: Vec<_> = t
        .owl
        .iter()
        .filter(|(op, b)| *op == SendOpcode::OwlResult as i16 && b[0] == 6)
        .collect();
    assert_eq!(bodies.len(), 1, "{:?}", t.owl);
    let mut r = &bodies[0].1[1..];
    assert_eq!(r.read_int().unwrap(), 0);
    let item = r.read_int().unwrap();
    let count = r.read_int().unwrap();
    let rows = (0..count)
        .map(|_| {
            (
                r.read_str_with_length().unwrap(),
                r.read_int().unwrap(),
                r.read_str_with_length().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_int().unwrap(),
                r.read_byte().unwrap(),
                r.read_byte().unwrap(),
            )
        })
        .collect();
    assert!(r.is_empty(), "left over: {r:?}");
    (item, rows)
}

async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    id
}

#[tokio::test]
async fn the_owl_finds_a_shop_and_goes_there() {
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
    grant_stack(a_id, ETC.into(), 1, SNAIL_SHELL, 10)
        .await
        .expect("give A snail shells");
    grant_stack(b_id, USE.into(), 1, OWL, 2)
        .await
        .expect("give B two owls");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;

    let mut at = Trace::default();
    assert_eq!(warp(&mut a.connection, MARKET, "", &mut at).await, MARKET);
    open_shop(&mut a.connection).await;

    // Outside the Free Market the Owl only unlocks the client.
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, PLAIN_MAP, "", &mut bt).await,
        PLAIN_MAP
    );
    let t = step(&mut b.connection, "outside", use_owl(SNAIL_SHELL)).await;
    assert!(t.owl.is_empty(), "{:?}", t.owl);
    assert!(t.stat_updates.iter().any(|s| s.is_empty()));

    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, OTHER_MARKET, "", &mut bt).await,
        OTHER_MARKET
    );
    read_for(&mut b.connection, &mut bt, 300).await;

    // Nobody sells blue shells: no rows, and the Owl is kept.
    let t = step(&mut b.connection, "none", use_owl(BLUE_SNAIL_SHELL)).await;
    assert_eq!(results(&t), (BLUE_SNAIL_SHELL, vec![]));
    assert!(t.quantities.is_empty() && t.removed.is_empty());

    // A's shells turn up from the other room, and one Owl goes.
    let t = step(&mut b.connection, "shells", use_owl(SNAIL_SHELL)).await;
    let (item, rows) = results(&t);
    assert_eq!(item, SNAIL_SHELL);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let (owner, map, title, bundles, per_bundle, price, room, channel, kind) = rows[0].clone();
    assert_eq!(owner, a.character_name);
    assert_eq!(
        (
            map,
            title.as_str(),
            bundles,
            per_bundle,
            price,
            channel,
            kind
        ),
        (MARKET, "Shells", 2, 5, 100_000, 0, ETC)
    );
    assert_eq!(t.quantities, [(USE, 1, 1)]);

    // The window's list of most searched items has the shells now.
    let t = step(&mut b.connection, "popular", owl_open()).await;
    let popular = t
        .owl
        .iter()
        .find(|(op, body)| *op == SendOpcode::OwlResult as i16 && body[0] == 7)
        .expect("the popular list");
    let ids: Vec<i32> = popular.1[2..]
        .chunks(4)
        .map(|mut c| c.read_int().unwrap())
        .collect();
    assert_eq!(ids.len(), usize::from(popular.1[1]));
    assert!(ids.contains(&SNAIL_SHELL), "{ids:?}");

    // GO says go, takes B to A's room and into the shop.
    let mut t = step(&mut b.connection, "go", go(room, MARKET)).await;
    until(&mut b.connection, &mut t, |t| {
        t.rooms.iter().any(|r| r.starts_with(&[5, 4, 4, 1]))
    })
    .await;
    let go_results: Vec<_> = t
        .owl
        .iter()
        .filter(|(op, _)| *op == SendOpcode::OwlGoResult as i16)
        .map(|(_, body)| body.clone())
        .collect();
    assert_eq!(go_results, [vec![0]]);
    assert_eq!(t.warps, [MARKET]);

    // Once A logs out the shop is closed, and GO says so.
    drop(a);
    let mut t = Trace::default();
    until(&mut b.connection, &mut t, |t| {
        t.rooms.iter().any(|r| r.starts_with(&[0xa, 1]))
    })
    .await;
    let t = step(&mut b.connection, "closed", go(room, MARKET)).await;
    let codes: Vec<_> = t.owl.iter().map(|(_, body)| body.clone()).collect();
    assert_eq!(codes, [vec![1]]);
}
