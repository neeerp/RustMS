//! Reef's Gratitude (2228): once Faust is beaten (2227), Reef's start script
//! thanks the player, which completes the quest at once, with the data's 8
//! fame.

use integration_harness::database::{set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const QUEST: i16 = 2228;
const REEF: i32 = 1_032_108;
const REEFS_MAP: i32 = 910_100_001;

#[tokio::test]
async fn reef_thanks_the_player_with_eight_fame() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 200, 45, 100)
        .await
        .expect("a level 45 magician");
    set_quest(id, 2227, 2, "").await.expect("Faust beaten");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, REEFS_MAP, "", &mut trace).await, REEFS_MAP);
    step(c, &m, walk_to(33, 209)).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, QUEST, REEF)).await;
    assert_eq!(t.prompts, [(REEF, 0)], "Reef speaks");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(QUEST, 2)], "done at once");
    assert_eq!(t.fame_gains, [8]);
    // Once.
    let t = step(c, &m, build_quest_action(SCRIPT_START, QUEST, REEF)).await;
    assert!(t.prompts.is_empty() && t.progress.is_empty());
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&QUEST), Some(&2));
}
