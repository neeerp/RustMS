//! Snowman's Rage: the clue Scadur wants (3108) is the broken Spirit of
//! Snow Statue in the Valley of Snowmen, whose start script completes the
//! quest at once for 1,000 EXP. With it found, Scadur can end Snowman's
//! Rage (3107), which asks for it.

use integration_harness::database::{set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const SNOW_STATUE: i32 = 2_020_012;
const VALLEY_OF_SNOWMEN: i32 = 211_040_102;
const SCADUR: i32 = 2_020_007;
const EL_NATH: i32 = 211_000_000;

#[tokio::test]
async fn the_broken_statue_is_the_clue_scadur_wants() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 50, 100).await.expect("a level 50 fighter");
    set_quest(id, 3107, 1, "")
        .await
        .expect("Snowman's Rage taken");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, VALLEY_OF_SNOWMEN, "", &mut trace).await,
        VALLEY_OF_SNOWMEN
    );
    step(c, &m, walk_to(297, -123)).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3108, SNOW_STATUE)).await;
    assert_eq!(t.prompts, [(SNOW_STATUE, 0)], "the statue is looked at");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(3108, 2)], "found at once");
    assert_eq!(t.gains, [1_000]);

    // Scadur now ends Snowman's Rage.
    let mut trace = Trace::default();
    assert_eq!(warp(c, EL_NATH, "", &mut trace).await, EL_NATH);
    step(c, &m, walk_to(-2598, 94)).await;
    let t = step(c, &m, build_quest_completion(3107, SCADUR, -1)).await;
    assert_eq!(t.progress, [(3107, 2)]);
}
