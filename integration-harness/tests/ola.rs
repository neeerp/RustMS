//! Ola Ola (report b172). Variant 2: a GM's `/ola21` opens it, Paul takes
//! players to its first level, and its gate (`join00`) stays closed until
//! the GM's `/start`. Then a player goes through the gate and the right
//! portal on each of the three levels to 109050000, and stays there when
//! the time is up, while one who stayed on the first level goes out to
//! 109050001. Pietro there gives the winner the Scroll of Secrets.
//! Variant 1 (`/ola11`): of each level's `rand_ola` portals one leads on
//! and the rest back to the level's start. The harness runs events 60
//! times as fast (`RUSTMS_EVENT_SCALE`), so their 360 s are 6.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::packets::{build_all_chat, build_change_map};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const PAUL: i32 = 9000000;
const PIETRO: i32 = 9000002;
const SCROLL_OF_SECRETS: i32 = 4031019;
const LEVEL_1: i32 = 109_030_101;
const LEVEL_2: i32 = 109_030_102;
const LEVEL_3: i32 = 109_030_103;
const WON: i32 = 109_050_000;
const EXIT: i32 = 109_050_001;
/// Where the gate leads: the first level's start (`start00`).
const START: u8 = 13;
const NOTICE: u8 = 0;
const CLOSED: (i16, u8) = (SendOpcode::BlockedMap as i16, 1);

/// The tests share the channel's one event, so they take turns.
static EVENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Going through a scripted portal.
fn touch(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Talking to `npc` on `map`.
fn talk_to(map: i32, npc: i32) -> Packet {
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let index = maps
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// Through the gate, from where players wait.
async fn gate(c: &mut MapleTestConnection, m: &str) -> Trace {
    let mut t = Trace::default();
    send(c, build_change_map(-1, "join00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty() || !t.warps.is_empty()).await;
    barrier(c, m, &mut t).await;
    t
}

#[tokio::test]
async fn the_gate_opens_at_the_start_and_the_right_portals_lead_out_of_the_third_level() {
    let _turn = EVENT.lock().await;
    let paul = talk_to(SOUTHPERRY, PAUL);
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
    send(g, build_all_chat("/ola21", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;

    // Paul takes both to the first level, where they wait.
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
        assert_eq!(t.warps, [LEVEL_1], "in");
        assert!(t.countdowns.is_empty(), "no clock yet");
        players.push((session, config.character_name.clone()));
    }
    let [(runner, rm), (stayer, _)] = &mut players[..] else {
        unreachable!()
    };

    let t = gate(&mut runner.connection, rm).await;
    assert_eq!(t.blocked, [CLOSED], "closed");
    assert!(t.warps.is_empty());

    // The GM starts it from the second level; it starts on all three.
    assert_eq!(warp(g, LEVEL_2, "", &mut gt).await, LEVEL_2);
    barrier(g, "on level 2", &mut gt).await;
    send(g, build_all_chat("/start", 0).unwrap()).await;
    let mut st = Trace::default();
    until(&mut stayer.connection, &mut st, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    assert_eq!(st.countdowns, [6], "the clock");

    // Through the gate to the start, then the right portal on each level.
    let mut rt = Trace::default();
    until(&mut runner.connection, &mut rt, |t| {
        !t.countdowns.is_empty()
    })
    .await;
    let t = gate(&mut runner.connection, rm).await;
    assert!(t.blocked.is_empty(), "open");
    assert_eq!(t.warps, [LEVEL_1]);
    assert_eq!(t.arrivals[0].1, START);
    let mut rt = Trace::default();
    let c = &mut runner.connection;
    assert_eq!(warp(c, -1, "ch00", &mut rt).await, LEVEL_2);
    assert_eq!(warp(c, -1, "ch01", &mut rt).await, LEVEL_3);
    assert_eq!(warp(c, -1, "ch03", &mut rt).await, WON);

    // The time is up: the stayer goes out with every notice heard, and the
    // winner stays where they are.
    until(&mut stayer.connection, &mut st, |t| !t.warps.is_empty()).await;
    assert_eq!(st.warps, [EXIT], "out of level 1");
    let notices = st.broadcasts.iter().filter(|(k, _)| *k == NOTICE).count();
    assert_eq!(notices, 9, "every notice");
    barrier(&mut runner.connection, rm, &mut rt).await;
    assert_eq!(rt.warps, [LEVEL_2, LEVEL_3, WON], "the winner stays");

    // Pietro gives the winner the prize and sends them back.
    let c = &mut runner.connection;
    let t = step(c, rm, talk_to(WON, PIETRO)).await;
    assert_eq!(t.prompts, [(PIETRO, 0)]);
    let t = step(c, rm, answer(0, 1, None)).await;
    assert_eq!(t.items, [(SCROLL_OF_SECRETS, 1)], "the prize");
    assert_eq!(t.warps, [SOUTHPERRY], "back");

    // The GM stays, and the gate is closed again.
    barrier(g, "over", &mut gt).await;
    assert_eq!(gt.warps, [LEVEL_2]);
    assert_eq!(warp(g, LEVEL_1, "", &mut gt).await, LEVEL_1);
    let t = gate(g, "closed again").await;
    assert_eq!(t.blocked, [CLOSED]);
    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}

#[tokio::test]
async fn one_of_each_rand_ola_level_s_portals_leads_on_and_the_rest_back() {
    let _turn = EVENT.lock().await;
    const LEVELS: [(i32, u8, usize); 3] = [
        (109_030_001, 13, 5),
        (109_030_002, 1, 8),
        (109_030_003, 1, 16),
    ];
    let paul = talk_to(SOUTHPERRY, PAUL);
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let player = load_harness_config_or_fail().await;
    login_to_world_session(&player).await.expect("player login");
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(g, build_all_chat("/ola11", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;

    let mut session = relogin(&player).await;
    let (c, m) = (&mut session.connection, player.character_name.as_str());
    let mut t = Trace::default();
    assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
    barrier(c, m, &mut t).await;
    step(c, m, paul).await;
    let t = step(c, m, answer(4, 1, Some(1))).await;
    assert_eq!(t.warps, [LEVELS[0].0], "in");

    assert_eq!(warp(g, LEVELS[0].0, "", &mut gt).await, LEVELS[0].0);
    barrier(g, "on level 1", &mut gt).await;
    send(g, build_all_chat("/start", 0).unwrap()).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.countdowns.is_empty()).await;
    let t = gate(c, m).await;
    assert_eq!(t.warps, [LEVELS[0].0], "through the gate");

    // Try each level's portals in turn until one leads on.
    for (level, &(map_id, start, ways)) in LEVELS.iter().enumerate() {
        let next = LEVELS
            .get(level + 1)
            .map_or((WON, 2), |&(map, start, _)| (map, start));
        let mut on = None;
        for way in 0..ways {
            let t = step(c, m, touch(&format!("ch{way:02}"))).await;
            assert_eq!(t.arrivals.len(), 1, "level {level} ch{way:02}");
            let arrival = (t.arrivals[0].0, t.arrivals[0].1);
            if arrival == next {
                on = Some(way);
                break;
            }
            assert_eq!(arrival, (map_id, start), "back to the start");
        }
        assert!(on.is_some(), "a way on from level {level}");
    }

    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}
