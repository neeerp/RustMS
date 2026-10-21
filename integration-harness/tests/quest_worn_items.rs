//! Start requirements on what the player wears (`Check/0/equipAllNeed`):
//! Bedin's Helping Out Bedin (3307) starts only for someone wearing
//! Zenumist's cape, and v83 doesn't count a copy in the bag.

use integration_harness::database::{grant_item, set_job, set_quest, wear_equip};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};

const BEDIN: i32 = 2111008;
const NPC_X: i16 = 562;
const NPC_Y: i16 = 62;
const ZENUMIST: i32 = 261010000;
const QUEST: i16 = 3307;
const PREREQUISITE: i16 = 3302;
const ZENUMIST_CAPE: i32 = 1102135;
const EQUIP: i16 = 1;
const CAPE: i16 = -9;

#[tokio::test]
async fn the_quest_starts_only_with_the_cape_worn() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, 0, 35, 50).await.expect("level up");
    set_quest(id, PREREQUISITE, 2, "")
        .await
        .expect("complete 3302");
    grant_item(id, EQUIP, 1, ZENUMIST_CAPE)
        .await
        .expect("hold the cape");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, ZENUMIST, "", &mut trace).await, ZENUMIST);
    send(c, walk_to(NPC_X, NPC_Y)).await;
    act(c, &mut trace, &m, START, QUEST, BEDIN).await;
    assert_eq!(trace.states.get(&QUEST), None);
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    wear_equip(id, CAPE, ZENUMIST_CAPE)
        .await
        .expect("wear the cape");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, ZENUMIST, "", &mut trace).await, ZENUMIST);
    send(c, walk_to(NPC_X, NPC_Y)).await;
    act(c, &mut trace, &m, START, QUEST, BEDIN).await;
    assert_eq!(trace.states.get(&QUEST), Some(&1));
}
