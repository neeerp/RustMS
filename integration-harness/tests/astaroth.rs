//! Astaroth comes out in his Hiding Place to one hunting him for the
//! Doorway's quest (28283).

use integration_harness::database::start_quest;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{ASTAROTH_HIDING_PLACE, ASTAROTH_QUEST};
use tokio::time::Duration;

const ASTAROTH: i32 = 9400633;

#[tokio::test]
async fn astaroth_comes_out_to_his_hunter() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, ASTAROTH_QUEST).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, ASTAROTH_HIDING_PLACE, "", &mut seen).await,
        ASTAROTH_HIDING_PLACE
    );
    until(c, &mut seen, |t| !t.field.alive(ASTAROTH).is_empty()).await;
}
