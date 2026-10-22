//! Kites (report c014): the map sees a kite with its flier's name and lines,
//! and so does whoever comes while it flies. The Free Market refuses them
//! and keeps the item.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use std::time::Duration;

const CASH: i16 = 5;
const KITE: i32 = 5_080_000;
const KERNING: i32 = 103_000_000;
const HENESYS: i32 = 100_000_000;
const FREE_MARKET: i32 = 910_000_000;

/// Read until a packet with `op`, and return its body.
async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

fn fly(message: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(KITE).unwrap();
    p.write_str_with_length(message).unwrap();
    p.write_int(0).unwrap();
    p
}

/// A spawned kite's (object id, item id, lines, name).
fn kite(body: Vec<u8>) -> (i32, i32, String, String) {
    let mut r = Cursor::new(&body);
    let object_id = r.read_int().unwrap();
    let item_id = r.read_int().unwrap();
    let message = r.read_str_with_length().unwrap();
    let name = r.read_str_with_length().unwrap();
    r.read_short().unwrap();
    r.read_short().unwrap();
    (object_id, item_id, message, name)
}

#[tokio::test]
async fn a_kite_flies_over_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("flier login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, KITE, 2)
        .await
        .expect("grant a kite");
    let mut flier = relogin(&config).await;
    let name = flier.character_name.clone();
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (f, mut ft) = (&mut flier.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());

    // The Free Market refuses it, keeps the item and unlocks the client.
    assert_eq!(warp(f, FREE_MARKET, "", &mut ft).await, FREE_MARKET);
    send(f, fly("hi\n\n")).await;
    loop {
        let p = read(f, &mut ft).await;
        assert_ne!(p.opcode(), SendOpcode::SpawnKite as i16, "refused");
        assert_ne!(p.opcode(), SendOpcode::InventoryOperation as i16, "kept");
        if p.opcode() == SendOpcode::StatChange as i16 {
            break;
        }
    }

    assert_eq!(warp(w, KERNING, "", &mut wt).await, KERNING);
    assert_eq!(warp(f, KERNING, "", &mut ft).await, KERNING);
    send(f, fly("happy\nnew\nyear")).await;
    next(f, &mut ft, SendOpcode::InventoryOperation).await;
    let flown = kite(next(f, &mut ft, SendOpcode::SpawnKite).await);
    assert_eq!(
        (flown.1, flown.2.as_str(), flown.3.as_str()),
        (KITE, "happy\nnew\nyear", name.as_str())
    );
    assert_eq!(kite(next(w, &mut wt, SendOpcode::SpawnKite).await), flown);

    // Someone arriving while it flies sees it too.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, KERNING, "", &mut wt).await, KERNING);
    let seen = loop {
        let seen = kite(next(w, &mut wt, SendOpcode::SpawnKite).await);
        if seen.0 == flown.0 {
            break seen;
        }
    };
    assert_eq!(seen, flown);
}
