//! Items whose time runs out are taken, and the player's client says which
//! (SHOW_STATUS_INFO 8): while they play, or at their next login. A quest
//! gift with a period (`Act/item/period`, minutes) comes dated.

use integration_harness::database::{date_item, set_quest, wear_equip};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

/// Rupi in the Orbis Tower; her "Lost Paper" (3085) lends 4031281 for an
/// hour.
const RUPI: i32 = 2060006;
const NPC_X: i16 = 148;
const NPC_Y: i16 = -874;
const AQUARIUM_ROAD: i32 = 230000003;
const QUEST: i16 = 3085;
const PREREQUISITE: i16 = 3084;
const PAPER: i32 = 4031281;
const ETC: u8 = 4;
const ZENUMIST_CAPE: i32 = 1102135;
const CAPE: i16 = -9;

#[tokio::test]
async fn a_quest_gift_lasts_its_period_then_goes() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, PREREQUISITE, 2, "")
        .await
        .expect("complete 3084");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, AQUARIUM_ROAD, "", &mut trace).await, AQUARIUM_ROAD);
    send(c, walk_to(NPC_X, NPC_Y)).await;
    act(c, &mut trace, &m, START, QUEST, RUPI).await;
    assert_eq!(trace.states.get(&QUEST), Some(&1));
    let &(_, slot, _) = trace
        .added
        .iter()
        .find(|&&(kind, _, item)| (kind, item) == (ETC, PAPER))
        .expect("the paper is given");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // An hour; then a few seconds, which run out while the player plays.
    let left = date_item(id, PAPER, 5).await.expect("date the paper");
    assert_eq!(left, Some(59));
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.expired.is_empty()).await;
    assert_eq!(t.expired, [PAPER]);
    until(c, &mut t, |t| !t.removed.is_empty()).await;
    assert_eq!(t.removed, [(ETC, slot)]);
}

#[tokio::test]
async fn a_worn_item_that_expired_meanwhile_is_gone_at_login() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(id, CAPE, ZENUMIST_CAPE)
        .await
        .expect("wear the cape");
    date_item(id, ZENUMIST_CAPE, -60)
        .await
        .expect("date the cape");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.expired.is_empty()).await;
    assert_eq!(t.expired, [ZENUMIST_CAPE]);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Gone for good: nothing is left to date.
    assert!(date_item(id, ZENUMIST_CAPE, 60).await.is_err());
}
