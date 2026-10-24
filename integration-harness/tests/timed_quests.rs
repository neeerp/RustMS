//! Timed quests (`QuestInfo` `timeLimit`): starting one shows its clock
//! first, giving it up takes the clock away, and when the time is up the
//! quest is forfeited and the chat log says why. The clock goes with the
//! player to another channel; leaving the game gives the quest up. The
//! harness shortens the 30 minutes of Jr. Officer Medin's plan to 30 seconds
//! (`RUSTMS_EVENT_SCALE`).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_job;
use integration_harness::hunting::walk_to;
use integration_harness::packets::{
    build_change_channel, build_player_logged_in, decode_channel_change, decode_set_field,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, FORFEIT, START};
use integration_harness::trace::{read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::{timeout, Duration, Instant};

const OMEGA_SECTOR: i32 = 221_000_000;
const MEDIN: i32 = 2_050_009;
const MEDIN_X: i16 = 5689;
const MEDIN_Y: i16 = -682;
/// Revolutionary Defensive Wall Construction Plan: 1800 seconds.
const WALL_PLAN: i16 = 3458;
const SHOWN_MS: i32 = 1_800_000 / 60;

#[tokio::test]
async fn a_timed_quest_runs_out_on_its_clock() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 40, 50).await.expect("level 40");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, OMEGA_SECTOR, "", &mut trace).await, OMEGA_SECTOR);
    send(c, walk_to(MEDIN_X, MEDIN_Y)).await;

    // The clock comes before the quest log hears of the start.
    act(c, &mut trace, &m, START, WALL_PLAN, MEDIN).await;
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&1));
    let started_at = trace.progress.len() - 1;
    assert_eq!(
        trace.quest_clocks,
        [(6, WALL_PLAN, SHOWN_MS, started_at)],
        "{:?}",
        trace.progress
    );

    // Given up, the clock goes first.
    trace.quest_clocks.clear();
    act(c, &mut trace, &m, FORFEIT, WALL_PLAN, 0).await;
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&0));
    assert_eq!(
        trace.quest_clocks,
        [(7, WALL_PLAN, 0, trace.progress.len() - 1)]
    );

    // Run out: the clock goes, then the quest, then the line in the chat log.
    trace.quest_clocks.clear();
    act(c, &mut trace, &m, START, WALL_PLAN, MEDIN).await;
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&1));
    trace.quest_clocks.clear();
    run_out(c, &mut trace).await;
    let gone = trace.progress.len();
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&0));
    assert_eq!(
        trace.quest_clocks,
        [(7, WALL_PLAN, 0, gone - 1), (15, WALL_PLAN, 0, gone)]
    );

    // Started again, it goes on through a channel change and runs out there.
    act(c, &mut trace, &m, START, WALL_PLAN, MEDIN).await;
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&1));
    let mut c = session.connection;
    send(&mut c, build_change_channel(1).unwrap()).await;
    let redirect = loop {
        let packet = read(&mut c, &mut trace).await;
        if packet.opcode() == SendOpcode::ChangeChannel as i16 {
            break decode_channel_change(&packet).unwrap();
        }
    };
    let mut c = MapleTestConnection::connect(
        format!("{}:{}", redirect.ip, redirect.port)
            .parse()
            .unwrap(),
        "change-channel reconnect",
    )
    .await
    .unwrap();
    send(&mut c, build_player_logged_in(id).unwrap()).await;
    let set_field = loop {
        let packet = read(&mut c, &mut trace).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break decode_set_field(&packet).unwrap();
        }
    };
    assert_eq!(set_field.quests.get(&WALL_PLAN), Some(&1));
    // The client still shows the clock; it isn't sent again.
    trace.quest_clocks.clear();
    run_out(&mut c, &mut trace).await;
    let gone = trace.progress.len();
    assert_eq!(
        trace.quest_clocks,
        [(7, WALL_PLAN, 0, gone - 1), (15, WALL_PLAN, 0, gone)]
    );
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&0));

    // Started again, leaving the game gives it up.
    let m = config.character_name.clone();
    send(&mut c, walk_to(MEDIN_X, MEDIN_Y)).await;
    act(&mut c, &mut trace, &m, START, WALL_PLAN, MEDIN).await;
    assert_eq!(trace.states.get(&WALL_PLAN), Some(&1));
    drop(c);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let session = relogin(&config).await;
    assert_eq!(session.quests.get(&WALL_PLAN), None);
}

/// Read until the chat line of a quest run out, within the 30 seconds.
async fn run_out(c: &mut MapleTestConnection, trace: &mut Trace) {
    let deadline = Instant::now() + Duration::from_secs(45);
    while !trace.quest_clocks.iter().any(|clock| clock.0 == 15) {
        let envelope = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("trace"),
        )
        .await
        .unwrap_or_else(|_| panic!("the quest never ran out: {trace:?}"))
        .unwrap();
        trace.observe(&envelope.packet);
    }
}
