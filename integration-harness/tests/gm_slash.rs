//! v83's GM slash commands, as the client sends them: `/noticeall` is an
//! ADMIN_BROADCAST the world sees, and `/w` an ADMIN_COMMAND 0x1E warning
//! that pops up for the player named. A player's are ignored.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;

fn notice_all(text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AdminBroadcast as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

fn warn(name: &str, text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AdminCommand as i16).unwrap();
    p.write_byte(0x1E).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

/// The next BROADCAST_MSG `c` receives.
async fn next_broadcast(c: &mut MapleTestConnection) -> (u8, String) {
    let mut trace = Trace::default();
    while trace.broadcasts.is_empty() {
        read(c, &mut trace).await;
    }
    trace.broadcasts.remove(0)
}

/// The value of the next ADMIN_RESULT 0x1E `c` receives.
async fn warn_result(c: &mut MapleTestConnection) -> u8 {
    let mut trace = Trace::default();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::AdminResult as i16 && p.bytes[2] == 0x1E {
            let mut r = Cursor::new(&p.bytes[3..]);
            return r.read_byte().unwrap();
        }
    }
}

#[tokio::test]
async fn a_gms_notice_and_warning_reach_players() {
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
    set_gm_level(&config.username, 3).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let (g, p) = (&mut gm.connection, &mut player.connection);
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The player's own go nowhere.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(p, notice_all("from a player")).await;
    send(p, warn(&config.character_name, "from a player")).await;
    barrier(p, "player done", &mut pt).await;
    barrier(g, "gm done", &mut gt).await;
    assert!(pt.broadcasts.is_empty(), "{:?}", pt.broadcasts);
    assert!(gt.broadcasts.is_empty(), "{:?}", gt.broadcasts);

    send(g, notice_all("hello world")).await;
    assert_eq!(next_broadcast(p).await, (0, "hello world".to_string()));

    send(g, warn(&other_config.character_name, "stop that")).await;
    assert_eq!(next_broadcast(p).await, (1, "stop that".to_string()));
    assert_eq!(warn_result(g).await, 1);

    send(g, warn("nobody_here", "stop that")).await;
    assert_eq!(warn_result(g).await, 0);
}
