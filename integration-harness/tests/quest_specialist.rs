//! Title Challenge - Quest Specialist (29001): Dalair gives the medal only
//! once 800 quests are completed (`Check/1/questComplete`).

use integration_harness::database::{complete_quests, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const QUEST_SPECIALIST: i16 = 29001;
const QUEST_SPECIALIST_MEDAL: i32 = 1142002;

/// Log in, then go to Dalair in Perion.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let m = &config.character_name;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    let npcs = &maps().field(PERION).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == DALAIR).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut trace).await;
    session
}

#[tokio::test]
async fn dalair_wants_eight_hundred_quests_completed() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    complete_quests(id, 1, 799).await.unwrap();

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(START, QUEST_SPECIALIST, DALAIR)).await;
    assert_eq!(t.progress, [(QUEST_SPECIALIST, 1)]);
    // 799 aren't enough.
    let t = step(c, &m, build_quest_completion(QUEST_SPECIALIST, DALAIR, -1)).await;
    assert!(t.progress.is_empty() && t.items.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    complete_quests(id, 800, 800).await.unwrap();
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_completion(QUEST_SPECIALIST, DALAIR, -1)).await;
    assert_eq!(t.progress, [(QUEST_SPECIALIST, 2)]);
    assert_eq!(t.items, [(QUEST_SPECIALIST_MEDAL, 1)]);
}
