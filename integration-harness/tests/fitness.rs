//! The Physical Fitness Test (report b172): a GM's `/fitness1` opens it,
//! Paul takes players to its lobby, and its gate (`join00`) stays closed
//! until the GM's `/start` there. Then the clock runs on all five maps, a
//! player goes through the gate and on to the first stage, and when the
//! time is up everyone still inside goes out to 109050001. The GM stays,
//! and a player who comes in to a stage after the time goes out.
//! The harness runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the
//! test's 900 s are 15.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::packets::{build_all_chat, build_change_map};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, step, until, until_within, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const PAUL: i32 = 9000000;
const LOBBY: i32 = 109_040_000;
const STAGE_1: i32 = 109_040_001;
const EXIT: i32 = 109_050_001;
/// Where the lobby's gate leads, up by its way on (`join01`).
const PAST_THE_GATE: u8 = 24;
const NOTICE: u8 = 0;
const CLOSED: (i16, u8) = (SendOpcode::BlockedMap as i16, 1);

fn talk_to_paul() -> Packet {
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let index = maps
        .field(SOUTHPERRY)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == PAUL)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// Through the gate, from the lobby.
async fn gate(c: &mut MapleTestConnection, m: &str) -> Trace {
    let mut t = Trace::default();
    send(c, build_change_map(-1, "join00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty() || !t.warps.is_empty()).await;
    barrier(c, m, &mut t).await;
    t
}

#[tokio::test]
async fn the_gate_opens_at_the_start_and_the_time_up_sends_everyone_out() {
    let paul = talk_to_paul();
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    for config in &configs {
        login_to_world_session(config).await.expect("player login");
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(g, build_all_chat("/fitness1", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;

    // Paul takes both to the lobby.
    let mut players = Vec::new();
    for config in &configs {
        let mut session = relogin(config).await;
        let (c, m) = (&mut session.connection, config.character_name.as_str());
        let mut t = Trace::default();
        assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
        barrier(c, m, &mut t).await;
        let t = step(c, m, paul.clone()).await;
        assert_eq!(t.prompts, [(PAUL, 4)], "his menu");
        let t = step(c, m, answer(4, 1, Some(1))).await;
        assert_eq!(t.warps, [LOBBY], "in");
        assert!(t.countdowns.is_empty(), "no clock yet");
        players.push((session, config.character_name.clone()));
    }
    let [(runner, rm), (stayer, _)] = &mut players[..] else {
        unreachable!()
    };

    // The gate is closed until the GM starts the test.
    let t = gate(&mut runner.connection, rm).await;
    assert_eq!(t.blocked, [CLOSED], "closed");
    assert!(t.warps.is_empty());

    assert_eq!(warp(g, LOBBY, "", &mut gt).await, LOBBY);
    barrier(g, "in the lobby", &mut gt).await;
    send(g, build_all_chat("/start", 0).unwrap()).await;
    let mut st = Trace::default();
    until(&mut stayer.connection, &mut st, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    assert_eq!(st.countdowns, [15], "the clock");

    // Now it opens, up to the way on, and the clock follows to stage 1.
    let mut rt = Trace::default();
    until(&mut runner.connection, &mut rt, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    let t = gate(&mut runner.connection, rm).await;
    assert!(t.blocked.is_empty(), "open");
    assert_eq!(t.warps, [LOBBY]);
    assert_eq!(t.arrivals[0].1, PAST_THE_GATE);
    let mut rt = Trace::default();
    assert_eq!(
        warp(&mut runner.connection, -1, "in00", &mut rt).await,
        STAGE_1
    );
    until(&mut runner.connection, &mut rt, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    assert!((1..=15).contains(&rt.countdowns[0]), "the clock left");

    // The time is up: the runner on stage 1 and the stayer in the lobby
    // go out, and each heard the notices on the way. The 15 s run from the
    // GM's `/start`, so the wait for them is longer.
    until_within(
        &mut runner.connection,
        &mut rt,
        Duration::from_secs(30),
        |t| t.warps.len() == 2,
    )
    .await;
    assert_eq!(rt.warps, [STAGE_1, EXIT], "out of stage 1");
    until(&mut stayer.connection, &mut st, |t| !t.warps.is_empty()).await;
    assert_eq!(st.warps, [EXIT], "out of the lobby");
    let notices = |t: &Trace| t.broadcasts.iter().filter(|(k, _)| *k == NOTICE).count();
    assert_eq!(notices(&st), 13, "every notice");
    assert!(notices(&rt) >= 10, "the notices on stage 1");

    // The stages are closed after it: one who comes in late goes out.
    let mut t = Trace::default();
    let late = &mut stayer.connection;
    assert_eq!(warp(late, STAGE_1, "", &mut t).await, STAGE_1);
    until(late, &mut t, |t| t.warps.len() == 2).await;
    assert_eq!(t.warps, [STAGE_1, EXIT], "late");

    // The GM stays, and the gate is closed again.
    barrier(g, "over", &mut gt).await;
    assert_eq!(gt.warps, [LOBBY]);
    let t = gate(g, "closed again").await;
    assert_eq!(t.blocked, [CLOSED]);
    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}
