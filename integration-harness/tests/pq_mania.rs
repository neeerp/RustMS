//! Title Challenge - PQ Mania (29000): Dalair gives the medal only once
//! five party quests' records rank S (`Check/1/partyQuest_S`).

use integration_harness::database::{set_job, set_quest_ex};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const PQ_MANIA: i16 = 29000;
const PQ_MANIA_MEDAL: i32 = 1142001;

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
async fn dalair_wants_five_party_quests_at_rank_s() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    for quest in 1200..=1203 {
        set_quest_ex(id, quest, "try=120;cmp=100;rank=S")
            .await
            .unwrap();
    }
    // Neither a lower-case s nor a record outside the party quests counts.
    set_quest_ex(id, 1204, "try=120;cmp=100;rank=s")
        .await
        .unwrap();
    set_quest_ex(id, 1400, "rank=S").await.unwrap();

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(START, PQ_MANIA, DALAIR)).await;
    assert_eq!(t.progress, [(PQ_MANIA, 1)]);
    // Four aren't enough.
    let t = step(c, &m, build_quest_completion(PQ_MANIA, DALAIR, -1)).await;
    assert!(t.progress.is_empty() && t.items.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_quest_ex(id, 1301, "try=3;vic=3;rank=S").await.unwrap();
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_completion(PQ_MANIA, DALAIR, -1)).await;
    assert_eq!(t.progress, [(PQ_MANIA, 2)]);
    assert_eq!(t.items, [(PQ_MANIA_MEDAL, 1)]);
}
