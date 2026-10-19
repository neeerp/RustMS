//! Weather items: the map sees the weather with the player's message, those
//! who arrive while it blows too, and it stops after 30 s. Maps with field
//! limit 0x4000 refuse them.

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
const SNOW: i32 = 5_120_000;
const PERION: i32 = 102_000_000;
const HENESYS: i32 = 100_000_000;
/// A training room with field limit 0x4000.
const NO_WEATHER: i32 = 910_060_000;

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

fn blow(message: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(SNOW).unwrap();
    p.write_str_with_length(message).unwrap();
    p.write_int(0).unwrap();
    p
}

/// A snow weather's body, whoever blew it.
fn assert_snow(body: &[u8], message: &str) {
    assert_eq!(body[..5], [0, 0x00, 0x20, 0x4E, 0x00], "{body:?}");
    let text = String::from_utf8_lossy(&body[7..]);
    assert!(text.ends_with(&format!("'s snow : {message}")), "{text}");
}

#[tokio::test]
async fn a_weather_item_blows_over_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("blower login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, SNOW, 2)
        .await
        .expect("grant a weather item");
    let mut blower = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (b, mut bt) = (&mut blower.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());

    // Where it isn't allowed, the item is kept and the client unlocked.
    assert_eq!(warp(b, NO_WEATHER, "", &mut bt).await, NO_WEATHER);
    send(b, blow("hi")).await;
    loop {
        let p = read(b, &mut bt).await;
        assert_ne!(p.opcode(), SendOpcode::BlowWeather as i16, "refused");
        if p.opcode() == SendOpcode::StatChange as i16 {
            break;
        }
    }

    assert_eq!(warp(w, PERION, "", &mut wt).await, PERION);
    assert_eq!(warp(b, PERION, "", &mut bt).await, PERION);
    send(b, blow("let it snow")).await;
    next(b, &mut bt, SendOpcode::InventoryOperation).await;
    assert_snow(
        &next(b, &mut bt, SendOpcode::BlowWeather).await,
        "let it snow",
    );
    assert_snow(
        &next(w, &mut wt, SendOpcode::BlowWeather).await,
        "let it snow",
    );

    // Someone arriving while it blows sees it too.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, PERION, "", &mut wt).await, PERION);
    assert_snow(
        &next(w, &mut wt, SendOpcode::BlowWeather).await,
        "let it snow",
    );

    // It stops 30 s after it started.
    let stop = tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            let p = w.read_packet("weather").await.expect("read packet").packet;
            if p.opcode() == SendOpcode::BlowWeather as i16 {
                return p.bytes[2..].to_vec();
            }
        }
    })
    .await
    .expect("the weather stops");
    assert_eq!(stop, [0, 0, 0, 0, 0]);
}
