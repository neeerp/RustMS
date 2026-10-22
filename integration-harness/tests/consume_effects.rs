//! An item buff's aura (SHOW_CONSUME_EFFECT, recv 166): the player and
//! the map are told when Horntail Squad Victory starts, those who arrive
//! later and the player on a new map are told again, and cancelling the
//! buff takes it away for everyone.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::hunting::{cancel_item_effect, use_item};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;

const USE: i16 = 2;
const SQUAD_VICTORY: i32 = 2_022_108;
const ELLINIA: i32 = 101_000_000;
const HENESYS: i32 = 100_000_000;

/// Read until the next SHOW_CONSUME_EFFECT, and return its player and item.
async fn aura(c: &mut MapleTestConnection, t: &mut Trace) -> (i32, i32) {
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::ShowConsumeEffect as i16 {
            let int = |at: usize| i32::from_le_bytes(p.bytes[at..at + 4].try_into().unwrap());
            return (int(2), int(6));
        }
    }
}

#[tokio::test]
async fn an_item_buff_shows_its_aura_to_the_map() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("drinker login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, SQUAD_VICTORY, 1)
        .await
        .expect("grant the potion");
    let mut drinker = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (d, mut dt) = (&mut drinker.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);

    // The drinker's own client is told as well as the map's.
    send(d, use_item(1, SQUAD_VICTORY)).await;
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));

    // SPAWN_PLAYER doesn't carry it: someone arriving is told after it,
    // and so is the drinker on their next map.
    assert_eq!(warp(w, HENESYS, "", &mut wt).await, HENESYS);
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));
    assert_eq!(warp(d, HENESYS, "", &mut dt).await, HENESYS);
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(warp(d, ELLINIA, "", &mut dt).await, ELLINIA);
    assert_eq!(aura(d, &mut dt).await, (id, SQUAD_VICTORY));
    assert_eq!(aura(w, &mut wt).await, (id, SQUAD_VICTORY));

    // Cancelling the buff takes it away.
    send(d, cancel_item_effect(-SQUAD_VICTORY)).await;
    assert_eq!(aura(d, &mut dt).await, (id, 0));
    assert_eq!(aura(w, &mut wt).await, (id, 0));
}
