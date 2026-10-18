//! Teleport rocks: the player keeps maps in a list, and a rock takes them to
//! one, or to a character on the channel, one rock per trip. The list
//! outlasts a relog.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const CASH: i16 = 5;
const ROCK: i32 = 5_040_000;
const ROCKS: i16 = 5;
const HENESYS: i32 = 100_000_000;
const ELLINIA: i32 = 101_000_000;
const EMPTY: i32 = 999_999_999;

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

/// Read until the next map change, and return the map.
async fn arrival(c: &mut MapleTestConnection, t: &mut Trace) -> i32 {
    let before = t.warps.len();
    until(c, t, |t| t.warps.len() > before).await;
    t.warps[before]
}

/// Keep the player's map in the regular list.
fn keep() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TeleportRockList as i16).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// Use the regular rock in Cash slot 1 to go to `to`: a map or a name.
fn use_rock(to: Result<i32, &str>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(ROCK).unwrap();
    match to {
        Ok(map_id) => {
            p.write_byte(0).unwrap();
            p.write_int(map_id).unwrap();
        }
        Err(name) => {
            p.write_byte(1).unwrap();
            p.write_str_with_length(name).unwrap();
        }
    }
    p.write_int(0).unwrap();
    p
}

/// Read a refusal, and the empty stat change that unlocks the client.
async fn refused(c: &mut MapleTestConnection, t: &mut Trace) -> u8 {
    let refusal = next(c, t, SendOpcode::MapTransferResult).await;
    assert_eq!(refusal.len(), 2, "a refusal and the list it is about");
    let unlocked = next(c, t, SendOpcode::StatChange).await;
    assert_eq!(unlocked[0], 1, "an empty stat change unlocks the client");
    refusal[0]
}

#[tokio::test]
async fn a_teleport_rock_goes_to_kept_maps_and_characters() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("traveller login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, ROCK, ROCKS)
        .await
        .expect("grant teleport rocks");
    let mut traveller = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (h, mut ht) = (&mut traveller.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());

    // Henesys is kept, and the list comes back.
    assert_eq!(warp(h, HENESYS, "", &mut ht).await, HENESYS);
    send(h, keep()).await;
    let mut list = vec![3, 0];
    list.extend(HENESYS.to_le_bytes());
    for _ in 0..4 {
        list.extend(EMPTY.to_le_bytes());
    }
    assert_eq!(next(h, &mut ht, SendOpcode::MapTransferResult).await, list);

    // From Ellinia a rock takes the traveller back, and is used up.
    assert_eq!(warp(h, ELLINIA, "", &mut ht).await, ELLINIA);
    send(h, use_rock(Ok(HENESYS))).await;
    assert_eq!(arrival(h, &mut ht).await, HENESYS);
    assert_eq!(ht.quantities.last(), Some(&(CASH as u8, 1, ROCKS - 1)));

    // Not to the map they are on, nor to a character who isn't there.
    send(h, use_rock(Ok(HENESYS))).await;
    assert_eq!(refused(h, &mut ht).await, 9);
    send(h, use_rock(Err("Nobody"))).await;
    assert_eq!(refused(h, &mut ht).await, 6);

    // To a character on the channel.
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    send(h, use_rock(Err(&watcher.character_name))).await;
    assert_eq!(arrival(h, &mut ht).await, ELLINIA);
    assert_eq!(ht.quantities.last(), Some(&(CASH as u8, 1, ROCKS - 2)));

    // The list outlasts a relog.
    drop(traveller);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut traveller = relogin(&config).await;
    let (h, mut ht) = (&mut traveller.connection, Trace::default());
    send(h, use_rock(Ok(HENESYS))).await;
    assert_eq!(arrival(h, &mut ht).await, HENESYS);
}
