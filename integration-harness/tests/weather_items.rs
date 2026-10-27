//! Weather items: the map sees the weather with the player's message, those
//! who arrive while it blows too, and it stops after 30 s. Maps with field
//! limit 0x4000 and the wedding halls refuse them. A buff weather also gives
//! everyone on the map its item's effect.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const CASH: i16 = 5;
const SNOW: i32 = 5_120_000;
/// Fighting Spirit, which gives what a 2022071 does: +20 attack and magic
/// attack for 10 minutes.
const FIGHTING_SPIRIT: i32 = 5_121_000;
const FIGHTING_SPIRIT_BUFF: i32 = 2_022_071;
const PERION: i32 = 102_000_000;
const HENESYS: i32 = 100_000_000;
/// Where the buff weather blows, away from the snow's test beside it.
const KERNING_CITY: i32 = 103_000_000;
/// A training room with field limit 0x4000.
const NO_WEATHER: i32 = 910_060_000;
/// A wedding hall (fieldType 60).
const WEDDING_HALL: i32 = 680_000_110;

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
    blow_from(1, SNOW, message)
}

fn blow_from(slot: i16, item_id: i32, message: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
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
    for map in [NO_WEATHER, WEDDING_HALL] {
        assert_eq!(warp(b, map, "", &mut bt).await, map);
        send(b, blow("hi")).await;
        loop {
            let p = read(b, &mut bt).await;
            assert_ne!(p.opcode(), SendOpcode::BlowWeather as i16, "refused");
            assert_ne!(p.opcode(), SendOpcode::InventoryOperation as i16, "kept");
            if p.opcode() == SendOpcode::StatChange as i16 {
                break;
            }
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

#[tokio::test]
async fn a_buff_weather_gives_everyone_on_the_map_its_buff() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("blower login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, FIGHTING_SPIRIT, 1)
        .await
        .expect("grant a buff weather");
    let mut blower = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (b, mut bt) = (&mut blower.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());

    assert_eq!(warp(w, KERNING_CITY, "", &mut wt).await, KERNING_CITY);
    assert_eq!(warp(b, KERNING_CITY, "", &mut bt).await, KERNING_CITY);
    send(b, blow_from(1, FIGHTING_SPIRIT, "fight on")).await;
    let body = next(w, &mut wt, SendOpcode::BlowWeather).await;
    assert_eq!(body[1..5], FIGHTING_SPIRIT.to_le_bytes(), "{body:?}");
    let buff = (-FIGHTING_SPIRIT_BUFF, 20, 600_000);
    until(b, &mut bt, |t| t.buffs.contains(&buff)).await;
    until(w, &mut wt, |t| t.buffs.contains(&buff)).await;
}
