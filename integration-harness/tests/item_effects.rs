//! Effect items: the rest of the map sees a player's effect, those who
//! arrive later too, and item 0 takes it away. The player's own client
//! draws it without being told, and items they don't hold are ignored.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const CASH: i16 = 5;
const SUNNY_DAY: i32 = 5_010_000;
const ELLINIA: i32 = 101_000_000;
const HENESYS: i32 = 100_000_000;

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

fn use_effect(item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseItemEffect as i16).unwrap();
    p.write_int(item_id).unwrap();
    p
}

fn shown(id: i32, item_id: i32) -> Vec<u8> {
    [id.to_le_bytes(), item_id.to_le_bytes()].concat()
}

/// Watch `id` arrive on the watcher's map, and return the effect they show.
async fn arrives(w: &mut MapleTestConnection, wt: &mut Trace, id: i32) -> i32 {
    wt.players.clear();
    until(w, wt, |t| t.players.iter().any(|p| p.character_id == id)).await;
    let seen = wt.players.iter().find(|p| p.character_id == id).unwrap();
    seen.item_effect
}

#[tokio::test]
async fn an_effect_item_shows_to_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("shower login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, CASH, 1, SUNNY_DAY, 1)
        .await
        .expect("grant an effect item");
    let mut shower = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (s, mut st) = (&mut shower.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(s, ELLINIA, "", &mut st).await, ELLINIA);
    assert_eq!(arrives(w, &mut wt, id).await, 0);

    // The watcher is told; the shower's client already draws it.
    send(s, use_effect(SUNNY_DAY)).await;
    assert_eq!(
        next(w, &mut wt, SendOpcode::ShowItemEffect).await,
        shown(id, SUNNY_DAY)
    );
    send(s, build_all_chat("shown", 0).unwrap()).await;
    loop {
        let p = read(s, &mut st).await;
        assert_ne!(
            p.opcode(),
            SendOpcode::ShowItemEffect as i16,
            "not to itself"
        );
        if p.opcode() == SendOpcode::ChatText as i16 {
            break;
        }
    }

    // Someone arriving sees it in SPAWN_PLAYER, and it comes along to the
    // next map.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(arrives(w, &mut wt, id).await, SUNNY_DAY);
    assert_eq!(warp(s, HENESYS, "", &mut st).await, HENESYS);
    assert_eq!(warp(s, ELLINIA, "", &mut st).await, ELLINIA);
    assert_eq!(arrives(w, &mut wt, id).await, SUNNY_DAY);

    // An item not held changes nothing; item 0 takes it away.
    send(s, use_effect(SUNNY_DAY + 1)).await;
    send(s, use_effect(0)).await;
    assert_eq!(
        next(w, &mut wt, SendOpcode::ShowItemEffect).await,
        shown(id, 0)
    );
}
