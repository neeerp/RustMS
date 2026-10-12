//! Rain's Maple Quiz (1009–1015) through the v83 quest flow. The client asks
//! the quiz from `Say.img` and sends `QUEST_ACTION`; the server validates the
//! start and completion and pays each reward once.

use integration_harness::packets::{
    build_change_channel, build_player_logged_in, decode_channel_change, decode_set_field,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    act, barrier, read, send, warp, QuestTrace, COMPLETE, FORFEIT, START,
};
use integration_harness::{connection::MapleTestConnection, protocol::WorldSession};
use net::packet::op::SendOpcode;

const AMHERST: i32 = 1_000_000;
const RAIN: i32 = 12101;
const PIO: i32 = 10000;

async fn in_amherst(
    config: &integration_harness::config::HarnessConfig,
    trace: &mut QuestTrace,
) -> WorldSession {
    let mut session = integration_harness::login_to_world_session(config)
        .await
        .unwrap();
    if session.map_id != AMHERST {
        assert_eq!(
            warp(&mut session.connection, AMHERST, "", trace).await,
            AMHERST
        );
        session.map_id = AMHERST;
    }
    session
}

#[tokio::test]
async fn seven_quizzes_award_once_and_survive_map_changes_and_relogin() {
    let config = load_harness_config_or_fail().await;
    let mut trace = QuestTrace::default();
    let mut session = in_amherst(&config, &mut trace).await;
    assert_eq!((session.level, session.exp, session.ap), (1, 0, 0));
    let c = &mut session.connection;
    let marker = config.character_name.clone();
    for id in 1009..=1015 {
        act(c, &mut trace, &marker, START, id, RAIN).await;
        assert_eq!(trace.states.get(&id), Some(&1));
        assert_eq!(trace.results.last(), Some(&(id, RAIN, 0)));
        act(c, &mut trace, &marker, COMPLETE, id, RAIN).await;
        assert_eq!(trace.states.get(&id), Some(&2));
        // Act/1/nextQuest continues into the next quiz.
        let next = if id < 1015 { id + 1 } else { 0 };
        assert_eq!(trace.results.last(), Some(&(id, RAIN, next)));
        // A replayed completion must not pay again.
        act(c, &mut trace, &marker, COMPLETE, id, RAIN).await;
        assert_eq!(trace.gains.len(), (id - 1008) as usize);
    }
    assert_eq!(trace.gains, [2, 4, 6, 10, 10, 10, 20]);
    assert_eq!(trace.stats, Some((3, 10, 13)));
    assert_eq!(warp(c, 10_000, "", &mut trace).await, 10_000);
    assert_eq!(warp(c, AMHERST, "", &mut trace).await, AMHERST);
    drop(session);
    let restored = integration_harness::login_to_world_session(&config)
        .await
        .unwrap();
    assert_eq!((restored.level, restored.ap, restored.exp), (3, 10, 13));
    assert_eq!(restored.quests.len(), 7);
    assert!(restored.quests.values().all(|state| *state == 2));
}

#[tokio::test]
async fn invalid_actions_are_refused_and_progress_is_per_character() {
    let config = load_harness_config_or_fail().await;
    let other = load_harness_config_or_fail().await;
    let mut trace = QuestTrace::default();
    let mut session = in_amherst(&config, &mut trace).await;
    let c = &mut session.connection;
    let m = config.character_name.clone();

    // Prerequisite: quiz 2 needs quiz 1 completed.
    act(c, &mut trace, &m, START, 1010, RAIN).await;
    // Completing a quest that was never started.
    act(c, &mut trace, &m, COMPLETE, 1009, RAIN).await;
    // Pio is in Amherst but does not offer Rain's quiz.
    act(c, &mut trace, &m, START, 1009, PIO).await;
    // Item quests are not supported yet, even at their own NPC.
    act(c, &mut trace, &m, START, 1034, 2102).await;
    assert!(trace.progress.is_empty() && trace.results.is_empty());

    act(c, &mut trace, &m, START, 1009, RAIN).await;
    assert_eq!(trace.progress, [(1009, 1)]);
    // Starting twice does nothing.
    act(c, &mut trace, &m, START, 1009, RAIN).await;
    assert_eq!(trace.progress, [(1009, 1)]);

    // Rain is not on Mushroom Town, so the field refuses the NPC.
    assert_eq!(warp(c, 10_000, "", &mut trace).await, 10_000);
    act(c, &mut trace, &m, COMPLETE, 1009, RAIN).await;
    assert!(trace.gains.is_empty());

    // A started quest can be forfeited and taken again.
    act(c, &mut trace, &m, FORFEIT, 1009, 0).await;
    assert_eq!(trace.states.get(&1009), Some(&0));
    drop(session);
    let mut session = in_amherst(&config, &mut trace).await;
    assert!(session.quests.is_empty());
    let c = &mut session.connection;
    act(c, &mut trace, &m, START, 1009, RAIN).await;
    drop(session);

    let restored = integration_harness::login_to_world_session(&config)
        .await
        .unwrap();
    assert_eq!(restored.quests.get(&1009), Some(&1));
    assert!(trace.gains.is_empty());
    let isolated = integration_harness::login_to_world_session(&other)
        .await
        .unwrap();
    assert!(isolated.quests.is_empty());
    assert_eq!(isolated.exp, 0);
}

#[tokio::test]
async fn started_quiz_survives_channel_migration() {
    let config = load_harness_config_or_fail().await;
    let mut trace = QuestTrace::default();
    let mut session = in_amherst(&config, &mut trace).await;
    let m = config.character_name.clone();
    act(&mut session.connection, &mut trace, &m, START, 1009, RAIN).await;
    send(&mut session.connection, build_change_channel(1).unwrap()).await;
    let redirect = loop {
        let packet = read(&mut session.connection, &mut trace).await;
        if packet.opcode() == SendOpcode::ChangeChannel as i16 {
            break decode_channel_change(&packet).unwrap();
        }
    };
    assert_eq!(redirect.port, config.channel_addr(1).port());
    let mut connection =
        MapleTestConnection::connect(config.channel_addr(1), "quest channel migration")
            .await
            .unwrap();
    send(
        &mut connection,
        build_player_logged_in(session.character_id).unwrap(),
    )
    .await;
    let snapshot = loop {
        let packet = read(&mut connection, &mut trace).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break decode_set_field(&packet).unwrap();
        }
    };
    assert_eq!(snapshot.quests.get(&1009), Some(&1));
    assert_eq!(snapshot.exp, 0);
    barrier(&mut connection, &m, &mut trace).await;
    act(&mut connection, &mut trace, &m, COMPLETE, 1009, RAIN).await;
    assert_eq!(trace.gains, [2]);
}
