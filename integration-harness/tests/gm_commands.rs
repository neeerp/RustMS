//! Chat commands (`net::gm_commands`): a GM's `!` line runs and nobody
//! else sees it; a player's is ordinary chat and gives nothing.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;
const HENESYS: i32 = 100_000_000;
const RED_POTION: i32 = 2_000_000;

/// The chat lines `c` receives up to and including `marker`.
async fn chat_until(c: &mut MapleTestConnection, marker: &str) -> Vec<String> {
    let mut trace = Trace::default();
    let mut lines = Vec::new();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() != SendOpcode::ChatText as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        r.read_int().unwrap();
        r.read_byte().unwrap();
        let line = r.read_str_with_length().unwrap();
        let done = line == marker;
        lines.push(line);
        if done {
            return lines;
        }
    }
}

#[tokio::test]
async fn a_gms_bang_lines_run_and_a_players_are_chat() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 2).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let (g, p) = (&mut gm.connection, &mut player.connection);
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The player's line is chat that both see, and gives nothing.
    let mut pt = Trace::default();
    send(p, build_all_chat("!item 2000000 5", 0).unwrap()).await;
    barrier(p, "player done", &mut pt).await;
    assert!(pt.added.is_empty(), "{:?}", pt.added);
    let seen = chat_until(g, "player done").await;
    assert!(seen.contains(&"!item 2000000 5".to_string()), "{seen:?}");

    // The GM's lines run, and the player sees none of them.
    let mut gt = Trace::default();
    send(g, build_all_chat("!item 2000000 5", 0).unwrap()).await;
    send(g, build_all_chat("!meso 1000", 0).unwrap()).await;
    send(g, build_all_chat("!nothing", 0).unwrap()).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(gt.items, [(RED_POTION, 5)]);
    assert_eq!(gt.meso_gains, [1000]);
    assert_eq!(
        gt.broadcasts,
        [(5, "Unknown command. Type !help for a list.".to_string())]
    );
    assert_eq!(chat_until(p, "gm done").await, ["gm done"]);

    // And a warp.
    send(g, build_all_chat(&format!("!warp {HENESYS}"), 0).unwrap()).await;
    let before = gt.warps.len();
    while gt.warps.len() == before {
        read(g, &mut gt).await;
    }
    assert_eq!(gt.warps.last(), Some(&HENESYS));
}
