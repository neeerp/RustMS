//! The jukebox: used, it plays its song for everyone on the map, under a
//! line with the player's name, and is used up.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const CASH: i16 = 5;
const SONG: i32 = 5_100_000;
const PERION: i32 = 102_000_000;

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

/// USE_CASH_ITEM: play the jukebox in Cash slot 1, whose song lasts 67 s.
fn play() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(SONG).unwrap();
    p.write_int(67_030).unwrap();
    p.write_int(0).unwrap();
    p
}

/// The song and the name of whoever played it.
fn song(body: &[u8]) -> (i32, String) {
    let item = i32::from_le_bytes(body[..4].try_into().unwrap());
    (item, String::from_utf8_lossy(&body[6..]).into_owned())
}

#[tokio::test]
async fn a_jukebox_plays_for_the_map() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config)
        .await
        .expect("player login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, SONG, 1)
        .await
        .expect("grant a jukebox");
    let mut player = relogin(&config).await;
    let name = player.character_name.clone();
    let listener_config = load_harness_config_or_fail().await;
    let mut listener = login_to_world_session(&listener_config).await.unwrap();
    let (p, mut pt) = (&mut player.connection, Trace::default());
    let (l, mut lt) = (&mut listener.connection, Trace::default());

    assert_eq!(warp(l, PERION, "", &mut lt).await, PERION);
    assert_eq!(warp(p, PERION, "", &mut pt).await, PERION);
    send(p, play()).await;
    next(p, &mut pt, SendOpcode::InventoryOperation).await;
    assert_eq!(pt.removed, [(CASH as u8, 1)], "the jukebox is used up");
    let heard = (SONG, name);
    assert_eq!(
        song(&next(p, &mut pt, SendOpcode::PlayJukebox).await),
        heard
    );
    assert_eq!(
        song(&next(l, &mut lt, SendOpcode::PlayJukebox).await),
        heard
    );
}
