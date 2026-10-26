//! Omok (PLAYER_INTERACTION, mini room type 1): a player with an Omok set
//! opens a game, another sits down, says they are ready, and the two take
//! turns until five stones in a row win. Whoever leaves a game under way
//! loses it, and both records last, shown to whoever sits down next.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MAP: i32 = 40_000;
const OMOK_SET: i32 = 4080000;
const ETC: u8 = 4;
/// A map whose fieldLimit keeps games from starting (0x80).
const NO_GAMES: i32 = 103_040_410;
const TITLE: &str = "Omok";

fn interaction(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PlayerInteraction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn create() -> Packet {
    let mut p = interaction(0);
    p.write_byte(1).unwrap();
    p.write_str_with_length(TITLE).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

fn visit(room: i32) -> Packet {
    let mut p = interaction(4);
    p.write_int(room).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

fn stone(x: i32, y: i32, color: u8) -> Packet {
    let mut p = interaction(0x40);
    p.write_int(x).unwrap();
    p.write_int(y).unwrap();
    p.write_byte(color).unwrap();
    p
}

/// The game room id in the balloon over `owner`, once it shows.
async fn balloon(c: &mut MapleTestConnection, owner: i32) -> i32 {
    let mut t = Trace::default();
    let over = |b: &Vec<u8>| (&b[..4]).read_int().unwrap() == owner && b[4] == 1;
    until(c, &mut t, |t| t.char_boxes.iter().any(over)).await;
    let body = t.char_boxes.iter().rfind(|b| over(b)).unwrap();
    (&body[5..]).read_int().unwrap()
}

/// Wins, ties and losses in a 20-byte record.
fn record(bytes: &[u8]) -> (i32, i32, i32) {
    let mut r = &bytes[4..];
    (
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_int().unwrap(),
    )
}

/// The records an enter packet for two shows, in slot order: they come
/// before 0xFF, the title, the stones and the tournament flag.
fn entered_records(body: &[u8]) -> [(i32, i32, i32); 2] {
    let end = body.len() - (1 + 2 + TITLE.len() + 2);
    assert_eq!(body[end], 0xff);
    let start = end - 2 * 21;
    assert_eq!((body[start], body[start + 21]), (0, 1));
    [
        record(&body[start + 1..start + 21]),
        record(&body[start + 22..end]),
    ]
}

async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    id
}

async fn meet(a: &mut WorldSession, b: &mut WorldSession) {
    let (mut at, mut bt) = (Trace::default(), Trace::default());
    assert_eq!(warp(&mut a.connection, MAP, "", &mut at).await, MAP);
    assert_eq!(warp(&mut b.connection, MAP, "", &mut bt).await, MAP);
    read_for(&mut a.connection, &mut at, 300).await;
    read_for(&mut b.connection, &mut bt, 300).await;
}

/// Read into `t` until it has a room packet starting with `head`.
async fn more(c: &mut MapleTestConnection, t: &mut Trace, head: &[u8]) {
    until(c, t, |t| t.rooms.iter().any(|b| b.starts_with(head))).await;
}

async fn room_packet(c: &mut MapleTestConnection, head: &[u8]) -> Trace {
    let mut t = Trace::default();
    more(c, &mut t, head).await;
    t
}

/// `by` puts a stone at (x, y), and both see it go down.
async fn play(by: &mut WorldSession, other: &mut WorldSession, x: i32, y: i32, color: u8) {
    send(&mut by.connection, stone(x, y, color)).await;
    let mut placed = vec![0x40];
    placed.extend(x.to_le_bytes());
    placed.extend(y.to_le_bytes());
    placed.push(color);
    room_packet(&mut by.connection, &placed).await;
    room_packet(&mut other.connection, &placed).await;
}

/// A opens a game, B sits down and is ready, and A starts it.
async fn sit_down_and_start(a: &mut WorldSession, b: &mut WorldSession, a_id: i32) -> Trace {
    let t = step(&mut a.connection, "create", create()).await;
    assert_eq!(t.rooms[0][..4], [5, 1, 2, 0], "{:?}", t.rooms);
    // As v83's owner does on entering (report 164 §4), to no effect.
    let mut open = interaction(0xb);
    open.write_byte(1).unwrap();
    send(&mut a.connection, open).await;
    let room = balloon(&mut b.connection, a_id).await;
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(t.rooms[0][..4], [5, 1, 2, 1], "{:?}", t.rooms);
    room_packet(&mut a.connection, &[4, 1]).await;
    send(&mut b.connection, interaction(0x3a)).await;
    room_packet(&mut a.connection, &[0x3a]).await;
    send(&mut a.connection, interaction(0x3d)).await;
    room_packet(&mut a.connection, &[0x3d]).await;
    room_packet(&mut b.connection, &[0x3d]).await
}

#[tokio::test]
async fn five_in_a_row_wins_and_leaving_loses() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let a_id = new_character(&a_config).await;
    let _b_id = new_character(&b_config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(a_id, ETC.into(), 1, OMOK_SET)
        .await
        .expect("give A an Omok set");
    let mut a = relogin(&a_config).await;
    let mut b = relogin(&b_config).await;

    // Nor where the map's fieldLimit has 0x80.
    let mut t = Trace::default();
    let forbidden = warp(&mut a.connection, NO_GAMES, "", &mut t).await;
    assert_eq!(forbidden, NO_GAMES);
    let t = step(&mut a.connection, "no games here", create()).await;
    assert!(t.rooms.is_empty(), "{:?}", t.rooms);

    meet(&mut a, &mut b).await;

    // Without a set, there is no game.
    let t = step(&mut b.connection, "no set", create()).await;
    assert!(t.rooms.is_empty(), "{:?}", t.rooms);

    // A waits first, so B opens with black.
    let t = sit_down_and_start(&mut a, &mut b, a_id).await;
    assert!(t.rooms.contains(&vec![0x3d, 0]), "{:?}", t.rooms);
    for x in 0..4 {
        play(&mut b, &mut a, x, 0, 1).await;
        play(&mut a, &mut b, x, 5, 2).await;
    }
    send(&mut b.connection, stone(4, 0, 1)).await;
    let t = room_packet(&mut a.connection, &[0x3e]).await;
    let result = t.rooms.iter().find(|r| r[0] == 0x3e).unwrap();
    // Won, by slot 1; A lost and B won.
    assert_eq!(result[1..3], [0, 1]);
    assert_eq!(record(&result[3..23]), (0, 0, 1));
    assert_eq!(record(&result[23..43]), (1, 0, 0));
    room_packet(&mut b.connection, &[0x3e]).await;

    // B leaves the next game under way, and loses it.
    send(&mut b.connection, interaction(0x3a)).await;
    room_packet(&mut a.connection, &[0x3a]).await;
    send(&mut a.connection, interaction(0x3d)).await;
    room_packet(&mut b.connection, &[0x3d]).await;
    send(&mut b.connection, interaction(0xa)).await;
    let mut t = room_packet(&mut a.connection, &[0x3e]).await;
    let result = t.rooms.iter().find(|r| r[0] == 0x3e).unwrap();
    assert_eq!(result[1..3], [2, 0], "a forfeit, won by A");
    more(&mut a.connection, &mut t, &[0xa, 1]).await;
    send(&mut a.connection, interaction(0xa)).await;
    drop(b);

    // The records outlast the session.
    let mut b = relogin(&b_config).await;
    meet(&mut a, &mut b).await;
    let t = step(&mut a.connection, "create", create()).await;
    assert_eq!(t.rooms[0][..4], [5, 1, 2, 0], "{:?}", t.rooms);
    let room = balloon(&mut b.connection, a_id).await;
    let t = step(&mut b.connection, "visit", visit(room)).await;
    assert_eq!(entered_records(&t.rooms[0]), [(1, 0, 1), (1, 0, 1)]);
    let t = room_packet(&mut a.connection, &[4, 1]).await;
    let visited = t.rooms.iter().find(|r| r.starts_with(&[4, 1])).unwrap();
    assert_eq!(record(&visited[visited.len() - 20..]), (1, 0, 1));
}
