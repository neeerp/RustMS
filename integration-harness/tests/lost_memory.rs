//! In Search for the Lost Memory: the Memory Keeper (3507) sends a player
//! back to the instructor who gave them their first job. That instructor's
//! start script (3523 for a warrior's Dances with Balrog) completes at once
//! and gives the memory back (record 7081), so the Memory Keeper can end
//! 3507.

use integration_harness::database::{set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const DANCES_WITH_BALROG: i32 = 1_022_000;
const WARRIORS_SANCTUARY: i32 = 102_000_003;
const MEMORY_KEEPER: i32 = 2_140_001;
const MEMORY_LANE: i32 = 270_010_111;

#[tokio::test]
async fn the_first_instructor_gives_the_memory_back() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 111, 105, 100)
        .await
        .expect("a level 105 crusader");
    set_quest(id, 3506, 2, "").await.expect("3506 done");
    set_quest(id, 3507, 1, "")
        .await
        .expect("Memory Keeper taken");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, WARRIORS_SANCTUARY, "", &mut trace).await,
        WARRIORS_SANCTUARY
    );
    step(c, &m, walk_to(-295, 20)).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, 3523, DANCES_WITH_BALROG),
    )
    .await;
    assert_eq!(t.prompts, [(DANCES_WITH_BALROG, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(
        t.progress,
        [(3523, 2), (7081, 1)],
        "found, and the memory back"
    );

    // The Memory Keeper now ends 3507.
    let mut trace = Trace::default();
    assert_eq!(warp(c, MEMORY_LANE, "", &mut trace).await, MEMORY_LANE);
    step(c, &m, walk_to(-521, -1103)).await;
    let t = step(c, &m, build_quest_completion(3507, MEMORY_KEEPER, -1)).await;
    assert_eq!(t.progress, [(3507, 2)]);
    assert_eq!(t.gains, [359_900]);
}
