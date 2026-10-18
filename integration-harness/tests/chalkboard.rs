//! The chalkboard: held up, everyone on the map sees the note, those who
//! arrive later too, and it comes along to the next map but not into the
//! Free Market rooms. Clicking it puts it away.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const CASH: i16 = 5;
const CHALKBOARD: i32 = 5_370_000;
const ELLINIA: i32 = 101_000_000;
const HENESYS: i32 = 100_000_000;
const FREE_MARKET_ROOM: i32 = 910_000_001;
const NOTE: &str = "selling ilbis";

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

fn write_note(note: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(CHALKBOARD).unwrap();
    p.write_str_with_length(note).unwrap();
    p.write_int(0).unwrap();
    p
}

fn close() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CloseChalkboard as i16).unwrap();
    p
}

/// CHALKBOARD's body: the holder, then the note if the board is up.
fn board(id: i32, note: Option<&str>) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    match note {
        Some(note) => {
            body.push(1);
            body.extend((note.len() as u16).to_le_bytes());
            body.extend(note.as_bytes());
        }
        None => body.push(0),
    }
    body
}

/// Watch the holder `id` arrive on the watcher's map, and return the note
/// they arrive with.
async fn arrives(w: &mut MapleTestConnection, wt: &mut Trace, id: i32) -> Option<String> {
    wt.players.clear();
    until(w, wt, |t| t.players.iter().any(|p| p.character_id == id)).await;
    let seen = wt.players.iter().find(|p| p.character_id == id).unwrap();
    seen.chalkboard.clone()
}

#[tokio::test]
async fn a_chalkboard_shows_its_note_on_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("holder login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, CHALKBOARD, 1)
        .await
        .expect("grant a chalkboard");
    let mut holder = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (h, mut ht) = (&mut holder.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(h, ELLINIA, "", &mut ht).await, ELLINIA);
    assert_eq!(arrives(w, &mut wt, id).await, None);

    // Held up, the holder and the watcher see the note, and the holder's
    // client is unlocked.
    send(h, write_note(NOTE)).await;
    assert_eq!(
        next(h, &mut ht, SendOpcode::Chalkboard).await,
        board(id, Some(NOTE))
    );
    let unlocked = next(h, &mut ht, SendOpcode::StatChange).await;
    assert_eq!(unlocked[0], 1, "an empty stat change unlocks the client");
    assert_eq!(
        next(w, &mut wt, SendOpcode::Chalkboard).await,
        board(id, Some(NOTE))
    );

    // It comes along to the next map, where the holder's client is told
    // again, and back, where the watcher sees it on arrival.
    assert_eq!(warp(h, HENESYS, "", &mut ht).await, HENESYS);
    assert_eq!(
        next(h, &mut ht, SendOpcode::Chalkboard).await,
        board(id, Some(NOTE))
    );
    assert_eq!(warp(h, ELLINIA, "", &mut ht).await, ELLINIA);
    next(h, &mut ht, SendOpcode::Chalkboard).await;
    assert_eq!(arrives(w, &mut wt, id).await.as_deref(), Some(NOTE));

    // Clicked, it is put away for both.
    send(h, close()).await;
    assert_eq!(
        next(h, &mut ht, SendOpcode::Chalkboard).await,
        board(id, None)
    );
    assert_eq!(
        next(w, &mut wt, SendOpcode::Chalkboard).await,
        board(id, None)
    );

    // The Free Market rooms put it away too.
    send(h, write_note(NOTE)).await;
    next(w, &mut wt, SendOpcode::Chalkboard).await;
    assert_eq!(
        warp(h, FREE_MARKET_ROOM, "", &mut ht).await,
        FREE_MARKET_ROOM
    );
    assert_eq!(warp(h, ELLINIA, "", &mut ht).await, ELLINIA);
    assert_eq!(arrives(w, &mut wt, id).await, None);
}
