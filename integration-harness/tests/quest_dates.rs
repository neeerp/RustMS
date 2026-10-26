//! Quest date windows (`Check/0/start` to `end`): v83 offers a quest only
//! between them, so Cassandra's 2008 event quest (9952) is over, and the
//! Maple Administrator's 9625, open for the first hour of 31 December 2030,
//! hasn't begun. Both are supported (`supported_quests.txt`), so the server
//! refuses them for their dates alone.

use integration_harness::database::set_job;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::relogin;
use integration_harness::trace::{warp, Trace};

const HENESYS: i32 = 100000000;
const CASSANDRA: i32 = 9010010;
const MAPLE_ADMINISTRATOR: i32 = 9010000;
const OVER: i16 = 9952;
const NOT_YET: i16 = 9625;

#[tokio::test]
async fn quests_outside_their_date_window_dont_start() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // 9625 needs level 200, 9952 level 10.
    set_job(id, 0, 200, 50).await.expect("level up");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, HENESYS, "", &mut trace).await, HENESYS);
    act(c, &mut trace, &m, START, OVER, CASSANDRA).await;
    assert_eq!(trace.states.get(&OVER), None);
    act(c, &mut trace, &m, START, NOT_YET, MAPLE_ADMINISTRATOR).await;
    assert_eq!(trace.states.get(&NOT_YET), None);
}
