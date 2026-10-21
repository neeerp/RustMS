//! Repeatable quests (`Check/0/interval`): Huckle's ingredients (3038) can
//! be taken again a day after they were last handed in.

use integration_harness::database::{backdate_quest, set_job, set_quest};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::relogin;
use integration_harness::trace::{warp, Trace};

const HUCKLE: i32 = 2030012;
const HIDDEN_ROOM: i32 = 200081201;
const INGREDIENTS: i16 = 3038;
const DAY: i32 = 24 * 60;

#[tokio::test]
async fn huckles_errand_reopens_after_a_day() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // 3038 needs level 20; it was handed in just now.
    set_job(id, 0, 20, 50).await.expect("level up");
    set_quest(id, INGREDIENTS, 2, "")
        .await
        .expect("complete 3038");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, HIDDEN_ROOM, "", &mut trace).await, HIDDEN_ROOM);
    act(c, &mut trace, &m, START, INGREDIENTS, HUCKLE).await;
    assert_eq!(trace.states.get(&INGREDIENTS), None);
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    backdate_quest(id, INGREDIENTS, DAY)
        .await
        .expect("backdate 3038");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, HIDDEN_ROOM, "", &mut trace).await, HIDDEN_ROOM);
    act(c, &mut trace, &m, START, INGREDIENTS, HUCKLE).await;
    assert_eq!(trace.states.get(&INGREDIENTS), Some(&1));
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&INGREDIENTS), Some(&1));
}
